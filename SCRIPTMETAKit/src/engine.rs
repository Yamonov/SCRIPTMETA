use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use uuid::Uuid;

use crate::{
    RootId,
    catalog::{
        CacheInvalidationReason, CacheScope, FileEntryChange, FileEntryChangeKind,
        FileListSnapshot, RefreshRequest, RootPurpose, RootRegistration, RootSnapshot, RootStatus,
        ScanChangeSummary, ScanMode, ScanRequest, ScanResult, ScriptMetaCatalogSnapshot,
        ScriptMetaKitConfig, ScriptMetaKitEvent, UpdateCheckProgress, UpdateCheckProgressPhase,
        UpdateCheckRequest, UpdateCheckResult, UpdateStatus, path_based_root_id,
        unresolved_distribution,
    },
    core::{ScriptMetaItem, ScriptMetaKitError, ScriptMetaKitResult},
    now_timestamp_millis,
    resolver::{DistributionResolverOptions, UpdateResolver},
    scanner::{
        CandidateCache, CandidateRecord, FileSystemEntry, deduplicated_items,
        file_items_from_cache, registered_root_signatures, scan_file_list_root,
        scan_metadata_roots,
    },
    storage::CachePayload,
    watcher::{
        ChangeRoutingOptions, RawChangeBatch, WatchPlan, build_watch_plan, normalize_path,
        route_change_batch,
    },
};

#[derive(Clone, Debug)]
pub struct ScriptMetaKitEngine {
    config: ScriptMetaKitConfig,
    roots: Vec<RootRegistration>,
    visible_root_id: Option<RootId>,
    root_snapshots: BTreeMap<RootId, RootSnapshot>,
    file_list_snapshots: BTreeMap<RootId, FileListSnapshot>,
    catalog_snapshot: Option<ScriptMetaCatalogSnapshot>,
}

impl ScriptMetaKitEngine {
    pub fn new(config: ScriptMetaKitConfig) -> ScriptMetaKitResult<Self> {
        validate_config(&config)?;
        Ok(Self {
            config,
            roots: Vec::new(),
            visible_root_id: None,
            root_snapshots: BTreeMap::new(),
            file_list_snapshots: BTreeMap::new(),
            catalog_snapshot: None,
        })
    }

    #[must_use]
    pub fn config(&self) -> &ScriptMetaKitConfig {
        &self.config
    }

    #[must_use]
    pub fn config_mut(&mut self) -> &mut ScriptMetaKitConfig {
        &mut self.config
    }

    pub fn set_roots(
        &mut self,
        roots: Vec<RootRegistration>,
    ) -> ScriptMetaKitResult<Vec<ScriptMetaKitEvent>> {
        let previous_root_ids: BTreeSet<_> = self
            .roots
            .iter()
            .map(|root| root.root_id.as_str())
            .collect();
        let next_root_ids: BTreeSet<_> = roots.iter().map(|root| root.root_id.as_str()).collect();
        if next_root_ids.len() != roots.len() {
            return Err(ScriptMetaKitError::InvalidConfig(
                "root_id values must be unique".to_string(),
            ));
        }
        let mut events = Vec::new();
        for removed_id in previous_root_ids.difference(&next_root_ids) {
            self.root_snapshots.remove(*removed_id);
            self.file_list_snapshots.remove(*removed_id);
            events.push(ScriptMetaKitEvent::RootRemoved {
                root_id: (*removed_id).to_string(),
            });
        }

        for root in &roots {
            if !previous_root_ids.contains(root.root_id.as_str()) {
                events.push(ScriptMetaKitEvent::RootRegistered {
                    root_id: root.root_id.clone(),
                });
            }
            self.root_snapshots
                .entry(root.root_id.clone())
                .or_insert_with(|| RootSnapshot::new(root.root_id.clone(), root.path.clone()));
        }

        self.roots = roots;
        Ok(events)
    }

    #[must_use]
    pub fn roots(&self) -> &[RootRegistration] {
        &self.roots
    }

