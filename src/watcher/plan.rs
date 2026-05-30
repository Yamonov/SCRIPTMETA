use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    RootId,
    catalog::RootRegistration,
    scanner::ExtensionPolicy,
    watcher::{MonitorRootStrategy, OverflowPolicy, WatchPolicy},
};

pub const DEFAULT_DEBOUNCE_DELAY_MILLIS: u64 = 500;
pub const DEFAULT_MAX_DELIVERY_DELAY_MILLIS: u64 = 2_000;
pub const DEFAULT_MAX_PENDING_PATHS: usize = 1_024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WatchPlan {
    pub physical_roots: Vec<PhysicalWatchRoot>,
    pub logical_roots: Vec<LogicalWatchRoot>,
    pub debounce_delay_millis: u64,
    pub max_delivery_delay_millis: u64,
    pub max_pending_paths: usize,
    pub supported_extensions: ExtensionPolicy,
    pub skip_hidden_paths: bool,
    pub skip_package_paths: bool,
}

impl WatchPlan {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            physical_roots: Vec::new(),
            logical_roots: Vec::new(),
            debounce_delay_millis: DEFAULT_DEBOUNCE_DELAY_MILLIS,
            max_delivery_delay_millis: DEFAULT_MAX_DELIVERY_DELAY_MILLIS,
            max_pending_paths: DEFAULT_MAX_PENDING_PATHS,
            supported_extensions: ExtensionPolicy::script_default(),
            skip_hidden_paths: true,
            skip_package_paths: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PhysicalWatchRoot {
    pub path: PathBuf,
    pub covers_root_ids: Vec<RootId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LogicalWatchRoot {
    pub root_id: RootId,
    pub path: PathBuf,
    pub physical_index: Option<usize>,
    pub active: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RawChangeBatch {
    pub paths: Vec<PathBuf>,
    pub overflowed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RootChangeBatch {
    pub paths: Vec<PathBuf>,
    pub overflowed: bool,
    pub affected_roots: Vec<RootChange>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RootChange {
    pub root_id: RootId,
    pub dirty_directories: Vec<PathBuf>,
    pub metadata_may_have_changed: bool,
    pub requires_full_rescan: bool,
}

pub fn build_watch_plan(
    roots: &[RootRegistration],
    global_policy: WatchPolicy,
    strategy: MonitorRootStrategy,
    visible_root_id: Option<&RootId>,
) -> WatchPlan {
    if matches!(global_policy, WatchPolicy::Disabled | WatchPolicy::Manual) {
        return WatchPlan::empty();
    }

    let mut candidates = Vec::with_capacity(roots.len());
    candidates.extend(
        roots
            .iter()
            .filter(|root| should_watch_root(root, global_policy, visible_root_id))
            .map(|root| (root.root_id.clone(), normalize_path(&root.path))),
    );

    candidates.sort_by(|lhs, rhs| {
        lhs.1
            .components()
            .count()
            .cmp(&rhs.1.components().count())
            .then_with(|| lhs.1.cmp(&rhs.1))
            .then_with(|| lhs.0.cmp(&rhs.0))
    });

    let mut plan = WatchPlan::empty();
    for (root_id, path) in candidates {
        let physical_index = match strategy {
            MonitorRootStrategy::ExactRoots => physical_index_for_exact_path(&mut plan, &path),
            MonitorRootStrategy::DeduplicateNestedRoots
            | MonitorRootStrategy::PlatformRecommended => {
                physical_index_for_deduplicated_path(&mut plan, &path)
            }
        };

        if let Some(physical_root) = plan.physical_roots.get_mut(physical_index)
            && !physical_root.covers_root_ids.contains(&root_id)
        {
            physical_root.covers_root_ids.push(root_id.clone());
        }

        plan.logical_roots.push(LogicalWatchRoot {
            root_id,
            path,
            physical_index: Some(physical_index),
            active: true,
        });
    }

    plan
}

pub(crate) struct ChangeRoutingOptions<'a> {
    pub extensions: &'a ExtensionPolicy,
    pub skip_hidden_paths: bool,
    pub skip_package_paths: bool,
    pub known_directory_paths: &'a BTreeSet<PathBuf>,
    pub overflow_policy: OverflowPolicy,
    pub max_deferred_dirty_directories: usize,
}

pub(crate) fn route_change_batch(
    roots: &[RootRegistration],
    raw: RawChangeBatch,
    options: ChangeRoutingOptions<'_>,
) -> RootChangeBatch {
    let raw_overflowed = raw.overflowed;
    let normalized_paths: Vec<PathBuf> = raw
        .paths
        .into_iter()
        .map(|path| normalize_path(&path))
        .collect();
    let mut observed_paths = BTreeSet::new();
    let mut affected_roots = Vec::new();

    for root in roots {
        let root_path = normalize_path(&root.path);
        let filtered_paths: Vec<PathBuf> = normalized_paths
            .iter()
            .filter(|path| path_affects_root(path, &root_path))
            .filter(|path| {
                raw_overflowed
                    || should_observe_change_path(
                        path,
                        &root_path,
                        options.extensions,
                        options.skip_hidden_paths,
                        options.skip_package_paths,
                        options.known_directory_paths,
                    )
            })
            .cloned()
            .collect();
        observed_paths.extend(filtered_paths.iter().cloned());

        let affects_root = raw_overflowed
            && options.overflow_policy == OverflowPolicy::MarkAllRootsDirty
            || filtered_paths
                .iter()
                .any(|changed_path| path_affects_root(changed_path, &root_path));

        if !affects_root {
            continue;
        }

        let mut dirty_directories = if raw_overflowed {
            vec![root_path.clone()]
        } else {
            dirty_directories_for_paths(&filtered_paths, &root_path)
        };
        dirty_directories.sort();
        dirty_directories.dedup();

        let requires_full_rescan = raw_overflowed
            || dirty_directories.len() > options.max_deferred_dirty_directories
            || dirty_directories.iter().any(|path| path == &root_path);

        if requires_full_rescan {
            dirty_directories = vec![root_path.clone()];
        }

        let metadata_may_have_changed = raw_overflowed || !filtered_paths.is_empty();

        affected_roots.push(RootChange {
            root_id: root.root_id.clone(),
            dirty_directories,
            metadata_may_have_changed,
            requires_full_rescan,
        });
    }

    RootChangeBatch {
        paths: observed_paths.into_iter().collect(),
        overflowed: raw_overflowed,
        affected_roots,
    }
}

fn should_watch_root(
    root: &RootRegistration,
    global_policy: WatchPolicy,
    visible_root_id: Option<&RootId>,
) -> bool {
    if matches!(
        root.watch_policy,
        WatchPolicy::Disabled | WatchPolicy::Manual
    ) {
        return false;
    }

    match global_policy {
        WatchPolicy::Disabled | WatchPolicy::Manual => false,
        WatchPolicy::VisibleOnly => visible_root_id == Some(&root.root_id),
        WatchPolicy::AllRegistered => match root.watch_policy {
            WatchPolicy::VisibleOnly => visible_root_id == Some(&root.root_id),
            WatchPolicy::AllRegistered => true,
            WatchPolicy::Disabled | WatchPolicy::Manual => false,
        },
    }
}

fn physical_index_for_exact_path(plan: &mut WatchPlan, path: &Path) -> usize {
    if let Some(index) = plan
        .physical_roots
        .iter()
        .position(|root| root.path == path)
    {
        return index;
    }

    plan.physical_roots.push(PhysicalWatchRoot {
        path: path.to_path_buf(),
        covers_root_ids: Vec::new(),
    });
    plan.physical_roots.len() - 1
}

fn physical_index_for_deduplicated_path(plan: &mut WatchPlan, path: &Path) -> usize {
    if let Some(index) = plan
        .physical_roots
        .iter()
        .position(|root| path_is_same_or_child(path, &root.path))
    {
        return index;
    }

    physical_index_for_exact_path(plan, path)
}

fn dirty_directories_for_paths(paths: &[PathBuf], root_path: &Path) -> Vec<PathBuf> {
    paths
        .iter()
        .filter(|path| path_affects_root(path, root_path))
        .map(|path| {
            if path_is_same_or_child(path, root_path) {
                path.parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| root_path.to_path_buf())
            } else {
                root_path.to_path_buf()
            }
        })
        .collect()
}

pub(crate) fn should_observe_change_path(
    path: &Path,
    root_path: &Path,
    extensions: &ExtensionPolicy,
    skip_hidden_paths: bool,
    skip_package_paths: bool,
    known_directory_paths: &BTreeSet<PathBuf>,
) -> bool {
    if skip_hidden_paths && path_has_hidden_component(path, root_path) {
        return false;
    }

    if skip_package_paths && path_has_package_component(path, root_path) {
        return false;
    }

    extensions.contains_path(path)
        || path_is_existing_directory(path)
        || known_directory_paths.contains(path)
}

fn path_has_hidden_component(path: &Path, root_path: &Path) -> bool {
    relative_change_path(path, root_path)
        .components()
        .any(|component| component.as_os_str().as_encoded_bytes().starts_with(b"."))
}

fn path_has_package_component(path: &Path, root_path: &Path) -> bool {
    relative_change_path(path, root_path)
        .components()
        .any(|component| {
            let component_path = Path::new(component.as_os_str());
            matches!(
                component_path
                    .extension()
                    .and_then(|extension| extension.to_str()),
                Some("app" | "bundle" | "framework" | "plugin" | "appex")
            )
        })
}

fn relative_change_path<'a>(path: &'a Path, root_path: &Path) -> &'a Path {
    path.strip_prefix(root_path).unwrap_or(path)
}

fn path_is_existing_directory(path: &Path) -> bool {
    path.metadata().is_ok_and(|metadata| metadata.is_dir())
}

fn path_affects_root(changed_path: &Path, root_path: &Path) -> bool {
    path_is_same_or_child(changed_path, root_path) || path_is_same_or_child(root_path, changed_path)
}

fn path_is_same_or_child(path: &Path, parent: &Path) -> bool {
    path == parent || path.starts_with(parent)
}

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    if let Ok(canonical_path) = path.canonicalize() {
        return canonical_path;
    }

    if let (Some(parent), Some(file_name)) = (path.parent(), path.file_name())
        && parent != path
    {
        return normalize_path(parent).join(file_name);
    }

    lexical_normalize(path)
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}
