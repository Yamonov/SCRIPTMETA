use std::{fs, path::Path};

#[cfg(target_os = "macos")]
use std::os::darwin::fs::MetadataExt;

use serde::{Deserialize, Serialize};

use crate::core::{ScriptMetaEditCapability, ScriptMetaEditState, ScriptRuntimeKind};

pub const SHEBANG_PROBE_BYTE_LIMIT: usize = 50;
pub const JSXBIN_PROBE_BYTE_LIMIT: usize = 8;
const JSXBIN_SIGNATURE: &[u8] = b"@JSXBIN";

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScriptFileInfo {
    pub runtime_kind: Option<ScriptRuntimeKind>,
    pub shebang: Option<String>,
}

#[must_use]
pub fn detect_script_file(path: &Path, prefix_text: Option<&str>) -> ScriptFileInfo {
    detect_script_file_from_bytes(path, prefix_text.map(str::as_bytes))
}

#[must_use]
pub fn detect_script_file_from_bytes(path: &Path, prefix_bytes: Option<&[u8]>) -> ScriptFileInfo {
    let is_jxa = has_jxa_osascript_shebang(prefix_bytes);
    let shebang = prefix_bytes.and_then(extract_shebang);
    let runtime_kind = runtime_kind_for_path(path, is_jxa);
    ScriptFileInfo {
        runtime_kind,
        shebang,
    }
}

#[must_use]
pub fn needs_shebang_probe(path: &Path) -> bool {
    extension_matches(path, &["js", "applescript"])
}

#[must_use]
pub fn needs_script_header_probe(path: &Path) -> bool {
    needs_shebang_probe(path) || needs_jsxbin_probe(path)
}

#[must_use]
pub fn script_header_probe_byte_limit(path: &Path) -> usize {
    if needs_shebang_probe(path) {
        SHEBANG_PROBE_BYTE_LIMIT
    } else if needs_jsxbin_probe(path) {
        JSXBIN_PROBE_BYTE_LIMIT
    } else {
        0
    }
}

#[must_use]
pub fn has_scriptmeta_tag(prefix_text: &str) -> bool {
    prefix_text.contains("SCRIPTMETA-BEGIN")
}

#[must_use]
pub fn scriptmeta_edit_capability_from_file_list_probe(
    path: &Path,
    prefix_bytes: Option<&[u8]>,
) -> ScriptMetaEditCapability {
    scriptmeta_edit_capability(path, prefix_bytes, false, false, false)
}

#[must_use]
pub fn scriptmeta_edit_capability_from_metadata(
    path: &Path,
    prefix_bytes: Option<&[u8]>,
    has_scriptmeta: bool,
    has_scriptmeta_edit_password: bool,
) -> ScriptMetaEditCapability {
    scriptmeta_edit_capability(
        path,
        prefix_bytes,
        true,
        has_scriptmeta,
        has_scriptmeta_edit_password,
    )
}

#[must_use]
pub fn scriptmeta_edit_capability_from_cached_metadata(
    path: &Path,
    previous_state: ScriptMetaEditState,
    has_scriptmeta: bool,
    has_scriptmeta_edit_password: bool,
) -> ScriptMetaEditCapability {
    if previous_state == ScriptMetaEditState::Obfuscated {
        let file_state = file_write_state(path);
        return ScriptMetaEditCapability::from_state(
            ScriptMetaEditState::Obfuscated,
            has_scriptmeta,
            has_scriptmeta_edit_password,
            file_state.is_file_locked,
            file_state.is_read_only,
        );
    }

    scriptmeta_edit_capability(
        path,
        None,
        true,
        has_scriptmeta,
        has_scriptmeta_edit_password,
    )
}

fn runtime_kind_for_path(path: &Path, is_jxa: bool) -> Option<ScriptRuntimeKind> {
    let extension = path.extension().and_then(|extension| extension.to_str())?;
    let extension = extension.trim().trim_start_matches('.');
    if extension.eq_ignore_ascii_case("scpt") {
        Some(ScriptRuntimeKind::AppleScript)
    } else if extension.eq_ignore_ascii_case("applescript") {
        Some(if is_jxa {
            ScriptRuntimeKind::JavaScriptForAutomation
        } else {
            ScriptRuntimeKind::AppleScript
        })
    } else if extension.eq_ignore_ascii_case("jxa")
        || (is_jxa && extension.eq_ignore_ascii_case("js"))
    {
        Some(ScriptRuntimeKind::JavaScriptForAutomation)
    } else if ["js", "jsx", "jsxbin", "jsxinc"]
        .iter()
        .any(|candidate| extension.eq_ignore_ascii_case(candidate))
    {
        Some(ScriptRuntimeKind::AdobeJavaScript)
    } else if ["idjs", "psjs"]
        .iter()
        .any(|candidate| extension.eq_ignore_ascii_case(candidate))
    {
        Some(ScriptRuntimeKind::AdobeUxp)
    } else {
        None
    }
}