    pub fn set_root_paths<I, P>(
        &mut self,
        paths: I,
        purpose: RootPurpose,
    ) -> ScriptMetaKitResult<Vec<ScriptMetaKitEvent>>
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        let roots = paths
            .into_iter()
            .map(|path| {
                let path = path.into();
                let root_id = path_based_root_id(&path);
                let display_name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                RootRegistration::user_initiated(root_id, path, purpose)
                    .with_display_name(display_name)
            })
            .collect();
        self.set_roots(roots)
    }

    pub fn scan_root_paths<I, P>(
        &mut self,
        paths: I,
        mode: ScanMode,
    ) -> ScriptMetaKitResult<ScanResult>
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        self.set_root_paths(paths, RootPurpose::from_scan_mode(mode))?;
        self.scan_roots(ScanRequest::all(mode))
    }

    pub fn set_visible_root(&mut self, root_id: Option<RootId>) {
        self.visible_root_id = root_id;
    }

    #[must_use]
    pub fn watch_plan(&self) -> WatchPlan {
        if !self.config.watcher.enabled {
            return self.watch_plan_with_delivery_options(WatchPlan::empty());
        }
        let plan = build_watch_plan(
            &self.roots,
            self.config.watcher.watch_policy,
            self.config.watcher.monitor_root_strategy,
            self.visible_root_id.as_ref(),
        );
        self.watch_plan_with_delivery_options(plan)
    }

    #[must_use]
    pub fn snapshot(&self, root_id: &RootId) -> Option<FileListSnapshot> {
        self.file_list_snapshots.get(root_id).cloned()
    }

    #[must_use]
    pub fn catalog_snapshot(&self) -> Option<ScriptMetaCatalogSnapshot> {
        self.catalog_snapshot.clone()
    }

    pub fn scan_roots(&mut self, request: ScanRequest) -> ScriptMetaKitResult<ScanResult> {
        let selected_root_indices = self.selected_root_indices(&request.root_ids);
        let root_ids: Vec<_> = selected_root_indices
            .iter()
            .map(|index| self.roots[*index].root_id.clone())
            .collect();
        let mut file_list_snapshots = Vec::new();
        let mut saw_previous_file_list = false;
        let mut previous_file_list_snapshots = BTreeMap::new();
        let mut change_summary = ScanChangeSummary::default();

        if request.mode.includes_file_list() {
            for root_index in &selected_root_indices {
                let root = &self.roots[*root_index];
                let root_id = root.root_id.clone();
                let output = scan_file_list_root(
                    &root.root_id,
                    &root.path,
                    &self.config.scanner,
                    &self.config.supported_extensions,
                );
                let previous_snapshot = self.file_list_snapshots.get(&root_id).cloned();
                let snapshot = FileListSnapshot {
                    root: output.root.clone(),
                    children: Some(output.children),
                    directory_states: output.directory_states,
                    truncated: output.truncated,
                };
                if let Some(previous_snapshot) = previous_snapshot {
                    saw_previous_file_list = true;
                    previous_file_list_snapshots.insert(root_id.clone(), previous_snapshot);
                }
                self.root_snapshots.insert(root_id.clone(), output.root);
                self.file_list_snapshots.insert(root_id, snapshot.clone());
                file_list_snapshots.push(snapshot);
            }
        }

        let catalog_snapshot = if request.mode.includes_metadata() {
            let previous_catalog = self.catalog_snapshot.clone();
            let previous_cache = previous_catalog
                .as_ref()
                .map(|snapshot| &snapshot.candidate_cache);
            let output = scan_metadata_roots(
                selected_root_indices
                    .iter()
                    .map(|index| &self.roots[*index]),
                &self.config.scanner,
                &self.config.supported_extensions,
                previous_cache,
            );
            for root in &output.roots {
                self.root_snapshots
                    .insert(root.root_id.clone(), root.clone());
            }
            let update_check_result =
                self.preserved_update_result(previous_catalog.as_ref(), &output.all_items);
            let snapshot = ScriptMetaCatalogSnapshot {
                source_revision: output.source_revision,
                roots: output.roots,
                all_items: output.all_items,
                file_items: output.file_items,
                candidate_cache: output.candidate_cache,
                update_check_result,
            };
            if request.mode.includes_file_list() {
                apply_metadata_capabilities_to_file_list_snapshots(
                    &mut file_list_snapshots,
                    &snapshot.candidate_cache.records,
                );
                for snapshot in &file_list_snapshots {
                    self.file_list_snapshots
                        .insert(snapshot.root.root_id.clone(), snapshot.clone());
                }
            }
            self.catalog_snapshot = Some(snapshot.clone());
            Some(snapshot)
        } else {
            None
        };

        if request.mode.includes_file_list()
            && !request.mode.includes_metadata()
            && let Some(snapshot) = self.catalog_snapshot.as_ref()
        {
            apply_metadata_capabilities_to_file_list_snapshots(
                &mut file_list_snapshots,
                &snapshot.candidate_cache.records,
            );
            for snapshot in &file_list_snapshots {
                self.file_list_snapshots
                    .insert(snapshot.root.root_id.clone(), snapshot.clone());
            }
        }

        for snapshot in &file_list_snapshots {
            if let Some(previous_snapshot) =
                previous_file_list_snapshots.get(&snapshot.root.root_id)
            {
                change_summary.extend(diff_file_list_snapshot(
                    &snapshot.root.root_id,
                    previous_snapshot,
                    snapshot,
                ));
            }
        }

        let roots = self.snapshots_for_roots(&root_ids);
        Ok(ScanResult {
            roots,
            file_list_snapshots,
            catalog_snapshot,
            change_summary: saw_previous_file_list.then_some(change_summary),
        })
    }

    pub fn mark_changed_paths(
        &mut self,
        batch: RawChangeBatch,
    ) -> ScriptMetaKitResult<Vec<ScriptMetaKitEvent>> {
        let known_directory_paths = self.known_directory_paths();
        let change_batch = route_change_batch(
            &self.roots,
            batch,
            ChangeRoutingOptions {
                extensions: &self.config.supported_extensions,
                skip_hidden_paths: self.config.scanner.skip_hidden,
                skip_package_paths: self.config.scanner.skip_packages,
                known_directory_paths: &known_directory_paths,
                overflow_policy: self.config.watcher.overflow_policy,
                max_deferred_dirty_directories: self.config.cache.max_deferred_dirty_directories,
            },
        );
        if change_batch.affected_roots.is_empty() && !change_batch.overflowed {
            return Ok(Vec::new());
        }

        let now = now_timestamp_millis();
        let mut root_events = Vec::with_capacity(change_batch.affected_roots.len() + 1);

        for change in &change_batch.affected_roots {
            if let Some(snapshot) = self.root_snapshots.get_mut(&change.root_id) {
                snapshot.is_dirty = true;
                snapshot.last_event_at = Some(now);
                snapshot.status = if change.requires_full_rescan {
                    RootStatus::Overflowed
                } else {
                    RootStatus::Dirty
                };
            }
            root_events.push(ScriptMetaKitEvent::RootMarkedDirty {
                root_id: change.root_id.clone(),
            });
        }

        if change_batch.overflowed {
            root_events.push(ScriptMetaKitEvent::WatchOverflowed {
                affected_roots: change_batch
                    .affected_roots
                    .iter()
                    .map(|change| change.root_id.clone())
                    .collect(),
            });
        }

        let mut events = Vec::with_capacity(root_events.len() + 1);
        events.push(ScriptMetaKitEvent::ChangeDetected {
            batch: change_batch,
        });
        events.extend(root_events);
        Ok(events)
    }

    pub fn refresh_dirty_roots(
        &mut self,
        request: RefreshRequest,
    ) -> ScriptMetaKitResult<ScanResult> {
        let dirty_root_ids = self
            .root_snapshots
            .values()
            .filter(|snapshot| snapshot.is_dirty)
            .map(|snapshot| snapshot.root_id.clone())
            .collect::<Vec<_>>();
        let all_root_ids: Vec<_> = self.roots.iter().map(|root| root.root_id.clone()).collect();
        if dirty_root_ids.is_empty() {
            return Ok(ScanResult {
                roots: self.snapshots_for_roots(&all_root_ids),
                file_list_snapshots: self.file_list_snapshots_for_roots(&all_root_ids),
                catalog_snapshot: request
                    .mode
                    .includes_metadata()
                    .then(|| self.catalog_snapshot.clone())
                    .flatten(),
                change_summary: None,
            });
        }

        let previous_catalog = self.catalog_snapshot.clone();
        let partial_result = self.scan_roots(ScanRequest {
            root_ids: dirty_root_ids,
            mode: request.mode,
        })?;

        if request.mode.includes_metadata()
            && let Some(partial_catalog) = partial_result.catalog_snapshot.as_ref()
        {
            let merged_catalog =
                self.merge_catalog_snapshot(previous_catalog.as_ref(), partial_catalog);
            self.catalog_snapshot = Some(merged_catalog);
        }

        Ok(ScanResult {
            roots: self.snapshots_for_roots(&all_root_ids),
            file_list_snapshots: self.file_list_snapshots_for_roots(&all_root_ids),
            catalog_snapshot: request
                .mode
                .includes_metadata()
                .then(|| self.catalog_snapshot.clone())
                .flatten(),
            change_summary: partial_result.change_summary,
        })
    }

    pub async fn check_updates(
        &mut self,
        request: UpdateCheckRequest,
    ) -> ScriptMetaKitResult<UpdateCheckResult> {
        self.check_updates_with_progress(request, |_| {}).await
    }

    pub async fn check_updates_with_progress(
        &mut self,
        request: UpdateCheckRequest,
        mut progress: impl FnMut(UpdateCheckProgress),
    ) -> ScriptMetaKitResult<UpdateCheckResult> {
        let checked_at = now_timestamp_millis();
        let mut result = UpdateCheckResult {
            checked_at,
            resolutions_by_item_id: BTreeMap::new(),
            failures_by_item_id: BTreeMap::new(),
            errors_by_item_id: BTreeMap::new(),
            statuses_by_item_id: BTreeMap::new(),
        };
        let total_items = request.items.len();

        let update_resolver = UpdateResolver::new(DistributionResolverOptions {
            request_timeout_millis: self.config.update_check.request_timeout_millis,
            ..DistributionResolverOptions::default()
        })?;
        let retry_attempts = self.config.update_check.retry_attempts;

        progress(UpdateCheckProgress {
            completed_items: 0,
            total_items,
            item_id: None,
            script_id: None,
            phase: UpdateCheckProgressPhase::Started,
            message: format!("Starting update check for {total_items} item(s)"),
        });

        for (index, item) in request.items.into_iter().enumerate() {
            let item_id = item.item_id();
            let script_id = item.script_id.clone();
            progress(UpdateCheckProgress {
                completed_items: index,
                total_items,
                item_id: Some(item_id.clone()),
                script_id: Some(script_id.clone()),
                phase: UpdateCheckProgressPhase::Checking,
                message: format!("{}/{} checking {}", index + 1, total_items, script_id),
            });

            let mut attempt = 0usize;
            let resolved_result = loop {
                match update_resolver.resolve_item(&item) {
                    Ok(resolved) => break Ok(resolved),
                    Err(error) if attempt < retry_attempts => {
                        attempt += 1;
                        progress(UpdateCheckProgress {
                            completed_items: index,
                            total_items,
                            item_id: Some(item_id.clone()),
                            script_id: Some(script_id.clone()),
                            phase: UpdateCheckProgressPhase::Retrying,
                            message: format!(
                                "{}/{} retrying {} after failure: {}",
                                index + 1,
                                total_items,
                                script_id,
                                error
                            ),
                        });
                    }
                    Err(error) => break Err(error),
                }
            };

            match resolved_result {
                Ok(resolved) => {
                    let resolved_status = resolved.status;
                    if let Some(resolution) = resolved.resolution {
                        if resolved_status == UpdateStatus::Failed {
                            let failure = crate::catalog::UpdateFailure::unresolved_distribution(
                                item_id.clone(),
                                &item,
                                &resolution,
                            );
                            result
                                .errors_by_item_id
                                .insert(item_id.clone(), failure.message.clone());
                            result.failures_by_item_id.insert(item_id.clone(), failure);
                        }
                        result
                            .resolutions_by_item_id
                            .insert(item_id.clone(), resolution);
                    }
                    if let Some(error) = resolved.error {
                        let failure = crate::catalog::UpdateFailure::from_message(
                            item_id.clone(),
                            &item,
                            resolved.checked_at,
                            "update_check_failed",
                            error,
                            None,
                        );
                        result
                            .errors_by_item_id
                            .insert(item_id.clone(), failure.message.clone());
                        result.failures_by_item_id.insert(item_id.clone(), failure);
                    }
                    result
                        .statuses_by_item_id
                        .insert(item_id.clone(), resolved_status);
                    let failed = resolved_status == UpdateStatus::Failed;
                    progress(UpdateCheckProgress {
                        completed_items: index + 1,
                        total_items,
                        item_id: Some(item_id),
                        script_id: Some(script_id.clone()),
                        phase: if failed {
                            UpdateCheckProgressPhase::FailedItem
                        } else {
                            UpdateCheckProgressPhase::FinishedItem
                        },
                        message: format!(
                            "{}/{} {} {}",
                            index + 1,
                            total_items,
                            if failed { "failed" } else { "finished" },
                            script_id
                        ),
                    });
                }
                Err(error) => {
                    let message = error.to_string();
                    let Some(meta_url) = item.meta_url.clone() else {
                        result
                            .statuses_by_item_id
                            .insert(item_id.clone(), UpdateStatus::NotCheckable);
                        progress(UpdateCheckProgress {
                            completed_items: index + 1,
                            total_items,
                            item_id: Some(item_id),
                            script_id: Some(script_id.clone()),
                            phase: UpdateCheckProgressPhase::FinishedItem,
                            message: format!("{}/{} skipped {}", index + 1, total_items, script_id),
                        });
                        continue;
                    };
                    let failure = crate::catalog::UpdateFailure::from_error(
                        item_id.clone(),
                        &item,
                        checked_at,
                        &error,
                    );
                    result.resolutions_by_item_id.insert(
                        item_id.clone(),
                        unresolved_distribution(meta_url, checked_at, failure.message.clone()),
                    );
                    result.errors_by_item_id.insert(item_id.clone(), message);
                    result.failures_by_item_id.insert(item_id.clone(), failure);
                    result
                        .statuses_by_item_id
                        .insert(item_id.clone(), UpdateStatus::Failed);
                    progress(UpdateCheckProgress {
                        completed_items: index + 1,
                        total_items,
                        item_id: Some(item_id),
                        script_id: Some(script_id.clone()),
                        phase: UpdateCheckProgressPhase::FailedItem,
                        message: format!("{}/{} failed {}", index + 1, total_items, script_id),
                    });
                }
            }
        }

        progress(UpdateCheckProgress {
            completed_items: total_items,
            total_items,
            item_id: None,
            script_id: None,
            phase: UpdateCheckProgressPhase::Finished,
            message: format!("Finished update check for {total_items} item(s)"),
        });

        if let Some(snapshot) = &mut self.catalog_snapshot {
            snapshot.update_check_result = Some(result.clone());
        }

        Ok(result)
    }

    pub fn load_cache(
        &mut self,
        payload: CachePayload,
    ) -> ScriptMetaKitResult<Vec<ScriptMetaKitEvent>> {
        match payload.scope {
            CacheScope::Catalog | CacheScope::All => {
                let snapshot: ScriptMetaCatalogSnapshot = serde_json::from_value(payload.data)
                    .map_err(|error| ScriptMetaKitError::Cache(error.to_string()))?;
                self.catalog_snapshot = Some(snapshot);
                Ok(vec![ScriptMetaKitEvent::CacheLoaded {
                    scope: CacheScope::Catalog,
                }])
            }
            CacheScope::FileList | CacheScope::Root => Ok(vec![ScriptMetaKitEvent::CacheLoaded {
                scope: payload.scope,
            }]),
        }
    }

    pub fn export_cache(&self, scope: CacheScope) -> ScriptMetaKitResult<CachePayload> {
        match scope {
            CacheScope::Catalog | CacheScope::All => {
                let data = if let Some(snapshot) = self.catalog_snapshot.as_ref() {
                    serde_json::to_value(snapshot)
                } else {
                    serde_json::to_value(ScriptMetaCatalogSnapshot {
                        source_revision: Uuid::new_v4(),
                        roots: self.root_snapshots.values().cloned().collect(),
                        all_items: Vec::new(),
                        file_items: Vec::new(),
                        candidate_cache: CandidateCache::empty(),
                        update_check_result: None,
                    })
                }
                .map_err(|error| ScriptMetaKitError::Cache(error.to_string()))?;
                Ok(CachePayload::new(CacheScope::Catalog, data))
            }
            CacheScope::FileList | CacheScope::Root => {
                let data = serde_json::to_value(&self.file_list_snapshots)
                    .map_err(|error| ScriptMetaKitError::Cache(error.to_string()))?;
                Ok(CachePayload::new(scope, data))
            }
        }
    }

    pub fn invalidate_cache(
        &mut self,
        scope: CacheScope,
        reason: CacheInvalidationReason,
    ) -> Vec<ScriptMetaKitEvent> {
        match scope {
            CacheScope::All => {
                self.catalog_snapshot = None;
                self.file_list_snapshots.clear();
            }
            CacheScope::Catalog => self.catalog_snapshot = None,
            CacheScope::FileList | CacheScope::Root => self.file_list_snapshots.clear(),
        }
        vec![ScriptMetaKitEvent::CacheInvalidated { scope, reason }]
    }

    fn selected_root_indices(&self, root_ids: &[RootId]) -> Vec<usize> {
        if root_ids.is_empty() {
            return (0..self.roots.len()).collect();
        }

        let requested: BTreeSet<_> = root_ids.iter().map(String::as_str).collect();
        self.roots
            .iter()
            .enumerate()
            .filter(|(_, root)| requested.contains(root.root_id.as_str()))
            .map(|(index, _)| index)
            .collect()
    }

    fn snapshots_for_roots(&self, root_ids: &[RootId]) -> Vec<RootSnapshot> {
        root_ids
            .iter()
            .filter_map(|root_id| self.root_snapshots.get(root_id).cloned())
            .collect()
    }

    fn file_list_snapshots_for_roots(&self, root_ids: &[RootId]) -> Vec<FileListSnapshot> {
        root_ids
            .iter()
            .filter_map(|root_id| self.file_list_snapshots.get(root_id).cloned())
            .collect()
    }

    fn watch_plan_with_delivery_options(&self, mut plan: WatchPlan) -> WatchPlan {
        plan.debounce_delay_millis = self.config.watcher.debounce_delay_millis;
        plan.max_delivery_delay_millis = self.config.watcher.max_delivery_delay_millis;
        plan.max_pending_paths = self.config.watcher.max_pending_paths;
        plan.supported_extensions = self.config.supported_extensions.clone();
        plan.skip_hidden_paths = self.config.scanner.skip_hidden;
        plan.skip_package_paths = self.config.scanner.skip_packages;
        plan
    }

    fn known_directory_paths(&self) -> BTreeSet<PathBuf> {
        let mut paths: BTreeSet<PathBuf> = self
            .roots
            .iter()
            .map(|root| normalize_path(&root.path))
            .collect();
        for snapshot in self.file_list_snapshots.values() {
            collect_directory_paths(snapshot.children.as_deref().unwrap_or_default(), &mut paths);
        }
        paths
    }

    fn merge_catalog_snapshot(
        &self,
        previous: Option<&ScriptMetaCatalogSnapshot>,
        refreshed: &ScriptMetaCatalogSnapshot,
    ) -> ScriptMetaCatalogSnapshot {
        let Some(previous) = previous else {
            return refreshed.clone();
        };

        let refreshed_root_ids: BTreeSet<_> = refreshed
            .roots
            .iter()
            .map(|root| root.root_id.as_str())
            .collect();
        let mut roots: Vec<_> = previous
            .roots
            .iter()
            .filter(|root| !refreshed_root_ids.contains(root.root_id.as_str()))
            .cloned()
            .collect();
        roots.extend(refreshed.roots.iter().cloned());
        let root_order: BTreeMap<_, _> = self
            .roots
            .iter()
            .enumerate()
            .map(|(index, root)| (root.root_id.as_str(), index))
            .collect();
        roots.sort_by_key(|root| {
            root_order
                .get(root.root_id.as_str())
                .copied()
                .unwrap_or(usize::MAX)
        });

        let mut records: Vec<_> = previous
            .candidate_cache
            .records
            .iter()
            .filter(|record| !refreshed_root_ids.contains(record.root_id.as_str()))
            .cloned()
            .collect();
        records.extend(refreshed.candidate_cache.records.iter().cloned());
        records.sort_by(|lhs, rhs| lhs.identity_path.cmp(&rhs.identity_path));

        let registered_roots: Vec<_> = self.roots.iter().collect();
        let candidate_cache = CandidateCache {
            schema_version: CandidateCache::CURRENT_SCHEMA_VERSION,
            built_at: now_timestamp_millis(),
            registered_roots: registered_root_signatures(&registered_roots),
            records,
        };
        let file_items = file_items_from_cache(&candidate_cache);
        let all_items = deduplicated_items(&file_items);
        let update_check_result = self.preserved_update_result(Some(previous), &all_items);

        ScriptMetaCatalogSnapshot {
            source_revision: refreshed.source_revision,
            roots,
            all_items,
            file_items,
            candidate_cache,
            update_check_result,
        }
    }

    fn preserved_update_result(
        &self,
        previous: Option<&ScriptMetaCatalogSnapshot>,
        current_items: &[ScriptMetaItem],
    ) -> Option<UpdateCheckResult> {
        if !self.config.cache.preserve_update_results {
            return None;
        }
        let previous = previous?;
        let previous_result = previous.update_check_result.as_ref()?;
        let previous_items_by_id: BTreeMap<_, _> = previous
            .all_items
            .iter()
            .map(|item| (item.item_id(), item))
            .collect();
        let preserved_item_ids: BTreeSet<_> = current_items
            .iter()
            .filter_map(|item| {
                let item_id = item.item_id();
                previous_items_by_id
                    .get(&item_id)
                    .is_some_and(|previous_item| *previous_item == item)
                    .then_some(item_id)
            })
            .collect();

        Some(UpdateCheckResult {
            checked_at: previous_result.checked_at,
            resolutions_by_item_id: previous_result
                .resolutions_by_item_id
                .iter()
                .filter(|(item_id, _)| preserved_item_ids.contains(*item_id))
                .map(|(item_id, resolution)| (item_id.clone(), resolution.clone()))
                .collect(),
            failures_by_item_id: previous_result
                .failures_by_item_id
                .iter()
                .filter(|(item_id, _)| preserved_item_ids.contains(*item_id))
                .map(|(item_id, failure)| (item_id.clone(), failure.clone()))
                .collect(),
            errors_by_item_id: previous_result
                .errors_by_item_id
                .iter()
                .filter(|(item_id, _)| preserved_item_ids.contains(*item_id))
                .map(|(item_id, error)| (item_id.clone(), error.clone()))
                .collect(),
            statuses_by_item_id: previous_result
                .statuses_by_item_id
                .iter()
                .filter(|(item_id, _)| preserved_item_ids.contains(*item_id))
                .map(|(item_id, status)| (item_id.clone(), *status))
                .collect(),
        })
    }
}

