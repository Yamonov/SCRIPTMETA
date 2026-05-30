use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        mpsc::{self, Receiver, RecvTimeoutError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use notify::{
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::{CreateKind, ModifyKind, RemoveKind},
};

use crate::{
    core::{ScriptMetaKitError, ScriptMetaKitResult},
    scanner::ExtensionPolicy,
    watcher::{RawChangeBatch, WatchPlan},
};

pub struct NativeWatcher {
    watcher: Option<RecommendedWatcher>,
    receiver: Receiver<RawChangeBatch>,
    worker: Option<JoinHandle<()>>,
}

impl NativeWatcher {
    pub fn start(plan: &WatchPlan) -> ScriptMetaKitResult<Self> {
        Self::start_with_notifier(plan, None)
    }

    pub fn start_with_notifier(
        plan: &WatchPlan,
        notifier: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    ) -> ScriptMetaKitResult<Self> {
        let (event_sender, event_receiver) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(event_sender, Config::default())
            .map_err(|error| ScriptMetaKitError::InvalidConfig(error.to_string()))?;

        for root in &plan.physical_roots {
            watcher
                .watch(&root.path, RecursiveMode::Recursive)
                .map_err(|error| ScriptMetaKitError::Io {
                    path: root.path.clone(),
                    message: error.to_string(),
                })?;
        }

        let debounce_delay = Duration::from_millis(plan.debounce_delay_millis);
        let max_delivery_delay = (plan.max_delivery_delay_millis > 0)
            .then(|| Duration::from_millis(plan.max_delivery_delay_millis));
        let max_pending_paths = plan.max_pending_paths;
        let overflow_paths: Vec<_> = plan
            .physical_roots
            .iter()
            .map(|root| root.path.clone())
            .collect();
        let watch_roots = overflow_paths.clone();
        let supported_extensions = plan.supported_extensions.clone();
        let skip_hidden_paths = plan.skip_hidden_paths;
        let skip_package_paths = plan.skip_package_paths;

        let (batch_sender, batch_receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut pending_paths = Vec::new();
            let mut pending_overflowed = false;
            let mut first_event_at = None;
            let mut last_event_at = None;

            loop {
                let receive_result = match next_flush_timeout(
                    first_event_at,
                    last_event_at,
                    debounce_delay,
                    max_delivery_delay,
                ) {
                    Some(timeout) => event_receiver.recv_timeout(timeout),
                    None => event_receiver
                        .recv()
                        .map_err(|_| RecvTimeoutError::Disconnected),
                };

                match receive_result {
                    Ok(event_result) => {
                        let (pending_changed, flush_now) = append_event_to_pending(
                            event_result,
                            &mut pending_paths,
                            &mut pending_overflowed,
                            NativeEventFilter {
                                overflow_paths: &overflow_paths,
                                max_pending_paths,
                                watch_roots: &watch_roots,
                                supported_extensions: &supported_extensions,
                                skip_hidden_paths,
                                skip_package_paths,
                            },
                        );
                        if !pending_changed {
                            continue;
                        }

                        let now = Instant::now();
                        if first_event_at.is_none() {
                            first_event_at = Some(now);
                        }
                        last_event_at = Some(now);

                        if (flush_now
                            || should_flush_after_event(
                                first_event_at,
                                debounce_delay,
                                max_delivery_delay,
                            ))
                            && !flush_pending_batch(
                                &batch_sender,
                                notifier.as_ref(),
                                &mut pending_paths,
                                &mut pending_overflowed,
                                &mut first_event_at,
                                &mut last_event_at,
                            )
                        {
                            break;
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        if !flush_pending_batch(
                            &batch_sender,
                            notifier.as_ref(),
                            &mut pending_paths,
                            &mut pending_overflowed,
                            &mut first_event_at,
                            &mut last_event_at,
                        ) {
                            break;
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        let _ = flush_pending_batch(
                            &batch_sender,
                            notifier.as_ref(),
                            &mut pending_paths,
                            &mut pending_overflowed,
                            &mut first_event_at,
                            &mut last_event_at,
                        );
                        break;
                    }
                }
            }
        });

        Ok(Self {
            watcher: Some(watcher),
            receiver: batch_receiver,
            worker: Some(worker),
        })
    }

    pub fn try_recv(&self) -> Option<RawChangeBatch> {
        self.receiver.try_recv().ok()
    }
}

impl Drop for NativeWatcher {
    fn drop(&mut self) {
        self.watcher.take();
        if let Some(worker) = self.worker.take()
            && worker.thread().id() != thread::current().id()
        {
            let _ = worker.join();
        }
    }
}

fn append_event_to_pending(
    event_result: notify::Result<Event>,
    pending_paths: &mut Vec<PathBuf>,
    pending_overflowed: &mut bool,
    filter: NativeEventFilter<'_>,
) -> (bool, bool) {
    match event_result {
        Ok(event) => {
            let mut paths = filtered_event_paths(&event, &filter);
            if paths.is_empty() {
                return (false, false);
            }

            pending_paths.append(&mut paths);
            pending_paths.sort();
            pending_paths.dedup();

            if pending_exceeded_max(pending_paths, filter.max_pending_paths) {
                mark_pending_overflowed(pending_paths, pending_overflowed, filter.overflow_paths);
                return (true, true);
            }

            (true, false)
        }
        Err(_) => {
            mark_pending_overflowed(pending_paths, pending_overflowed, filter.overflow_paths);
            (true, false)
        }
    }
}

struct NativeEventFilter<'a> {
    overflow_paths: &'a [PathBuf],
    max_pending_paths: usize,
    watch_roots: &'a [PathBuf],
    supported_extensions: &'a ExtensionPolicy,
    skip_hidden_paths: bool,
    skip_package_paths: bool,
}

fn filtered_event_paths(event: &Event, filter: &NativeEventFilter<'_>) -> Vec<PathBuf> {
    if matches!(event.kind, EventKind::Access(_)) {
        return Vec::new();
    }

    event
        .paths
        .iter()
        .filter(|path| {
            should_keep_event_path(
                path,
                event.kind,
                filter.watch_roots,
                filter.supported_extensions,
                filter.skip_hidden_paths,
                filter.skip_package_paths,
            )
        })
        .cloned()
        .collect()
}

fn should_keep_event_path(
    path: &Path,
    event_kind: EventKind,
    watch_roots: &[PathBuf],
    supported_extensions: &ExtensionPolicy,
    skip_hidden_paths: bool,
    skip_package_paths: bool,
) -> bool {
    if skip_hidden_paths && path_has_hidden_component(path, watch_roots) {
        return false;
    }

    if skip_package_paths && path_has_package_component(path, watch_roots) {
        return false;
    }

    if supported_extensions.contains_path(path) {
        return true;
    }

    if !event_kind_may_change_directory_tree(event_kind) {
        return false;
    }

    event_kind_identifies_folder(event_kind)
        || path_is_existing_directory(path)
        || path.extension().is_none()
}

fn event_kind_may_change_directory_tree(event_kind: EventKind) -> bool {
    matches!(
        event_kind,
        EventKind::Any
            | EventKind::Other
            | EventKind::Create(CreateKind::Any | CreateKind::Folder | CreateKind::Other)
            | EventKind::Remove(RemoveKind::Any | RemoveKind::Folder | RemoveKind::Other)
            | EventKind::Modify(ModifyKind::Any | ModifyKind::Name(_) | ModifyKind::Other)
    )
}

fn event_kind_identifies_folder(event_kind: EventKind) -> bool {
    matches!(
        event_kind,
        EventKind::Create(CreateKind::Folder) | EventKind::Remove(RemoveKind::Folder)
    )
}

fn path_has_hidden_component(path: &Path, watch_roots: &[PathBuf]) -> bool {
    relative_event_path(path, watch_roots)
        .components()
        .any(|component| component.as_os_str().as_encoded_bytes().starts_with(b"."))
}

fn path_has_package_component(path: &Path, watch_roots: &[PathBuf]) -> bool {
    relative_event_path(path, watch_roots)
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

fn relative_event_path<'a>(path: &'a Path, watch_roots: &[PathBuf]) -> &'a Path {
    watch_roots
        .iter()
        .find_map(|root| path.strip_prefix(root).ok())
        .unwrap_or(path)
}

