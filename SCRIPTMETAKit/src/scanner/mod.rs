mod file_list;
mod metadata_scan;
mod options;
mod path_resolution;
mod script_detection;

pub use file_list::{DirectoryScanOutput, FileSystemEntry, scan_file_list_root};
pub use metadata_scan::{
    CandidateCache, CandidateRecord, MetadataScanOutput, RegisteredRootSignature,
    scan_metadata_roots,
};
pub(crate) use metadata_scan::{
    deduplicated_items, file_items_from_cache, registered_root_signatures,
};
pub use options::{ExtensionPolicy, ScannerOptions};
pub use path_resolution::{PathKind, PathResolutionStatus};
pub use script_detection::{ScriptFileInfo, detect_script_file};