fn validate_config(config: &ScriptMetaKitConfig) -> ScriptMetaKitResult<()> {
    if config.app_id.trim().is_empty() {
        return Err(ScriptMetaKitError::InvalidConfig(
            "app_id must not be empty".to_string(),
        ));
    }
    if config.cache_namespace.trim().is_empty() {
        return Err(ScriptMetaKitError::InvalidConfig(
            "cache_namespace must not be empty".to_string(),
        ));
    }
    Ok(())
}

fn diff_file_list_snapshot(
    root_id: &RootId,
    previous: &FileListSnapshot,
    current: &FileListSnapshot,
) -> ScanChangeSummary {
    let mut previous_entries = BTreeMap::new();
    let mut current_entries = BTreeMap::new();
    collect_file_entries(
        previous.children.as_deref().unwrap_or_default(),
        &mut previous_entries,
    );
    collect_file_entries(
        current.children.as_deref().unwrap_or_default(),
        &mut current_entries,
    );

    let mut summary = ScanChangeSummary::default();
    for (path, current_entry) in &current_entries {
        match previous_entries.get(path) {
            Some(previous_entry) if file_entry_changed(previous_entry, current_entry) => {
                summary.modified_count += 1;
                summary.changes.push(FileEntryChange::from_entry(
                    root_id.clone(),
                    FileEntryChangeKind::Modified,
                    current_entry,
                ));
            }
            Some(_) => {}
            None => {
                summary.added_count += 1;
                summary.changes.push(FileEntryChange::from_entry(
                    root_id.clone(),
                    FileEntryChangeKind::Added,
                    current_entry,
                ));
            }
        }
    }

    for (path, previous_entry) in &previous_entries {
        if current_entries.contains_key(path) {
            continue;
        }
        summary.removed_count += 1;
        summary.changes.push(FileEntryChange::from_entry(
            root_id.clone(),
            FileEntryChangeKind::Removed,
            previous_entry,
        ));
    }

    summary
        .changes
        .sort_by(|lhs, rhs| lhs.resolved_path.cmp(&rhs.resolved_path));
    summary
}

