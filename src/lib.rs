#![doc = "Core scanning, metadata parsing, update checking, and watch planning APIs for SCRIPTMETA."]
#![forbid(unsafe_code)]

pub mod catalog;
pub mod core;
pub mod engine;
pub mod resolver;
pub mod scanner;
pub mod storage;
pub mod watcher;

pub use catalog::{
    CacheInvalidationReason, CacheOptions, CachePolicy, CacheScope, DirectoryState,
    DirectoryStateMap, FileEntryChange, FileEntryChangeKind, FileListSnapshot, ProgressUpdate,
    RefreshPolicy, RefreshRequest, RootError, RootPriority, RootPurpose, RootRegistration,
    RootSnapshot, RootStatus, ScanChangeSummary, ScanMode, ScanRequest, ScanResult,
    ScriptMetaCatalogSnapshot, ScriptMetaKitConfig, ScriptMetaKitEvent, UpdateCheckOptions,
    UpdateCheckProgress, UpdateCheckProgressPhase, UpdateCheckRequest, UpdateCheckResult,
    UpdateFailure, UpdateStatus, WatcherOptions, path_based_root_id,
};
pub use core::{
    DistributionMetadata, DistributionResolution, ParserOptions, ScriptMetaEditCapability,
    ScriptMetaEditState, ScriptMetaItem, ScriptMetaKitError, ScriptMetaKitResult, ScriptMetadata,
    ScriptRuntimeKind, VersionOrdering, compare_versions, parse_distribution_metadata,
    parse_distribution_metadata_for_script, parse_script_metadata,
};
pub use engine::ScriptMetaKitEngine;
pub use resolver::{DistributionResolver, DistributionResolverOptions, UpdateResolver};
pub use scanner::{
    ExtensionPolicy, FileSystemEntry, PathKind, PathResolutionStatus, ScannerOptions,
};
pub use storage::{CachePayload, CacheSchema, load_cache_payload, save_cache_payload};
#[cfg(feature = "native-watch")]
pub use watcher::NativeWatcher;
pub use watcher::{
    LogicalWatchRoot, MonitorRootStrategy, OverflowPolicy, PhysicalWatchRoot, RawChangeBatch,
    RootChange, RootChangeBatch, WatchPlan, WatchPolicy,
};

pub type RootId = String;
pub type ItemId = String;
pub type TimestampMillis = u64;

/// Returns the Rust package name.
#[must_use]
pub fn package_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

/// Returns the Rust package version.
#[must_use]
pub fn package_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[must_use]
pub fn now_timestamp_millis() -> TimestampMillis {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{package_name, package_version};

    #[test]
    fn exposes_package_metadata() {
        assert_eq!(package_name(), "scriptmetakit");
        assert_eq!(package_version(), "0.1.0");
    }
}