fn has_jxa_osascript_shebang(prefix_bytes: Option<&[u8]>) -> bool {
    let Some(shebang) = prefix_bytes.and_then(extract_shebang) else {
        return false;
    };
    let Some(command_line) = shebang.strip_prefix("#!") else {
        return false;
    };
    let tokens = command_line.split_whitespace().collect::<Vec<_>>();
    let Some((program, args)) = tokens.split_first() else {
        return false;
    };

    if is_osascript_program(program) {
        return osascript_args_request_jxa(args);
    }
    if is_env_program(program)
        && let Some(index) = args.iter().position(|token| is_osascript_program(token))
    {
        return osascript_args_request_jxa(&args[index + 1..]);
    }
    false
}

fn is_osascript_program(token: &str) -> bool {
    token.rsplit('/').next() == Some("osascript")
}

fn is_env_program(token: &str) -> bool {
    token.rsplit('/').next() == Some("env")
}

fn osascript_args_request_jxa(args: &[&str]) -> bool {
    args.windows(2).any(|pair| {
        matches!(pair[0], "-l" | "-lang" | "-language")
            && pair[1].eq_ignore_ascii_case("JavaScript")
    })
}

fn extract_shebang(bytes: &[u8]) -> Option<String> {
    if !bytes.starts_with(b"#!") {
        return None;
    }

    let end = bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .unwrap_or(bytes.len());
    let line = bytes[..end].strip_suffix(b"\r").unwrap_or(&bytes[..end]);
    let shebang = String::from_utf8_lossy(line).trim().to_string();
    (!shebang.is_empty()).then_some(shebang)
}

fn extension_matches(path: &Path, candidates: &[&str]) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.trim().trim_start_matches('.'))
        .is_some_and(|extension| {
            candidates
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
        })
}

fn scriptmeta_edit_capability(
    path: &Path,
    prefix_bytes: Option<&[u8]>,
    metadata_state_known: bool,
    has_scriptmeta: bool,
    has_scriptmeta_edit_password: bool,
) -> ScriptMetaEditCapability {
    let file_state = file_write_state(path);
    let state = if is_jsxbin_obfuscated_script(path, prefix_bytes) {
        ScriptMetaEditState::Obfuscated
    } else if !supports_inline_scriptmeta(path) {
        ScriptMetaEditState::Unsupported
    } else if !metadata_state_known {
        ScriptMetaEditState::Unknown
    } else if file_state.is_read_only {
        ScriptMetaEditState::ReadOnly
    } else if has_scriptmeta {
        ScriptMetaEditState::Editable
    } else {
        ScriptMetaEditState::Appendable
    };

    ScriptMetaEditCapability::from_state(
        state,
        has_scriptmeta,
        has_scriptmeta_edit_password,
        file_state.is_file_locked,
        file_state.is_read_only,
    )
}

fn supports_inline_scriptmeta(path: &Path) -> bool {
    extension_matches(
        path,
        &[
            "js",
            "jsx",
            "jsxinc",
            "scpt",
            "applescript",
            "idjs",
            "jxa",
            "psjs",
        ],
    )
}

fn needs_jsxbin_probe(path: &Path) -> bool {
    extension_matches(path, &["js", "jsx", "jsxbin"])
}

fn is_jsxbin_obfuscated_script(path: &Path, prefix_bytes: Option<&[u8]>) -> bool {
    extension_matches(path, &["jsxbin"])
        || (needs_jsxbin_probe(path)
            && prefix_bytes.is_some_and(|bytes| bytes.starts_with(JSXBIN_SIGNATURE)))
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct FileWriteState {
    is_file_locked: bool,
    is_read_only: bool,
}

fn file_write_state(path: &Path) -> FileWriteState {
    let file_metadata = fs::metadata(path).ok();
    let is_file_locked = file_metadata
        .as_ref()
        .is_some_and(is_macos_locked_or_append_only);
    let file_writable = file_metadata
        .as_ref()
        .map(|metadata| !metadata.permissions().readonly())
        .unwrap_or(false);
    let parent_writable = path
        .parent()
        .and_then(|parent| fs::metadata(parent).ok())
        .map(|metadata| !metadata.permissions().readonly())
        .unwrap_or(false);

    FileWriteState {
        is_file_locked,
        is_read_only: is_file_locked || !file_writable || !parent_writable,
    }
}

#[cfg(target_os = "macos")]
fn is_macos_locked_or_append_only(metadata: &fs::Metadata) -> bool {
    const UF_IMMUTABLE: u32 = 0x0000_0002;
    const UF_APPEND: u32 = 0x0000_0004;
    const SF_IMMUTABLE: u32 = 0x0002_0000;
    const SF_APPEND: u32 = 0x0004_0000;
    let flags = metadata.st_flags();
    flags & (UF_IMMUTABLE | UF_APPEND | SF_IMMUTABLE | SF_APPEND) != 0
}

#[cfg(not(target_os = "macos"))]
fn is_macos_locked_or_append_only(_metadata: &fs::Metadata) -> bool {
    false
}
