use std::{collections::BTreeSet, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExtensionPolicy {
    extensions: BTreeSet<String>,
}

impl ExtensionPolicy {
    #[must_use]
    pub fn new<I, S>(extensions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self {
            extensions: extensions
                .into_iter()
                .map(|extension| normalize_extension(extension.as_ref()))
                .filter(|extension| !extension.is_empty())
                .collect(),
        }
    }

    #[must_use]
    pub fn script_default() -> Self {
        Self::new([
            "js",
            "jsx",
            "jsxbin",
            "jsxinc",
            "scpt",
            "applescript",
            "jxa",
            "idjs",
            "psjs",
        ])
    }

    #[must_use]
    pub fn contains_path(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| self.contains_extension(extension))
    }

    #[must_use]
    pub fn contains_extension(&self, extension: &str) -> bool {
        let extension = extension.trim().trim_start_matches('.');
        if extension.is_empty() {
            return false;
        }

        if extension.bytes().any(|byte| byte.is_ascii_uppercase()) {
            let normalized = extension.to_ascii_lowercase();
            self.extensions.contains(normalized.as_str())
        } else {
            self.extensions.contains(extension)
        }
    }

    #[must_use]
    pub fn extensions(&self) -> &BTreeSet<String> {
        &self.extensions
    }
}

impl Default for ExtensionPolicy {
    fn default() -> Self {
        Self::script_default()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScannerOptions {
    pub max_depth: usize,
    pub max_nodes_per_root: usize,
    pub max_prefix_bytes: usize,
    pub skip_hidden: bool,
    pub skip_packages: bool,
    pub follow_symlinks: bool,
    pub resolve_macos_alias: bool,
    pub reuse_unchanged_records: bool,
    #[serde(default)]
    pub include_empty_directories: bool,
    pub scan_timeout_per_root_millis: Option<u64>,
}

impl Default for ScannerOptions {
    fn default() -> Self {
        Self {
            max_depth: 24,
            max_nodes_per_root: 20_000,
            max_prefix_bytes: 128 * 1024,
            skip_hidden: true,
            skip_packages: true,
            follow_symlinks: true,
            resolve_macos_alias: true,
            reuse_unchanged_records: true,
            include_empty_directories: false,
            scan_timeout_per_root_millis: None,
        }
    }
}

fn normalize_extension(extension: &str) -> String {
    extension
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase()
}