fn collect_file_entries<'a>(
    entries: &'a [FileSystemEntry],
    output: &mut BTreeMap<PathBuf, &'a FileSystemEntry>,
) {
    for entry in entries {
        output.insert(entry.resolved_path.clone(), entry);
        collect_file_entries(&entry.children, output);
    }
}

fn collect_directory_paths(entries: &[FileSystemEntry], output: &mut BTreeSet<PathBuf>) {
    for entry in entries {
        if entry.is_directory {
            output.insert(entry.resolved_path.clone());
        }
        collect_directory_paths(&entry.children, output);
    }
}

fn file_entry_changed(lhs: &FileSystemEntry, rhs: &FileSystemEntry) -> bool {
    lhs.is_directory != rhs.is_directory
        || lhs.path_kind != rhs.path_kind
        || lhs.resolution_status != rhs.resolution_status
        || lhs.resolution_message.as_deref() != rhs.resolution_message.as_deref()
        || lhs.file_size != rhs.file_size
        || lhs.content_modified_at != rhs.content_modified_at
        || lhs.runtime_kind != rhs.runtime_kind
        || lhs.shebang.as_deref() != rhs.shebang.as_deref()
        || lhs.has_scriptmeta != rhs.has_scriptmeta
        || lhs.has_scriptmeta_edit_password != rhs.has_scriptmeta_edit_password
        || lhs.is_file_locked != rhs.is_file_locked
        || lhs.is_read_only != rhs.is_read_only
        || lhs.can_edit_scriptmeta != rhs.can_edit_scriptmeta
        || lhs.can_append_scriptmeta != rhs.can_append_scriptmeta
        || lhs.scriptmeta_edit_state != rhs.scriptmeta_edit_state
}

