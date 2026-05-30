use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{Read, Take},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    RootId, TimestampMillis,
    catalog::{RootError, RootRegistration, RootSnapshot, RootStatus},
    core::{
        ScriptMetaEditState, ScriptMetaItem, ScriptRuntimeKind, VersionOrdering, compare_versions,
        parse_script_metadata,
    },
    now_timestamp_millis,
    scanner::{ExtensionPolicy, ScannerOptions},
    watcher::normalize_path,
};

use super::path_resolution::{
    PathKind, PathResolutionStatus, path_error_status, resolve_scannable_path,
};
use super::{
    file_list::system_time_millis,
    script_detection::{
        detect_script_file, has_scriptmeta_tag, scriptmeta_edit_capability_from_cached_metadata,
        scriptmeta_edit_capability_from_metadata,
    },
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CandidateCache {
    pub schema_version: u32,
    pub built_at: TimestampMillis,
    pub registered_roots: Vec<RegisteredRootSignature>,
    pub records: Vec<CandidateRecord>,
}

impl CandidateCache {
    pub const CURRENT_SCHEMA_VERSION: u32 = 4;

    #[must_use]
    pub fn empty() -> Self {
        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            built_at: now_timestamp_millis(),
            registered_roots: Vec::new(),
            records: Vec::new(),
        }
    }

    #[must_use]
    pub fn is_current_schema(&self) -> bool {
        self.schema_version == Self::CURRENT_SCHEMA_VERSION
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RegisteredRootSignature {
    pub root_id: RootId,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CandidateRecord {
    pub root_id: RootId,
    pub root_path: PathBuf,
    pub file_path: PathBuf,
    pub identity_path: PathBuf,
    #[serde(default)]
    pub path_kind: PathKind,
    #[serde(default)]
    pub resolution_status: PathResolutionStatus,
    #[serde(default)]
    pub resolution_message: Option<String>,
    #[serde(default)]
    pub runtime_kind: Option<ScriptRuntimeKind>,
    #[serde(default)]
    pub shebang: Option<String>,
    #[serde(default)]
    pub has_scriptmeta: bool,
    #[serde(default)]
    pub has_scriptmeta_edit_password: bool,
    #[serde(default)]
    pub is_file_locked: bool,
    #[serde(default)]
    pub is_read_only: bool,
    #[serde(default)]
    pub can_edit_scriptmeta: bool,
    #[serde(default)]
    pub can_append_scriptmeta: bool,
    #[serde(default)]
    pub scriptmeta_edit_state: ScriptMetaEditState,
    pub file_size: Option<u64>,
    pub content_modified_at: Option<TimestampMillis>,
    pub item: Option<ScriptMetaItem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetadataScanOutput {
    pub roots: Vec<RootSnapshot>,
    pub all_items: Vec<ScriptMetaItem>,
    pub file_items: Vec<ScriptMetaItem>,
    pub candidate_cache: CandidateCache,
    pub source_revision: Uuid,
}

pub fn scan_metadata_roots<'a, I>(
    roots: I,
    options: &ScannerOptions,
    extensions: &ExtensionPolicy,
    previous_cache: Option<&CandidateCache>,
) -> MetadataScanOutput
where
    I: IntoIterator<Item = &'a RootRegistration>,
{
    let roots: Vec<_> = roots.into_iter().collect();
    let reusable_records = reusable_records_by_identity_path(previous_cache, options);
    let mut records = Vec::new();
    let mut root_snapshots = Vec::new();

    for root in &roots {
        let root = *root;
        let started = Instant::now();
        let timeout = options
            .scan_timeout_per_root_millis
            .map(Duration::from_millis);
        let mut snapshot = RootSnapshot::new(root.root_id.clone(), root.path.clone());

        if !root.path.exists() {
            snapshot.status = RootStatus::Missing;
            snapshot.error = Some(RootError {
                code: "missing".to_string(),
                message: "root path does not exist".to_string(),
            });
            root_snapshots.push(snapshot);
            continue;
        }

        let mut state = MetadataWalkState {
            root,
            options,
            extensions,
            reusable_records: &reusable_records,
            timeout,
            started,
            visited_directories: BTreeSet::new(),
            visited_nodes: 0,
            timed_out: false,
        };

        let root_resolution = resolve_scannable_path(root.path.clone(), root.path.clone(), options);
        let root_metadata = match fs::metadata(&root_resolution.resolved_path) {
            Ok(metadata) => metadata,
            Err(error) => {
                snapshot.status = RootStatus::Missing;
                snapshot.error = Some(RootError {
                    code: "unresolved_root".to_string(),
                    message: error.to_string(),
                });
                root_snapshots.push(snapshot);
                continue;
            }
        };
        if !root_metadata.is_dir() {
            snapshot.status = RootStatus::Missing;
            snapshot.error = Some(RootError {
                code: "not_directory".to_string(),
                message: "root path does not resolve to a directory".to_string(),
            });
            root_snapshots.push(snapshot);
            continue;
        }

        scan_directory(
            &root.path,
            &root_resolution.resolved_path,
            0,
            &mut state,
            &mut records,
        );
        snapshot.status = if state.timed_out {
            RootStatus::TimedOut
        } else {
            RootStatus::Ready
        };
        snapshot.is_dirty = false;
        snapshot.last_loaded_at = Some(now_timestamp_millis());
        snapshot.item_count = records
            .iter()
            .filter(|record| record.root_id == root.root_id && record.item.is_some())
            .count();
        root_snapshots.push(snapshot);
    }

    let candidate_cache = CandidateCache {
        schema_version: CandidateCache::CURRENT_SCHEMA_VERSION,
        built_at: now_timestamp_millis(),
        registered_roots: registered_root_signatures(&roots),
        records,
    };

    let file_items = file_items_from_cache(&candidate_cache);
    let all_items = deduplicated_items(&file_items);

    MetadataScanOutput {
        roots: root_snapshots,
        all_items,
        file_items,
        candidate_cache,
        source_revision: Uuid::new_v4(),
    }
}

struct MetadataWalkState<'a> {
    root: &'a RootRegistration,
    options: &'a ScannerOptions,
    extensions: &'a ExtensionPolicy,
    reusable_records: &'a BTreeMap<&'a Path, &'a CandidateRecord>,
    timeout: Option<Duration>,
    started: Instant,
    visited_directories: BTreeSet<PathBuf>,
    visited_nodes: usize,
    timed_out: bool,
}

fn scan_directory(
    display_directory: &Path,
    source_directory: &Path,
    depth: usize,
    state: &mut MetadataWalkState<'_>,
    records: &mut Vec<CandidateRecord>,
) {
    if should_stop(depth, state) {
        return;
    }

    let resolved_directory = normalize_path(source_directory);
    if !state.visited_directories.insert(resolved_directory) {
        return;
    }

    let Ok(entries) = fs::read_dir(source_directory) else {
        return;
    };

    for entry in entries.flatten() {
        if should_stop(depth, state) {
            return;
        }

        let source_path = entry.path();
        let display_path = display_directory.join(entry.file_name());
        if should_skip_path(&display_path, state.options) {
            continue;
        }

        let resolved = resolve_scannable_path(display_path, source_path, state.options);
        if should_skip_resolution_error(resolved.resolution_status) {
            continue;
        }
        if state.options.skip_packages && is_package_path(&resolved.resolved_path) {
            continue;
        }

        let metadata = match fs::metadata(&resolved.resolved_path) {
            Ok(metadata) => metadata,
            Err(error) => {
                if path_error_status(&error) == PathResolutionStatus::PermissionDenied {
                    state.visited_nodes += 1;
                }
                continue;
            }
        };

        if metadata.is_dir() {
            let resolved_directory = normalize_path(&resolved.resolved_path);
            if !state.visited_directories.contains(&resolved_directory) {
                scan_directory(
                    &resolved.display_path,
                    &resolved.resolved_path,
                    depth + 1,
                    state,
                    records,
                );
            }
            continue;
        }

        if !metadata.is_file() || !state.extensions.contains_path(&resolved.resolved_path) {
            continue;
        }

        state.visited_nodes += 1;
        records.push(candidate_record(
            &resolved.display_path,
            &resolved.resolved_path,
            resolved.path_kind,
            resolved.resolution_status,
            resolved.resolution_message.clone(),
            &metadata,
            state,
        ));
    }
}

fn should_skip_resolution_error(status: PathResolutionStatus) -> bool {
    status != PathResolutionStatus::NotRequested && status != PathResolutionStatus::Resolved
}

fn candidate_record(
    file_path: &Path,
    identity_path: &Path,
    path_kind: PathKind,
    resolution_status: PathResolutionStatus,
    resolution_message: Option<String>,
    metadata: &fs::Metadata,
    state: &MetadataWalkState<'_>,
) -> CandidateRecord {
    let file_size = Some(metadata.len());
    let content_modified_at = metadata.modified().ok().and_then(system_time_millis);

    if let Some(record) = state.reusable_records.get(identity_path).filter(|record| {
        state.options.reuse_unchanged_records
            && record.file_size == file_size
            && record.content_modified_at == content_modified_at
    }) {
        let reused_capability = scriptmeta_edit_capability_from_cached_metadata(
            identity_path,
            record.scriptmeta_edit_state,
            record.has_scriptmeta,
            record.has_scriptmeta_edit_password,
        );
        return CandidateRecord {
            root_id: state.root.root_id.clone(),
            root_path: state.root.path.clone(),
            file_path: file_path.to_path_buf(),
            identity_path: identity_path.to_path_buf(),
            path_kind,
            resolution_status,
            resolution_message,
            runtime_kind: record.runtime_kind,
            shebang: record.shebang.clone(),
            has_scriptmeta: record.has_scriptmeta,
            has_scriptmeta_edit_password: record.has_scriptmeta_edit_password,
            is_file_locked: reused_capability.is_file_locked,
            is_read_only: reused_capability.is_read_only,
            can_edit_scriptmeta: reused_capability.can_edit_scriptmeta,
            can_append_scriptmeta: reused_capability.can_append_scriptmeta,
            scriptmeta_edit_state: reused_capability.scriptmeta_edit_state,
            file_size,
            content_modified_at,
            item: record.item.as_ref().map(|item| {
                item.with_location(
                    state.root.root_id.clone(),
                    file_path.to_path_buf(),
                    identity_path.to_path_buf(),
                )
                .with_scriptmeta_edit_capability(reused_capability)
            }),
        };
    }

    let prefix_text = read_prefix(
        identity_path,
        state.options.max_prefix_bytes,
        metadata.len(),
    )
    .ok();
    let script_info = detect_script_file(identity_path, prefix_text.as_deref());
    let has_scriptmeta = prefix_text.as_deref().is_some_and(has_scriptmeta_tag);
    let parsed_metadata = prefix_text
        .as_deref()
        .and_then(|text| parse_script_metadata(text).ok());
    let has_scriptmeta_edit_password = parsed_metadata
        .as_ref()
        .is_some_and(|metadata| metadata.edit_password_sha256.is_some());
    let capability = scriptmeta_edit_capability_from_metadata(
        identity_path,
        prefix_text.as_deref().map(str::as_bytes),
        has_scriptmeta,
        has_scriptmeta_edit_password,
    );
    let item = prefix_text.and(parsed_metadata).map(|metadata| {
        ScriptMetaItem::from_metadata(
            state.root.root_id.clone(),
            file_path.to_path_buf(),
            identity_path.to_path_buf(),
            metadata,
        )
        .with_script_file_info(script_info.runtime_kind, script_info.shebang.clone())
        .with_scriptmeta_edit_capability(capability)
    });

    CandidateRecord {
        root_id: state.root.root_id.clone(),
        root_path: state.root.path.clone(),
        file_path: file_path.to_path_buf(),
        identity_path: identity_path.to_path_buf(),
        path_kind,
        resolution_status,
        resolution_message,
        runtime_kind: script_info.runtime_kind,
        shebang: script_info.shebang,
        has_scriptmeta,
        has_scriptmeta_edit_password,
        is_file_locked: capability.is_file_locked,
        is_read_only: capability.is_read_only,
        can_edit_scriptmeta: capability.can_edit_scriptmeta,
        can_append_scriptmeta: capability.can_append_scriptmeta,
        scriptmeta_edit_state: capability.scriptmeta_edit_state,
        file_size,
        content_modified_at,
        item,
    }
}

fn read_prefix(path: &Path, max_bytes: usize, file_size: u64) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut limited: Take<&mut File> = file.by_ref().take(max_bytes as u64);
    let capacity = usize::try_from(file_size)
        .unwrap_or(max_bytes)
        .min(max_bytes);
    let mut buffer = Vec::with_capacity(capacity);
    limited.read_to_end(&mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

fn should_stop(depth: usize, state: &mut MetadataWalkState<'_>) -> bool {
    if depth > state.options.max_depth || state.visited_nodes >= state.options.max_nodes_per_root {
        return true;
    }

    if let Some(timeout) = state.timeout
        && state.started.elapsed() >= timeout
    {
        state.timed_out = true;
        return true;
    }

    false
}

fn should_skip_path(path: &Path, options: &ScannerOptions) -> bool {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    if options.skip_hidden && name.starts_with('.') {
        return true;
    }

    options.skip_packages && is_package_path(path)
}

fn is_package_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("app" | "bundle" | "framework" | "plugin" | "appex")
    )
}

fn reusable_records_by_identity_path<'a>(
    previous_cache: Option<&'a CandidateCache>,
    options: &ScannerOptions,
) -> BTreeMap<&'a Path, &'a CandidateRecord> {
    if !options.reuse_unchanged_records {
        return BTreeMap::new();
    }

    previous_cache
        .filter(|cache| cache.is_current_schema())
        .map(|cache| {
            cache
                .records
                .iter()
                .map(|record| (record.identity_path.as_path(), record))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn registered_root_signatures(
    roots: &[&RootRegistration],
) -> Vec<RegisteredRootSignature> {
    roots
        .iter()
        .map(|root| RegisteredRootSignature {
            root_id: root.root_id.clone(),
            path: normalize_path(&root.path),
        })
        .collect()
}

pub(crate) fn file_items_from_cache(cache: &CandidateCache) -> Vec<ScriptMetaItem> {
    let mut items: Vec<_> = cache
        .records
        .iter()
        .filter_map(|record| record.item.clone())
        .collect();
    items.sort_by(|lhs, rhs| {
        lhs.root_id
            .cmp(&rhs.root_id)
            .then_with(|| lhs.file_path.cmp(&rhs.file_path))
            .then_with(|| lhs.identity_path.cmp(&rhs.identity_path))
    });
    items
}

pub(crate) fn deduplicated_items(items: &[ScriptMetaItem]) -> Vec<ScriptMetaItem> {
    let mut best_by_script_id: BTreeMap<&str, &ScriptMetaItem> = BTreeMap::new();
    for item in items {
        best_by_script_id
            .entry(item.script_id.as_str())
            .and_modify(|current| {
                if should_replace_item(current, item) {
                    *current = item;
                }
            })
            .or_insert(item);
    }

    let mut deduplicated: Vec<_> = best_by_script_id.into_values().cloned().collect();
    deduplicated.sort_by(|lhs, rhs| lhs.file_path.cmp(&rhs.file_path));
    deduplicated
}

fn should_replace_item(current: &ScriptMetaItem, candidate: &ScriptMetaItem) -> bool {
    match (&current.version, &candidate.version) {
        (None, Some(_)) => true,
        (Some(_), None) => false,
        (Some(current_version), Some(candidate_version)) => {
            match compare_versions(current_version, candidate_version) {
                VersionOrdering::Less => true,
                VersionOrdering::Equal => candidate.file_path < current.file_path,
                VersionOrdering::Greater => false,
            }
        }
        (None, None) => candidate.file_path < current.file_path,
    }
}
