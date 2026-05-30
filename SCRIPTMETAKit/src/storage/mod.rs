use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::Path,
};

use crate::{
    catalog::CacheScope,
    core::{ScriptMetaKitError, ScriptMetaKitResult},
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CacheSchema {
    pub schema_version: u32,
    pub package_version: String,
}

impl CacheSchema {
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;

    #[must_use]
    pub fn current() -> Self {
        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            package_version: crate::package_version().to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CachePayload {
    pub schema: CacheSchema,
    pub scope: CacheScope,
    pub data: Value,
}

impl CachePayload {
    #[must_use]
    pub fn new(scope: CacheScope, data: Value) -> Self {
        Self {
            schema: CacheSchema::current(),
            scope,
            data,
        }
    }

    #[must_use]
    pub fn is_current_schema(&self) -> bool {
        self.schema.schema_version == CacheSchema::CURRENT_SCHEMA_VERSION
    }
}

pub fn save_cache_payload(
    path: impl AsRef<Path>,
    payload: &CachePayload,
) -> ScriptMetaKitResult<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| ScriptMetaKitError::Io {
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;
    }

    let file = File::create(path).map_err(|error| ScriptMetaKitError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    serde_json::to_writer_pretty(BufWriter::new(file), payload)
        .map_err(|error| ScriptMetaKitError::Cache(error.to_string()))
}

pub fn load_cache_payload(path: impl AsRef<Path>) -> ScriptMetaKitResult<CachePayload> {
    let path = path.as_ref();
    let file = File::open(path).map_err(|error| ScriptMetaKitError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let payload: CachePayload = serde_json::from_reader(BufReader::new(file))
        .map_err(|error| ScriptMetaKitError::Cache(error.to_string()))?;
    if !payload.is_current_schema() {
        return Err(ScriptMetaKitError::Cache(format!(
            "unsupported cache schema version {}",
            payload.schema.schema_version
        )));
    }
    Ok(payload)
}