fn apply_metadata_capabilities_to_file_list_snapshots(
    snapshots: &mut [FileListSnapshot],
    records: &[CandidateRecord],
) {
    let capability_by_display_path: BTreeMap<_, _> = records
        .iter()
        .map(|record| {
            (
                (record.root_id.as_str(), record.file_path.as_path()),
                record,
            )
        })
        .collect();
    let capability_by_identity_path: BTreeMap<_, _> = records
        .iter()
        .map(|record| {
            (
                (record.root_id.as_str(), record.identity_path.as_path()),
                record,
            )
        })
        .collect();

    for snapshot in snapshots {
        if let Some(children) = snapshot.children.as_mut() {
            apply_metadata_capabilities_to_entries(
                snapshot.root.root_id.as_str(),
                children,
                &capability_by_display_path,
                &capability_by_identity_path,
            );
        }
    }
}

fn apply_metadata_capabilities_to_entries(
    root_id: &str,
    entries: &mut [FileSystemEntry],
    by_display_path: &BTreeMap<(&str, &std::path::Path), &CandidateRecord>,
    by_identity_path: &BTreeMap<(&str, &std::path::Path), &CandidateRecord>,
) {
    for entry in entries {
        if let Some(record) = by_display_path
            .get(&(root_id, entry.display_path.as_path()))
            .or_else(|| by_identity_path.get(&(root_id, entry.resolved_path.as_path())))
        {
            entry.has_scriptmeta = record.has_scriptmeta;
            entry.has_scriptmeta_edit_password = record.has_scriptmeta_edit_password;
            entry.is_file_locked = record.is_file_locked;
            entry.is_read_only = record.is_read_only;
            entry.can_edit_scriptmeta = record.can_edit_scriptmeta;
            entry.can_append_scriptmeta = record.can_append_scriptmeta;
            entry.scriptmeta_edit_state = record.scriptmeta_edit_state;
        }
        apply_metadata_capabilities_to_entries(
            root_id,
            &mut entry.children,
            by_display_path,
            by_identity_path,
        );
    }
}
