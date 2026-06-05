use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::Path,
};

use crate::{
    catalog::{CacheScope, RootRegistration, ScriptMetaKitConfig},
    core::{ScriptMetaKitError, ScriptMetaKitResult},
    now_timestamp_millis,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CacheSchema {
    pub schema_version: u32,
    #[serde(default)]
    pub min_supported_schema_version: u32,
    pub package_version: String,
    #[serde(default)]
    pub app_id: Option<String>,
    #[serde(default)]
    pub cache_namespace: Option<String>,
    #[serde(default)]
    pub created_at_millis: u64,
}

impl CacheSchema {
    pub const CURRENT_SCHEMA_VERSION: u32 = 2;
    pub const MIN_SUPPORTED_SCHEMA_VERSION: u32 = 1;

    #[must_use]
    pub fn current() -> Self {
        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            min_supported_schema_version: Self::MIN_SUPPORTED_SCHEMA_VERSION,
            package_version: crate::package_version().to_string(),
            app_id: None,
            cache_namespace: None,
            created_at_millis: now_timestamp_millis(),
        }
    }

    #[must_use]
    pub fn current_for_config(config: &ScriptMetaKitConfig) -> Self {
        Self {
            app_id: Some(config.app_id.clone()),
            cache_namespace: Some(config.cache_namespace.clone()),
            ..Self::current()
        }
    }

    #[must_use]
    pub fn is_supported(&self) -> bool {
        (Self::MIN_SUPPORTED_SCHEMA_VERSION..=Self::CURRENT_SCHEMA_VERSION)
            .contains(&self.schema_version)
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
    pub fn new_for_config(scope: CacheScope, data: Value, config: &ScriptMetaKitConfig) -> Self {
        Self {
            schema: CacheSchema::current_for_config(config),
            scope,
            data,
        }
    }

    #[must_use]
    pub fn is_supported_schema(&self) -> bool {
        self.schema.is_supported()
    }

    pub fn migrate(mut self) -> ScriptMetaKitResult<Self> {
        if !self.is_supported_schema() {
            return Err(ScriptMetaKitError::Cache(format!(
                "unsupported cache schema version {}",
                self.schema.schema_version
            )));
        }
        if self.schema.schema_version < CacheSchema::CURRENT_SCHEMA_VERSION {
            self.schema.schema_version = CacheSchema::CURRENT_SCHEMA_VERSION;
            self.schema.min_supported_schema_version = CacheSchema::MIN_SUPPORTED_SCHEMA_VERSION;
            if self.schema.created_at_millis == 0 {
                self.schema.created_at_millis = now_timestamp_millis();
            }
        }
        Ok(self)
    }

    pub fn validate_for_config(&self, config: &ScriptMetaKitConfig) -> ScriptMetaKitResult<()> {
        if let Some(app_id) = self.schema.app_id.as_deref()
            && app_id != config.app_id
        {
            return Err(ScriptMetaKitError::Cache(format!(
                "cache app_id `{app_id}` does not match `{}`",
                config.app_id
            )));
        }
        if let Some(namespace) = self.schema.cache_namespace.as_deref()
            && namespace != config.cache_namespace
        {
            return Err(ScriptMetaKitError::Cache(format!(
                "cache namespace `{namespace}` does not match `{}`",
                config.cache_namespace
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CacheRootSignature {
    pub root_id: String,
    pub path: String,
}

#[must_use]
pub fn cache_root_signatures(roots: &[RootRegistration]) -> Vec<CacheRootSignature> {
    let mut signatures = roots
        .iter()
        .map(|root| CacheRootSignature {
            root_id: root.root_id.clone(),
            path: root.path.to_string_lossy().into_owned(),
        })
        .collect::<Vec<_>>();
    signatures.sort_by(|lhs, rhs| lhs.root_id.cmp(&rhs.root_id));
    signatures
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
    payload.migrate()
}
