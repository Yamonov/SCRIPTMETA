#[cfg(feature = "native-watch")]
mod native;
mod plan;
mod policy;

#[cfg(feature = "native-watch")]
pub use native::NativeWatcher;
pub use plan::{
    DEFAULT_DEBOUNCE_DELAY_MILLIS, DEFAULT_MAX_DELIVERY_DELAY_MILLIS, DEFAULT_MAX_PENDING_PATHS,
    LogicalWatchRoot, PhysicalWatchRoot, RawChangeBatch, RootChange, RootChangeBatch, WatchPlan,
    build_watch_plan,
};
pub use policy::{MonitorRootStrategy, OverflowPolicy, WatchPolicy};

pub(crate) use plan::{ChangeRoutingOptions, normalize_path, route_change_batch};
