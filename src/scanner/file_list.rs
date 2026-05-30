use std::{
    borrow::Cow,
    collections::BTreeSet,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

use serde::{Deserialize, Serialize};

use crate::{
    RootId,
    catalog::{DirectoryState, DirectoryStateMap, RootError, RootSnapshot, RootStatus},
    core::ScriptRuntimeKind,
    now_timestamp_millis,
    scanner::{ExtensionPolicy, ScannerOptions},
    watcher::normalize_path,
};

use super::{
    path_resolution::{
        PathKind, PathResolutionStatus, ResolvedPath, path_error_status, resolve_scannable_path,
    },
    script_detection::{
        detect_script_file, detect_script_file_from_bytes, needs_script_header_probe,
        script_header_probe_byte_limit, scriptmeta_edit_capability_from_file_list_probe,
    },
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FileSystemEntry {
    pub display_path: PathBuf,
    pub resolved_path: PathBuf,
    #[serde(default)]
    pub path_kind: PathKind,
    #[serde(default)]
    pub resolution_status: PathResolutionStatus,
    #[serde(default)]
    pub resolution_message: Option<String>,
    pub is_directory: bool,
    #[serde(default)]
    pub file_size: Option<u64>,
    #[serde(default)]
    pub content_modified_at: Option<u64>,
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
    pub scriptmeta_edit_state: crate::core::ScriptMetaEditState,
    pub children: Vec<FileSystemEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryScanOutput {
    pub root: RootSnapshot,
    pub children: Vec<FileSystemEntry>,
    pub directory_states: DirectoryStateMap,
    pub truncated: bool,
}

pub fn scan_file_list_root(
    root_id: &RootId,
    root_path: &Path,
    options: &ScannerOptions,
    extensions: &ExtensionPolicy,
) -> DirectoryScanOutput {
    let mut root = RootSnapshot::new(root_id.clone(), root_path.to_path_buf());
    let started = Instant::now();
    let timeout = options
        .scan_timeout_per_root_millis
        .map(Duration::from_millis);

    if !root_path.exists() {
        root.status = RootStatus::Missing;
        root.error = Some(RootError {
            code: "missing".to_string(),
            message: "root path does not exist".to_string(),
        });
        return DirectoryScanOutput {
            root,
            children: Vec::new(),
            directory_states: DirectoryStateMap::new(),
            truncated: false,
        };
    }

    let mut state = FileListWalkState {
        options,
        extensions,
        timeout,
        started,
        visited_directories: BTreeSet::new(),
        visited_nodes: 0,
        directory_states: DirectoryStateMap::new(),
        truncated: false,
        timed_out: false,
    };

    let root_resolution =
        resolve_scannable_path(root_path.to_path_buf(), root_path.to_path_buf(), options);
    let root_metadata = match fs::metadata(&root_resolution.resolved_path) {
        Ok(metadata) => metadata,
        Err(error) => {
            root.status = RootStatus::Missing;
            root.error = Some(RootError {
                code: "unresolved_root".to_string(),
                message: error.to_string(),
            });
            return DirectoryScanOutput {
                root,
                children: Vec::new(),
                directory_states: state.directory_states,
                truncated: state.truncated,
            };
        }
    };
    if !root_metadata.is_dir() {
        root.status = RootStatus::Missing;
        root.error = Some(RootError {
            code: "not_directory".to_string(),
            message: "root path does not resolve to a directory".to_string(),
        });
        return DirectoryScanOutput {
            root,
            children: Vec::new(),
            directory_states: state.directory_states,
            truncated: state.truncated,
        };
    }

    let children = scan_directory(root_path, &root_resolution.resolved_path, 0, &mut state)
        .unwrap_or_default();
    root.item_count = state.visited_nodes;
    root.last_loaded_at = Some(now_timestamp_millis());
    root.is_dirty = false;
    root.status = if state.timed_out {
        RootStatus::TimedOut
    } else {
        RootStatus::Ready
    };

    DirectoryScanOutput {
        root,
        children,
        directory_states: state.directory_states,
        truncated: state.truncated,
    }
}

struct FileListWalkState<'a> {
    options: &'a ScannerOptions,
    extensions: &'a ExtensionPolicy,
    timeout: Option<Duration>,
    started: Instant,
    visited_directories: BTreeSet<PathBuf>,
    visited_nodes: usize,
    directory_states: DirectoryStateMap,
    truncated: bool,
    timed_out: bool,
}

fn scan_directory(
    display_directory: &Path,
    source_directory: &Path,
    depth: usize,
    state: &mut FileListWalkState<'_>,
) -> Option<Vec<FileSystemEntry>> {
    if should_stop(depth, state) {
        return None;
    }

    let resolved_directory = normalize_path(source_directory);
    if !state.visited_directories.insert(resolved_directory.clone()) {
        return Some(Vec::new());
    }

    let entries = match fs::read_dir(source_directory) {
        Ok(entries) => entries,
        Err(_) => return Some(Vec::new()),
    };

    let mut children = Vec::new();
    for entry in entries.flatten() {
        if should_stop(depth, state) {
            break;
        }

        let source_path = entry.path();
        let display_path = display_directory.join(entry.file_name());
        if should_skip_path(&display_path, state.options) {
            continue;
        }

        let resolved = resolve_scannable_path(display_path, source_path, state.options);
        if should_include_resolution_error(&resolved) {
            children.push(resolution_error_entry(resolved));
            state.visited_nodes += 1;
            continue;
        }
        if state.options.skip_packages && is_package_path(&resolved.resolved_path) {
            continue;
        }

        let metadata = match fs::metadata(&resolved.resolved_path) {
            Ok(metadata) => metadata,
            Err(error) => {
                if should_include_resolution_error(&resolved) {
                    children.push(resolution_error_entry(
                        resolved.with_status(path_error_status(&error), Some(error.to_string())),
                    ));
                    state.visited_nodes += 1;
                }
                continue;
            }
        };

        if metadata.is_dir() {
            let resolved_directory = normalize_path(&resolved.resolved_path);
            let is_cycle = state.visited_directories.contains(&resolved_directory);
            let (resolved, nested_children) = if is_cycle {
                (
                    resolved.with_status(
                        PathResolutionStatus::Cycle,
                        Some("resolved path was already visited".to_string()),
                    ),
                    Vec::new(),
                )
            } else {
                let nested_children = scan_directory(
                    &resolved.display_path,
                    &resolved.resolved_path,
                    depth + 1,
                    state,
                )
                .unwrap_or_default();
                (resolved, nested_children)
            };
            if state.options.include_empty_directories
                || !nested_children.is_empty()
                || resolved.resolution_status == PathResolutionStatus::Cycle
            {
                children.push(FileSystemEntry {
                    display_path: resolved.display_path,
                    resolved_path: resolved.resolved_path,
                    path_kind: resolved.path_kind,
                    resolution_status: resolved.resolution_status,
                    resolution_message: resolved.resolution_message,
                    is_directory: true,
                    file_size: None,
                    content_modified_at: None,
                    runtime_kind: None,
                    shebang: None,
                    has_scriptmeta: false,
                    has_scriptmeta_edit_password: false,
                    is_file_locked: false,
                    is_read_only: false,
                    can_edit_scriptmeta: false,
                    can_append_scriptmeta: false,
                    scriptmeta_edit_state: crate::core::ScriptMetaEditState::Unsupported,
                    children: nested_children,
                });
                state.visited_nodes += 1;
            }
            continue;
        }

        if metadata.is_file() && state.extensions.contains_path(&resolved.resolved_path) {
            let file_info = detect_script_info_for_file_list(&resolved.resolved_path);
            children.push(FileSystemEntry {
                display_path: resolved.display_path,
                resolved_path: resolved.resolved_path,
                path_kind: resolved.path_kind,
                resolution_status: resolved.resolution_status,
                resolution_message: resolved.resolution_message,
                is_directory: false,
                file_size: Some(metadata.len()),
                content_modified_at: metadata.modified().ok().and_then(system_time_millis),
                runtime_kind: file_info.script.runtime_kind,
                shebang: file_info.script.shebang,
                has_scriptmeta: file_info.capability.has_scriptmeta,
                has_scriptmeta_edit_password: file_info.capability.has_scriptmeta_edit_password,
                is_file_locked: file_info.capability.is_file_locked,
                is_read_only: file_info.capability.is_read_only,
                can_edit_scriptmeta: file_info.capability.can_edit_scriptmeta,
                can_append_scriptmeta: file_info.capability.can_append_scriptmeta,
                scriptmeta_edit_state: file_info.capability.scriptmeta_edit_state,
                children: Vec::new(),
            });
            state.visited_nodes += 1;
        }
    }

    children.sort_by(|lhs, rhs| {
        rhs.is_directory
            .cmp(&lhs.is_directory)
            .then_with(|| display_name(&lhs.display_path).cmp(&display_name(&rhs.display_path)))
    });

    let fingerprint = child_fingerprint(&children);
    state.directory_states.insert(
        resolved_directory.to_string_lossy().into_owned(),
        DirectoryState {
            modification_time_millis: modification_time_millis(source_directory),
            child_count: children.len(),
            child_fingerprint: fingerprint,
        },
    );

    Some(children)
}

fn should_stop(depth: usize, state: &mut FileListWalkState<'_>) -> bool {
    if depth > state.options.max_depth || state.visited_nodes >= state.options.max_nodes_per_root {
        state.truncated = true;
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
    let name = display_name(path);
    if options.skip_hidden && name.starts_with('.') {
        return true;
    }

    options.skip_packages && is_package_path(path)
}

fn should_include_resolution_error(resolved: &ResolvedPath) -> bool {
    resolved.path_kind != PathKind::Normal
        && resolved.resolution_status != PathResolutionStatus::NotRequested
        && resolved.resolution_status != PathResolutionStatus::Resolved
}

fn resolution_error_entry(resolved: ResolvedPath) -> FileSystemEntry {
    FileSystemEntry {
        display_path: resolved.display_path,
        resolved_path: resolved.resolved_path,
        path_kind: resolved.path_kind,
        resolution_status: resolved.resolution_status,
        resolution_message: resolved.resolution_message,
        is_directory: false,
        file_size: None,
        content_modified_at: None,
        runtime_kind: None,
        shebang: None,
        has_scriptmeta: false,
        has_scriptmeta_edit_password: false,
        is_file_locked: false,
        is_read_only: false,
        can_edit_scriptmeta: false,
        can_append_scriptmeta: false,
        scriptmeta_edit_state: crate::core::ScriptMetaEditState::Unknown,
        children: Vec::new(),
    }
}

fn is_package_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("app" | "bundle" | "framework" | "plugin" | "appex")
    )
}

fn display_name(path: &Path) -> Cow<'_, str> {
    path.file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or(Cow::Borrowed(""))
}