fn path_is_existing_directory(path: &Path) -> bool {
    path.metadata().is_ok_and(|metadata| metadata.is_dir())
}

fn pending_exceeded_max(pending_paths: &[PathBuf], max_pending_paths: usize) -> bool {
    max_pending_paths > 0 && pending_paths.len() > max_pending_paths
}

fn mark_pending_overflowed(
    pending_paths: &mut Vec<PathBuf>,
    pending_overflowed: &mut bool,
    overflow_paths: &[PathBuf],
) {
    pending_paths.clear();
    pending_paths.extend_from_slice(overflow_paths);
    pending_paths.sort();
    pending_paths.dedup();
    *pending_overflowed = true;
}

fn should_flush_after_event(
    first_event_at: Option<Instant>,
    debounce_delay: Duration,
    max_delivery_delay: Option<Duration>,
) -> bool {
    debounce_delay.is_zero()
        || first_event_at.is_some_and(|first_event_at| {
            max_delivery_delay.is_some_and(|delay| first_event_at.elapsed() >= delay)
        })
}

fn next_flush_timeout(
    first_event_at: Option<Instant>,
    last_event_at: Option<Instant>,
    debounce_delay: Duration,
    max_delivery_delay: Option<Duration>,
) -> Option<Duration> {
    let (Some(first_event_at), Some(last_event_at)) = (first_event_at, last_event_at) else {
        return None;
    };

    let quiet_deadline = last_event_at + debounce_delay;
    let deadline = max_delivery_delay
        .map(|delay| (first_event_at + delay).min(quiet_deadline))
        .unwrap_or(quiet_deadline);
    let now = Instant::now();
    Some(deadline.saturating_duration_since(now))
}

fn flush_pending_batch(
    batch_sender: &mpsc::Sender<RawChangeBatch>,
    notifier: Option<&Arc<dyn Fn() + Send + Sync + 'static>>,
    pending_paths: &mut Vec<PathBuf>,
    pending_overflowed: &mut bool,
    first_event_at: &mut Option<Instant>,
    last_event_at: &mut Option<Instant>,
) -> bool {
    if pending_paths.is_empty() && !*pending_overflowed {
        *first_event_at = None;
        *last_event_at = None;
        return true;
    }

    let batch = RawChangeBatch {
        paths: std::mem::take(pending_paths),
        overflowed: *pending_overflowed,
    };
    *pending_overflowed = false;
    *first_event_at = None;
    *last_event_at = None;

    if batch_sender.send(batch).is_err() {
        return false;
    }

    if let Some(notifier) = notifier {
        notifier();
    }
    true
}
