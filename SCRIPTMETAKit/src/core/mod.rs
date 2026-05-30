mod distribution;
mod error;
mod metadata;
mod parser;
mod version;

pub use distribution::{DistributionMetadata, DistributionResolution};
pub use error::{ScriptMetaKitError, ScriptMetaKitResult};
pub use metadata::{
    ParserOptions, ScriptMetaEditCapability, ScriptMetaEditState, ScriptMetaItem, ScriptMetadata,
    ScriptRuntimeKind,
};
pub use parser::{
    parse_distribution_metadata, parse_distribution_metadata_for_script, parse_script_metadata,
};
pub use version::{VersionOrdering, compare_versions};