struct FileListScriptInfo {
    script: super::script_detection::ScriptFileInfo,
    capability: crate::core::ScriptMetaEditCapability,
}

fn detect_script_info_for_file_list(path: &Path) -> FileListScriptInfo {
    if !needs_script_header_probe(path) {
        return FileListScriptInfo {
            script: detect_script_file(path, None),
            capability: scriptmeta_edit_capability_from_file_list_probe(path, None),
        };
    }

    let Ok(mut file) = File::open(path) else {
        return FileListScriptInfo {
            script: detect_script_file(path, None),
            capability: scriptmeta_edit_capability_from_file_list_probe(path, None),
        };
    };

    let mut buffer = vec![0; script_header_probe_byte_limit(path)];
    let read_count = file.read(&mut buffer).unwrap_or(0);
    let prefix_bytes = &buffer[..read_count];
    FileListScriptInfo {
        script: detect_script_file_from_bytes(path, Some(prefix_bytes)),
        capability: scriptmeta_edit_capability_from_file_list_probe(path, Some(prefix_bytes)),
    }
}

fn child_fingerprint(children: &[FileSystemEntry]) -> u64 {
    let mut hash = 14_695_981_039_346_656_037u64;
    for child in children {
        for byte in display_name(&child.display_path).as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
        hash ^= u64::from(child.is_directory);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}

fn modification_time_millis(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(system_time_millis)
}

pub(crate) fn system_time_millis(time: SystemTime) -> Option<u64> {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
}
