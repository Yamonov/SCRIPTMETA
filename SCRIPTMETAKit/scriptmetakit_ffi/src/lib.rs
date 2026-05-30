#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(feature = "native-watch")]
use std::sync::Arc;
use std::{
    collections::BTreeMap,
    ffi::c_void,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    ptr, slice, str,
};

use scriptmetakit::{
    CachePolicy, DistributionResolution, FileEntryChange, FileEntryChangeKind, FileListSnapshot,
    FileSystemEntry, RefreshPolicy, RootPriority, RootPurpose, RootRegistration, RootSnapshot,
    RootStatus, ScanChangeSummary, ScanMode, ScanRequest, ScanResult, ScriptMetaItem,
    ScriptMetaKitConfig, ScriptMetaKitEngine, ScriptRuntimeKind, UpdateCheckProgress,
    UpdateCheckProgressPhase, UpdateCheckRequest, UpdateCheckResult, UpdateFailure, UpdateStatus,
    WatchPolicy,
};
use url::Url;

#[cfg(feature = "native-watch")]
use scriptmetakit::{NativeWatcher, RefreshRequest};

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SmkStatus {
    Ok = 0,
    NullArgument = 1,
    InvalidUtf8 = 2,
    InvalidArgument = 3,
    EngineError = 4,
    Panic = 5,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SmkUtf8Slice {
    pub ptr: *const u8,
    pub len: usize,
}

impl SmkUtf8Slice {
    const fn empty() -> Self {
        Self {
            ptr: ptr::null(),
            len: 0,
        }
    }
}

impl Default for SmkUtf8Slice {
    fn default() -> Self {
        Self::empty()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkRootSnapshot {
    pub root_id: SmkUtf8Slice,
    pub path: SmkUtf8Slice,
    pub status: SmkUtf8Slice,
    pub is_dirty: u8,
    pub has_last_loaded_at: u8,
    pub last_loaded_at: u64,
    pub has_last_event_at: u8,
    pub last_event_at: u64,
    pub item_count: usize,
    pub error_code: SmkUtf8Slice,
    pub error_message: SmkUtf8Slice,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkFileEntry {
    pub display_path: SmkUtf8Slice,
    pub resolved_path: SmkUtf8Slice,
    pub path_kind: SmkUtf8Slice,
    pub resolution_status: SmkUtf8Slice,
    pub resolution_message: SmkUtf8Slice,
    pub is_directory: u8,
    pub has_file_size: u8,
    pub file_size: u64,
    pub has_content_modified_at: u8,
    pub content_modified_at: u64,
    pub runtime_kind: SmkUtf8Slice,
    pub shebang: SmkUtf8Slice,
    pub has_scriptmeta: u8,
    pub has_scriptmeta_edit_password: u8,
    pub is_file_locked: u8,
    pub is_read_only: u8,
    pub can_edit_scriptmeta: u8,
    pub can_append_scriptmeta: u8,
    pub scriptmeta_edit_state: SmkUtf8Slice,
    pub first_child_index: usize,
    pub child_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkFileListSnapshot {
    pub root_index: usize,
    pub first_child_index: usize,
    pub child_count: usize,
    pub truncated: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkScriptItem {
    pub root_id: SmkUtf8Slice,
    pub file_path: SmkUtf8Slice,
    pub identity_path: SmkUtf8Slice,
    pub runtime_kind: SmkUtf8Slice,
    pub shebang: SmkUtf8Slice,
    pub script_id: SmkUtf8Slice,
    pub version: SmkUtf8Slice,
    pub name: SmkUtf8Slice,
    pub description: SmkUtf8Slice,
    pub target_app: SmkUtf8Slice,
    pub meta_url: SmkUtf8Slice,
    pub author: SmkUtf8Slice,
    pub release_date: SmkUtf8Slice,
    pub edit_password_sha256: SmkUtf8Slice,
    pub has_scriptmeta: u8,
    pub has_scriptmeta_edit_password: u8,
    pub is_file_locked: u8,
    pub is_read_only: u8,
    pub can_edit_scriptmeta: u8,
    pub can_append_scriptmeta: u8,
    pub scriptmeta_edit_state: SmkUtf8Slice,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkUpdateCheckInfo {
    pub has_update_check: u8,
    pub checked_at: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkUpdateStatusEntry {
    pub item_id: SmkUtf8Slice,
    pub status: SmkUtf8Slice,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkDistributionResolutionEntry {
    pub item_id: SmkUtf8Slice,
    pub latest_version: SmkUtf8Slice,
    pub latest_page_url: SmkUtf8Slice,
    pub final_page_url: SmkUtf8Slice,
    pub first_latest_url_history_index: usize,
    pub latest_url_history_count: usize,
    pub checked_at: u64,
    pub is_unresolved: u8,
    pub note: SmkUtf8Slice,
    pub has_redirect_count: u8,
    pub redirect_count: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkUpdateFailureEntry {
    pub item_id: SmkUtf8Slice,
    pub code: SmkUtf8Slice,
    pub message: SmkUtf8Slice,
    pub file_path: SmkUtf8Slice,
    pub script_id: SmkUtf8Slice,
    pub current_version: SmkUtf8Slice,
    pub meta_url: SmkUtf8Slice,
    pub source_url: SmkUtf8Slice,
    pub checked_at: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkUpdateErrorEntry {
    pub item_id: SmkUtf8Slice,
    pub message: SmkUtf8Slice,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkUpdateProgress {
    pub completed_items: usize,
    pub total_items: usize,
    pub item_id: SmkUtf8Slice,
    pub script_id: SmkUtf8Slice,
    pub phase: SmkUtf8Slice,
    pub message: SmkUtf8Slice,
}

pub type SmkUpdateProgressCallback =
    Option<extern "C" fn(progress: *const SmkUpdateProgress, context: *mut c_void)>;

pub type SmkWatchNotificationCallback = Option<extern "C" fn(context: *mut c_void)>;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkScanChangeInfo {
    pub has_change_summary: u8,
    pub added_count: usize,
    pub removed_count: usize,
    pub modified_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkFileEntryChange {
    pub root_id: SmkUtf8Slice,
    pub kind: SmkUtf8Slice,
    pub display_path: SmkUtf8Slice,
    pub resolved_path: SmkUtf8Slice,
    pub path_kind: SmkUtf8Slice,
    pub resolution_status: SmkUtf8Slice,
    pub resolution_message: SmkUtf8Slice,
    pub is_directory: u8,
    pub has_file_size: u8,
    pub file_size: u64,
    pub has_content_modified_at: u8,
    pub content_modified_at: u64,
    pub runtime_kind: SmkUtf8Slice,
    pub shebang: SmkUtf8Slice,
    pub has_scriptmeta: u8,
    pub has_scriptmeta_edit_password: u8,
    pub is_file_locked: u8,
    pub is_read_only: u8,
    pub can_edit_scriptmeta: u8,
    pub can_append_scriptmeta: u8,
    pub scriptmeta_edit_state: SmkUtf8Slice,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkRootSnapshotSlice {
    pub ptr: *const SmkRootSnapshot,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkFileListSnapshotSlice {
    pub ptr: *const SmkFileListSnapshot,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkFileEntrySlice {
    pub ptr: *const SmkFileEntry,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkScriptItemSlice {
    pub ptr: *const SmkScriptItem,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkUpdateStatusEntrySlice {
    pub ptr: *const SmkUpdateStatusEntry,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkDistributionResolutionEntrySlice {
    pub ptr: *const SmkDistributionResolutionEntry,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkUpdateFailureEntrySlice {
    pub ptr: *const SmkUpdateFailureEntry,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkUpdateErrorEntrySlice {
    pub ptr: *const SmkUpdateErrorEntry,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkUtf8SliceSlice {
    pub ptr: *const SmkUtf8Slice,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SmkFileEntryChangeSlice {
    pub ptr: *const SmkFileEntryChange,
    pub len: usize,
}

pub struct SmkEngine {
    engine: ScriptMetaKitEngine,
    last_error: Vec<u8>,
    #[cfg(feature = "native-watch")]
    watcher: Option<NativeWatcher>,
}

pub struct SmkScanResult {
    string_storage: Vec<u8>,
    roots: Vec<SmkRootSnapshot>,
    file_lists: Vec<SmkFileListSnapshot>,
    file_entries: Vec<SmkFileEntry>,
    items: Vec<SmkScriptItem>,
    file_items: Vec<SmkScriptItem>,
    update_info: SmkUpdateCheckInfo,
    update_statuses: Vec<SmkUpdateStatusEntry>,
    update_resolutions: Vec<SmkDistributionResolutionEntry>,
    update_failures: Vec<SmkUpdateFailureEntry>,
    update_errors: Vec<SmkUpdateErrorEntry>,
    latest_url_history_urls: Vec<SmkUtf8Slice>,
    change_info: SmkScanChangeInfo,
    file_entry_changes: Vec<SmkFileEntryChange>,
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `out_engine` must be a valid, writable pointer to receive the new opaque
/// engine handle. The returned handle must be released with `smk_engine_free`.
pub unsafe extern "C" fn smk_engine_create_default(out_engine: *mut *mut SmkEngine) -> SmkStatus {
    ffi_guard(|| {
        let out_engine = out_mut(out_engine)?;
        let mut config = ScriptMetaKitConfig::default();
        config.watcher.watch_policy = WatchPolicy::AllRegistered;
        let engine = ScriptMetaKitEngine::new(config)
            .map_err(|error| (SmkStatus::EngineError, error.to_string()))?;
        let handle = Box::new(SmkEngine {
            engine,
            last_error: Vec::new(),
            #[cfg(feature = "native-watch")]
            watcher: None,
        });
        *out_engine = Box::into_raw(handle);
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `engine` must be null or a live handle returned by `smk_engine_create_default`.
/// Each non-null handle must be freed at most once.
pub unsafe extern "C" fn smk_engine_free(engine: *mut SmkEngine) {
    if !engine.is_null() {
        // SAFETY: `engine` must be a pointer returned by `smk_engine_create_default`.
        unsafe {
            drop(Box::from_raw(engine));
        }
    }
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `engine` must be a live engine handle. `out_message` must be a valid,
/// writable pointer. The returned slice is borrowed from `engine` and remains
/// valid only until the next mutable call using that handle or until the handle
/// is freed.
pub unsafe extern "C" fn smk_engine_last_error(
    engine: *const SmkEngine,
    out_message: *mut SmkUtf8Slice,
) -> SmkStatus {
    ffi_guard(|| {
        let engine = engine_ref(engine)?;
        let out_message = out_mut(out_message)?;
        *out_message = borrowed_slice(&engine.last_error);
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `engine` must be a live engine handle returned by
/// `smk_engine_create_default`.
pub unsafe extern "C" fn smk_engine_set_resolve_macos_alias(
    engine: *mut SmkEngine,
    enabled: u8,
) -> SmkStatus {
    let status = ffi_guard(|| {
        let engine = engine_mut(engine)?;
        engine.clear_error();
        engine.engine.config_mut().scanner.resolve_macos_alias = enabled != 0;
        Ok(())
    });

    if status == SmkStatus::Panic {
        set_engine_error(engine, "panic crossed scriptmetakit_ffi boundary");
    }
    status
}

#[unsafe(no_mangle)]
/// # Safety
///
/// Compatibility wrapper for a single-folder scan. `engine` must be a live
/// engine handle. If `path_len` is greater than zero, `path_ptr` must point to
/// `path_len` readable UTF-8 bytes for the duration of the call. `out_result`
/// must be a valid, writable pointer. A non-null result must be released with
/// `smk_scan_result_free`.
pub unsafe extern "C" fn smk_engine_scan_folder(
    engine: *mut SmkEngine,
    path_ptr: *const u8,
    path_len: usize,
    out_result: *mut *mut SmkScanResult,
) -> SmkStatus {
    let path = SmkUtf8Slice {
        ptr: path_ptr,
        len: path_len,
    };
    // SAFETY: forwards the caller-provided path slice as a one-element slice.
    unsafe { smk_engine_scan_folders(engine, &path, 1, 0, out_result) }
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `engine` must be a live engine handle. `paths_ptr` must point to
/// `path_count` readable `SmkUtf8Slice` values for the duration of the call.
/// Each non-empty path slice must contain valid UTF-8. `check_updates` treats
/// any non-zero value as true. `out_result` must be a valid, writable pointer.
/// A non-null result must be released with `smk_scan_result_free`.
pub unsafe extern "C" fn smk_engine_scan_folders(
    engine: *mut SmkEngine,
    paths_ptr: *const SmkUtf8Slice,
    path_count: usize,
    check_updates: u8,
    out_result: *mut *mut SmkScanResult,
) -> SmkStatus {
    let status = ffi_guard(|| {
        let engine = engine_mut(engine)?;
        let out_result = out_mut(out_result)?;
        *out_result = ptr::null_mut();
        engine.clear_error();

        let paths = utf8_path_slices(paths_ptr, path_count)?;
        if paths.is_empty() {
            let message = "folder path is empty".to_string();
            engine.set_error(&message);
            return Err((SmkStatus::InvalidArgument, message));
        }

        match scan_folders(
            &mut engine.engine,
            paths,
            check_updates != 0,
            None,
            ptr::null_mut(),
        ) {
            Ok(result) => {
                *out_result = Box::into_raw(Box::new(result));
                Ok(())
            }
            Err(error) => {
                let message = error.to_string();
                engine.set_error(&message);
                Err((SmkStatus::EngineError, message))
            }
        }
    });

    if status == SmkStatus::Panic {
        set_engine_error(engine, "panic crossed scriptmetakit_ffi boundary");
    }
    status
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `engine` must be a live engine handle. `paths_ptr` must point to
/// `path_count` readable `SmkUtf8Slice` values for the duration of the call.
/// Each non-empty path slice must contain valid UTF-8. `progress_callback`, if
/// non-null, is called synchronously during this function; strings in
/// `SmkUpdateProgress` are valid only for the duration of that callback.
/// `out_result` must be a valid, writable pointer. A non-null result must be
/// released with `smk_scan_result_free`.
pub unsafe extern "C" fn smk_engine_scan_folders_with_progress(
    engine: *mut SmkEngine,
    paths_ptr: *const SmkUtf8Slice,
    path_count: usize,
    check_updates: u8,
    progress_callback: SmkUpdateProgressCallback,
    progress_context: *mut c_void,
    out_result: *mut *mut SmkScanResult,
) -> SmkStatus {
    let status = ffi_guard(|| {
        let engine = engine_mut(engine)?;
        let out_result = out_mut(out_result)?;
        *out_result = ptr::null_mut();
        engine.clear_error();

        let paths = utf8_path_slices(paths_ptr, path_count)?;
        if paths.is_empty() {
            let message = "folder path is empty".to_string();
            engine.set_error(&message);
            return Err((SmkStatus::InvalidArgument, message));
        }

        match scan_folders(
            &mut engine.engine,
            paths,
            check_updates != 0,
            progress_callback,
            progress_context,
        ) {
            Ok(result) => {
                *out_result = Box::into_raw(Box::new(result));
                Ok(())
            }
            Err(error) => {
                let message = error.to_string();
                engine.set_error(&message);
                Err((SmkStatus::EngineError, message))
            }
        }
    });

    if status == SmkStatus::Panic {
        set_engine_error(engine, "panic crossed scriptmetakit_ffi boundary");
    }
    status
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `engine` must be a live engine handle. The engine must already have roots,
/// normally by calling one of the scan functions first.
pub unsafe extern "C" fn smk_engine_start_watching(engine: *mut SmkEngine) -> SmkStatus {
    let status = ffi_guard(|| {
        let engine = engine_mut(engine)?;
        engine.clear_error();
        start_watching_engine(engine, None, ptr::null_mut()).map_err(|message| {
            engine.set_error(&message);
            (SmkStatus::EngineError, message)
        })
    });

    if status == SmkStatus::Panic {
        set_engine_error(engine, "panic crossed scriptmetakit_ffi boundary");
    }
    status
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `engine` must be a live engine handle. The engine must already have roots,
/// normally by calling one of the scan functions first. `callback`, if non-null,
/// may be called from a background watcher thread after a file event has been
/// queued. `context` is passed through unchanged and must remain valid until
/// `smk_engine_stop_watching` or `smk_engine_free` returns.
pub unsafe extern "C" fn smk_engine_start_watching_with_callback(
    engine: *mut SmkEngine,
    callback: SmkWatchNotificationCallback,
    context: *mut c_void,
) -> SmkStatus {
    let status = ffi_guard(|| {
        let engine = engine_mut(engine)?;
        engine.clear_error();
        start_watching_engine(engine, callback, context).map_err(|message| {
            engine.set_error(&message);
            (SmkStatus::EngineError, message)
        })
    });

    if status == SmkStatus::Panic {
        set_engine_error(engine, "panic crossed scriptmetakit_ffi boundary");
    }
    status
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `engine` must be null or a live engine handle.
pub unsafe extern "C" fn smk_engine_stop_watching(engine: *mut SmkEngine) -> SmkStatus {
    let status = ffi_guard(|| {
        let engine = engine_mut(engine)?;
        engine.clear_error();
        stop_watching_engine(engine);
        Ok(())
    });

    if status == SmkStatus::Panic {
        set_engine_error(engine, "panic crossed scriptmetakit_ffi boundary");
    }
    status
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `engine` must be a live engine handle. `out_changed` and `out_result` must be
/// valid, writable pointers. When `out_changed` is set to zero, `out_result` is
/// set to null. A non-null result must be released with `smk_scan_result_free`.
pub unsafe extern "C" fn smk_engine_poll_watcher_scan(
    engine: *mut SmkEngine,
    out_changed: *mut u8,
    out_result: *mut *mut SmkScanResult,
) -> SmkStatus {
    let status = ffi_guard(|| {
        let engine = engine_mut(engine)?;
        let out_changed = out_mut(out_changed)?;
        let out_result = out_mut(out_result)?;
        *out_changed = 0;
        *out_result = ptr::null_mut();
        engine.clear_error();

        match poll_watcher_scan(engine) {
            Ok(Some(result)) => {
                *out_changed = 1;
                *out_result = Box::into_raw(Box::new(result));
                Ok(())
            }
            Ok(None) => Ok(()),
            Err(message) => {
                engine.set_error(&message);
                Err((SmkStatus::EngineError, message))
            }
        }
    });

    if status == SmkStatus::Panic {
        set_engine_error(engine, "panic crossed scriptmetakit_ffi boundary");
    }
    status
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `result` must be a live scan result handle. `out_roots` must be a valid,
/// writable pointer. The returned slice is borrowed from `result` and remains
/// valid only until `result` is freed.
pub unsafe extern "C" fn smk_scan_result_roots(
    result: *const SmkScanResult,
    out_roots: *mut SmkRootSnapshotSlice,
) -> SmkStatus {
    ffi_guard(|| {
        let result = scan_result_ref(result)?;
        let out_roots = out_mut(out_roots)?;
        *out_roots = SmkRootSnapshotSlice {
            ptr: result.roots.as_ptr(),
            len: result.roots.len(),
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `result` must be a live scan result handle. `out_file_lists` must be a
/// valid, writable pointer. The returned slice is borrowed from `result` and
/// remains valid only until `result` is freed.
pub unsafe extern "C" fn smk_scan_result_file_lists(
    result: *const SmkScanResult,
    out_file_lists: *mut SmkFileListSnapshotSlice,
) -> SmkStatus {
    ffi_guard(|| {
        let result = scan_result_ref(result)?;
        let out_file_lists = out_mut(out_file_lists)?;
        *out_file_lists = SmkFileListSnapshotSlice {
            ptr: result.file_lists.as_ptr(),
            len: result.file_lists.len(),
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `result` must be a live scan result handle. `out_file_entries` must be a
/// valid, writable pointer. The returned slice is borrowed from `result` and
/// remains valid only until `result` is freed.
pub unsafe extern "C" fn smk_scan_result_file_entries(
    result: *const SmkScanResult,
    out_file_entries: *mut SmkFileEntrySlice,
) -> SmkStatus {
    ffi_guard(|| {
        let result = scan_result_ref(result)?;
        let out_file_entries = out_mut(out_file_entries)?;
        *out_file_entries = SmkFileEntrySlice {
            ptr: result.file_entries.as_ptr(),
            len: result.file_entries.len(),
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `result` must be a live scan result handle. `out_items` must be a valid,
/// writable pointer. The returned item slice is borrowed from `result` and
/// remains valid only until `result` is freed.
pub unsafe extern "C" fn smk_scan_result_items(
    result: *const SmkScanResult,
    out_items: *mut SmkScriptItemSlice,
) -> SmkStatus {
    ffi_guard(|| {
        let result = scan_result_ref(result)?;
        let out_items = out_mut(out_items)?;
        *out_items = SmkScriptItemSlice {
            ptr: result.items.as_ptr(),
            len: result.items.len(),
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `result` must be a live scan result handle. `out_items` must be a valid,
/// writable pointer. The returned item slice is borrowed from `result` and
/// remains valid only until `result` is freed.
pub unsafe extern "C" fn smk_scan_result_file_items(
    result: *const SmkScanResult,
    out_items: *mut SmkScriptItemSlice,
) -> SmkStatus {
    ffi_guard(|| {
        let result = scan_result_ref(result)?;
        let out_items = out_mut(out_items)?;
        *out_items = SmkScriptItemSlice {
            ptr: result.file_items.as_ptr(),
            len: result.file_items.len(),
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `result` must be a live scan result handle. `out_info` must be a valid,
/// writable pointer.
pub unsafe extern "C" fn smk_scan_result_update_info(
    result: *const SmkScanResult,
    out_info: *mut SmkUpdateCheckInfo,
) -> SmkStatus {
    ffi_guard(|| {
        let result = scan_result_ref(result)?;
        let out_info = out_mut(out_info)?;
        *out_info = result.update_info;
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `result` must be a live scan result handle. `out_statuses` must be a valid,
/// writable pointer. The returned slice is borrowed from `result`.
pub unsafe extern "C" fn smk_scan_result_update_statuses(
    result: *const SmkScanResult,
    out_statuses: *mut SmkUpdateStatusEntrySlice,
) -> SmkStatus {
    ffi_guard(|| {
        let result = scan_result_ref(result)?;
        let out_statuses = out_mut(out_statuses)?;
        *out_statuses = SmkUpdateStatusEntrySlice {
            ptr: result.update_statuses.as_ptr(),
            len: result.update_statuses.len(),
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `result` must be a live scan result handle. `out_resolutions` must be a
/// valid, writable pointer. The returned slice is borrowed from `result`.
pub unsafe extern "C" fn smk_scan_result_update_resolutions(
    result: *const SmkScanResult,
    out_resolutions: *mut SmkDistributionResolutionEntrySlice,
) -> SmkStatus {
    ffi_guard(|| {
        let result = scan_result_ref(result)?;
        let out_resolutions = out_mut(out_resolutions)?;
        *out_resolutions = SmkDistributionResolutionEntrySlice {
            ptr: result.update_resolutions.as_ptr(),
            len: result.update_resolutions.len(),
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `result` must be a live scan result handle. `out_failures` must be a valid,
/// writable pointer. The returned slice is borrowed from `result`.
pub unsafe extern "C" fn smk_scan_result_update_failures(
    result: *const SmkScanResult,
    out_failures: *mut SmkUpdateFailureEntrySlice,
) -> SmkStatus {
    ffi_guard(|| {
        let result = scan_result_ref(result)?;
        let out_failures = out_mut(out_failures)?;
        *out_failures = SmkUpdateFailureEntrySlice {
            ptr: result.update_failures.as_ptr(),
            len: result.update_failures.len(),
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `result` must be a live scan result handle. `out_errors` must be a valid,
/// writable pointer. The returned slice is borrowed from `result`.
pub unsafe extern "C" fn smk_scan_result_update_errors(
    result: *const SmkScanResult,
    out_errors: *mut SmkUpdateErrorEntrySlice,
) -> SmkStatus {
    ffi_guard(|| {
        let result = scan_result_ref(result)?;
        let out_errors = out_mut(out_errors)?;
        *out_errors = SmkUpdateErrorEntrySlice {
            ptr: result.update_errors.as_ptr(),
            len: result.update_errors.len(),
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `result` must be a live scan result handle. `out_urls` must be a valid,
/// writable pointer. The returned slice is borrowed from `result`.
pub unsafe extern "C" fn smk_scan_result_latest_url_history_urls(
    result: *const SmkScanResult,
    out_urls: *mut SmkUtf8SliceSlice,
) -> SmkStatus {
    ffi_guard(|| {
        let result = scan_result_ref(result)?;
        let out_urls = out_mut(out_urls)?;
        *out_urls = SmkUtf8SliceSlice {
            ptr: result.latest_url_history_urls.as_ptr(),
            len: result.latest_url_history_urls.len(),
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `result` must be a live scan result handle. `out_info` must be a valid,
/// writable pointer.
pub unsafe extern "C" fn smk_scan_result_change_info(
    result: *const SmkScanResult,
    out_info: *mut SmkScanChangeInfo,
) -> SmkStatus {
    ffi_guard(|| {
        let result = scan_result_ref(result)?;
        let out_info = out_mut(out_info)?;
        *out_info = result.change_info;
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `result` must be a live scan result handle. `out_changes` must be a valid,
/// writable pointer. The returned slice is borrowed from `result`.
pub unsafe extern "C" fn smk_scan_result_file_entry_changes(
    result: *const SmkScanResult,
    out_changes: *mut SmkFileEntryChangeSlice,
) -> SmkStatus {
    ffi_guard(|| {
        let result = scan_result_ref(result)?;
        let out_changes = out_mut(out_changes)?;
        *out_changes = SmkFileEntryChangeSlice {
            ptr: result.file_entry_changes.as_ptr(),
            len: result.file_entry_changes.len(),
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `result` must be null or a live handle returned by `smk_engine_scan_folder`
/// or `smk_engine_scan_folders`. Each non-null handle must be freed at most once.
pub unsafe extern "C" fn smk_scan_result_free(result: *mut SmkScanResult) {
    if !result.is_null() {
        // SAFETY: `result` must be a pointer returned by this crate.
        unsafe {
            drop(Box::from_raw(result));
        }
    }
}

fn scan_folders(
    engine: &mut ScriptMetaKitEngine,
    folders: Vec<PathBuf>,
    check_updates: bool,
    progress_callback: SmkUpdateProgressCallback,
    progress_context: *mut c_void,
) -> scriptmetakit::ScriptMetaKitResult<SmkScanResult> {
    let roots = folders
        .into_iter()
        .map(|folder| {
            let root_id = folder
                .canonicalize()
                .unwrap_or_else(|_| folder.clone())
                .to_string_lossy()
                .into_owned();
            let display_name = folder
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| folder.display().to_string());
            RootRegistration {
                root_id,
                path: folder,
                display_name: Some(display_name),
                purpose: RootPurpose::FileListAndMetadata,
                watch_policy: WatchPolicy::AllRegistered,
                cache_policy: CachePolicy::MemoryAndPersistent,
                refresh_policy: RefreshPolicy::OnFileEvent,
                priority: RootPriority::UserInitiated,
            }
        })
        .collect();

    engine.set_roots(roots)?;
    let mut scan_result = engine.scan_roots(ScanRequest {
        root_ids: Vec::new(),
        mode: ScanMode::FileListAndMetadata,
    })?;
    let update_result = if check_updates {
        let items = scan_result
            .catalog_snapshot
            .as_ref()
            .map(|snapshot| snapshot.all_items.clone())
            .unwrap_or_default();
        let update_result = pollster::block_on(
            engine.check_updates_with_progress(UpdateCheckRequest { items }, |progress| {
                emit_update_progress(progress_callback, progress_context, &progress)
            }),
        )?;
        if let Some(snapshot) = scan_result.catalog_snapshot.as_mut() {
            snapshot.update_check_result = Some(update_result.clone());
        }
        Some(update_result)
    } else {
        None
    };

    Ok(SmkScanResult::from_scan_result(scan_result, update_result))
}

#[cfg(feature = "native-watch")]
fn start_watching_engine(
    engine: &mut SmkEngine,
    callback: SmkWatchNotificationCallback,
    context: *mut c_void,
) -> Result<(), String> {
    let plan = engine.engine.watch_plan();
    let watcher = if let Some(callback) = callback {
        let context = context as usize;
        NativeWatcher::start_with_notifier(
            &plan,
            Some(Arc::new(move || {
                callback(context as *mut c_void);
            })),
        )
    } else {
        NativeWatcher::start(&plan)
    }
    .map_err(|error| error.to_string())?;
    engine.watcher = Some(watcher);
    Ok(())
}

#[cfg(not(feature = "native-watch"))]
fn start_watching_engine(
    _engine: &mut SmkEngine,
    _callback: SmkWatchNotificationCallback,
    _context: *mut c_void,
) -> Result<(), String> {
    Err("native-watch feature is not enabled".to_string())
}

#[cfg(feature = "native-watch")]
fn stop_watching_engine(engine: &mut SmkEngine) {
    engine.watcher = None;
}

#[cfg(not(feature = "native-watch"))]
fn stop_watching_engine(_engine: &mut SmkEngine) {}

#[cfg(feature = "native-watch")]
fn poll_watcher_scan(engine: &mut SmkEngine) -> Result<Option<SmkScanResult>, String> {
    let Some(watcher) = engine.watcher.as_ref() else {
        return Err("watcher is not running".to_string());
    };
    let Some(mut batch) = watcher.try_recv() else {
        return Ok(None);
    };
    while let Some(next_batch) = watcher.try_recv() {
        batch.paths.extend(next_batch.paths);
        batch.overflowed |= next_batch.overflowed;
    }
    batch.paths.sort();
    batch.paths.dedup();

    engine
        .engine
        .mark_changed_paths(batch)
        .map_err(|error| error.to_string())?;
    let scan_result = engine
        .engine
        .refresh_dirty_roots(RefreshRequest {
            mode: ScanMode::FileListAndMetadata,
        })
        .map_err(|error| error.to_string())?;
    Ok(Some(SmkScanResult::from_scan_result(scan_result, None)))
}

#[cfg(not(feature = "native-watch"))]
fn poll_watcher_scan(_engine: &mut SmkEngine) -> Result<Option<SmkScanResult>, String> {
    Err("native-watch feature is not enabled".to_string())
}

impl SmkScanResult {
    fn from_scan_result(scan_result: ScanResult, update_result: Option<UpdateCheckResult>) -> Self {
        let update_result = update_result.or_else(|| {
            scan_result
                .catalog_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.update_check_result.clone())
        });
        let string_capacity = total_string_bytes(&scan_result, update_result.as_ref());
        let mut result = Self {
            string_storage: Vec::with_capacity(string_capacity),
            roots: Vec::with_capacity(scan_result.roots.len()),
            file_lists: Vec::with_capacity(scan_result.file_list_snapshots.len()),
            file_entries: Vec::new(),
            items: scan_result
                .catalog_snapshot
                .as_ref()
                .map_or_else(Vec::new, |snapshot| {
                    Vec::with_capacity(snapshot.all_items.len())
                }),
            file_items: scan_result
                .catalog_snapshot
                .as_ref()
                .map_or_else(Vec::new, |snapshot| {
                    Vec::with_capacity(snapshot.file_items.len())
                }),
            update_info: SmkUpdateCheckInfo::default(),
            update_statuses: Vec::new(),
            update_resolutions: Vec::new(),
            update_failures: Vec::new(),
            update_errors: Vec::new(),
            latest_url_history_urls: Vec::new(),
            change_info: SmkScanChangeInfo::default(),
            file_entry_changes: Vec::new(),
        };

        for root in &scan_result.roots {
            result.push_root(root);
        }

        let root_indices = result.root_index_by_id();
        for snapshot in &scan_result.file_list_snapshots {
            result.push_file_list(snapshot, &root_indices);
        }

        if let Some(snapshot) = scan_result.catalog_snapshot.as_ref() {
            for item in &snapshot.all_items {
                result.push_script_item(item);
            }
            for item in &snapshot.file_items {
                result.push_file_script_item(item);
            }
        }

        if let Some(change_summary) = scan_result.change_summary.as_ref() {
            result.push_change_summary(change_summary);
        }

        if let Some(update_result) = update_result {
            result.push_update_result(&update_result);
        }

        result
    }

    fn root_index_by_id(&self) -> BTreeMap<String, usize> {
        self.roots
            .iter()
            .enumerate()
            .map(|(index, root)| (self.string(root.root_id), index))
            .collect()
    }

    fn push_root(&mut self, root: &RootSnapshot) {
        let (has_last_loaded_at, last_loaded_at) = optional_u64(root.last_loaded_at);
        let (has_last_event_at, last_event_at) = optional_u64(root.last_event_at);
        let ffi_root = SmkRootSnapshot {
            root_id: self.push_string(Some(root.root_id.as_str())),
            path: self.push_path(&root.path),
            status: self.push_string(Some(root_status(root.status))),
            is_dirty: bool_byte(root.is_dirty),
            has_last_loaded_at,
            last_loaded_at,
            has_last_event_at,
            last_event_at,
            item_count: root.item_count,
            error_code: self.push_string(root.error.as_ref().map(|error| error.code.as_str())),
            error_message: self
                .push_string(root.error.as_ref().map(|error| error.message.as_str())),
        };
        self.roots.push(ffi_root);
    }

    fn push_file_list(
        &mut self,
        snapshot: &FileListSnapshot,
        root_indices: &BTreeMap<String, usize>,
    ) {
        let first_child_index = self.file_entries.len();
        let child_count = snapshot
            .children
            .as_ref()
            .map_or(0, |children| self.push_file_entries(children));
        let root_index = root_indices
            .get(snapshot.root.root_id.as_str())
            .copied()
            .unwrap_or(usize::MAX);
        self.file_lists.push(SmkFileListSnapshot {
            root_index,
            first_child_index,
            child_count,
            truncated: bool_byte(snapshot.truncated),
        });
    }

    fn push_file_entries(&mut self, entries: &[FileSystemEntry]) -> usize {
        let first_entry_index = self.file_entries.len();
        for entry in entries {
            let ffi_entry = SmkFileEntry {
                display_path: self.push_path(&entry.display_path),
                resolved_path: self.push_path(&entry.resolved_path),
                path_kind: self.push_string(Some(entry.path_kind.as_str())),
                resolution_status: self.push_string(Some(entry.resolution_status.as_str())),
                resolution_message: self.push_string(entry.resolution_message.as_deref()),
                is_directory: bool_byte(entry.is_directory),
                has_file_size: bool_byte(entry.file_size.is_some()),
                file_size: entry.file_size.unwrap_or_default(),
                has_content_modified_at: bool_byte(entry.content_modified_at.is_some()),
                content_modified_at: entry.content_modified_at.unwrap_or_default(),
                runtime_kind: self.push_string(entry.runtime_kind.map(script_runtime_kind)),
                shebang: self.push_string(entry.shebang.as_deref()),
                has_scriptmeta: bool_byte(entry.has_scriptmeta),
                has_scriptmeta_edit_password: bool_byte(entry.has_scriptmeta_edit_password),
                is_file_locked: bool_byte(entry.is_file_locked),
                is_read_only: bool_byte(entry.is_read_only),
                can_edit_scriptmeta: bool_byte(entry.can_edit_scriptmeta),
                can_append_scriptmeta: bool_byte(entry.can_append_scriptmeta),
                scriptmeta_edit_state: self.push_string(Some(entry.scriptmeta_edit_state.as_str())),
                first_child_index: 0,
                child_count: 0,
            };
            self.file_entries.push(ffi_entry);
        }

        for (offset, entry) in entries.iter().enumerate() {
            let entry_index = first_entry_index + offset;
            let first_child_index = self.file_entries.len();
            let child_count = self.push_file_entries(&entry.children);
            self.file_entries[entry_index].first_child_index = first_child_index;
            self.file_entries[entry_index].child_count = child_count;
        }
        entries.len()
    }

    fn push_script_item(&mut self, item: &ScriptMetaItem) {
        let ffi_item = self.script_item(item);
        self.items.push(ffi_item);
    }

    fn push_file_script_item(&mut self, item: &ScriptMetaItem) {
        let ffi_item = self.script_item(item);
        self.file_items.push(ffi_item);
    }

    fn script_item(&mut self, item: &ScriptMetaItem) -> SmkScriptItem {
        SmkScriptItem {
            root_id: self.push_string(Some(item.root_id.as_str())),
            file_path: self.push_path(&item.file_path),
            identity_path: self.push_path(&item.identity_path),
            runtime_kind: self.push_string(item.runtime_kind.map(script_runtime_kind)),
            shebang: self.push_string(item.shebang.as_deref()),
            script_id: self.push_string(Some(item.script_id.as_str())),
            version: self.push_string(item.version.as_deref()),
            name: self.push_string(item.name.as_deref()),
            description: self.push_string(item.description.as_deref()),
            target_app: self.push_string(item.target_app.as_deref()),
            meta_url: self.push_url(item.meta_url.as_ref()),
            author: self.push_string(item.author.as_deref()),
            release_date: self.push_string(item.release_date.as_deref()),
            edit_password_sha256: self.push_string(item.edit_password_sha256.as_deref()),
            has_scriptmeta: bool_byte(item.has_scriptmeta),
            has_scriptmeta_edit_password: bool_byte(item.has_scriptmeta_edit_password),
            is_file_locked: bool_byte(item.is_file_locked),
            is_read_only: bool_byte(item.is_read_only),
            can_edit_scriptmeta: bool_byte(item.can_edit_scriptmeta),
            can_append_scriptmeta: bool_byte(item.can_append_scriptmeta),
            scriptmeta_edit_state: self.push_string(Some(item.scriptmeta_edit_state.as_str())),
        }
    }

    fn push_change_summary(&mut self, summary: &ScanChangeSummary) {
        self.change_info = SmkScanChangeInfo {
            has_change_summary: 1,
            added_count: summary.added_count,
            removed_count: summary.removed_count,
            modified_count: summary.modified_count,
        };
        self.file_entry_changes.reserve(summary.changes.len());
        for change in &summary.changes {
            self.push_file_entry_change(change);
        }
    }

    fn push_file_entry_change(&mut self, change: &FileEntryChange) {
        let entry = SmkFileEntryChange {
            root_id: self.push_string(Some(change.root_id.as_str())),
            kind: self.push_string(Some(file_entry_change_kind(change.kind))),
            display_path: self.push_path(&change.display_path),
            resolved_path: self.push_path(&change.resolved_path),
            path_kind: self.push_string(Some(change.path_kind.as_str())),
            resolution_status: self.push_string(Some(change.resolution_status.as_str())),
            resolution_message: self.push_string(change.resolution_message.as_deref()),
            is_directory: bool_byte(change.is_directory),
            has_file_size: bool_byte(change.file_size.is_some()),
            file_size: change.file_size.unwrap_or_default(),
            has_content_modified_at: bool_byte(change.content_modified_at.is_some()),
            content_modified_at: change.content_modified_at.unwrap_or_default(),
            runtime_kind: self.push_string(change.runtime_kind.map(script_runtime_kind)),
            shebang: self.push_string(change.shebang.as_deref()),
            has_scriptmeta: bool_byte(change.has_scriptmeta),
            has_scriptmeta_edit_password: bool_byte(change.has_scriptmeta_edit_password),
            is_file_locked: bool_byte(change.is_file_locked),
            is_read_only: bool_byte(change.is_read_only),
            can_edit_scriptmeta: bool_byte(change.can_edit_scriptmeta),
            can_append_scriptmeta: bool_byte(change.can_append_scriptmeta),
            scriptmeta_edit_state: self.push_string(Some(change.scriptmeta_edit_state.as_str())),
        };
        self.file_entry_changes.push(entry);
    }

    fn push_update_result(&mut self, update_result: &UpdateCheckResult) {
        self.update_info = SmkUpdateCheckInfo {
            has_update_check: 1,
            checked_at: update_result.checked_at,
        };
        self.update_statuses
            .reserve(update_result.statuses_by_item_id.len());
        self.update_resolutions
            .reserve(update_result.resolutions_by_item_id.len());
        self.update_failures
            .reserve(update_result.failures_by_item_id.len());
        self.update_errors
            .reserve(update_result.errors_by_item_id.len());

        for (item_id, status) in &update_result.statuses_by_item_id {
            let entry = SmkUpdateStatusEntry {
                item_id: self.push_string(Some(item_id.as_str())),
                status: self.push_string(Some(update_status(*status))),
            };
            self.update_statuses.push(entry);
        }

        for (item_id, resolution) in &update_result.resolutions_by_item_id {
            self.push_distribution_resolution(item_id, resolution);
        }

        for (item_id, failure) in &update_result.failures_by_item_id {
            self.push_update_failure(item_id, failure);
        }

        for (item_id, message) in &update_result.errors_by_item_id {
            let entry = SmkUpdateErrorEntry {
                item_id: self.push_string(Some(item_id.as_str())),
                message: self.push_string(Some(message.as_str())),
            };
            self.update_errors.push(entry);
        }
    }

    fn push_distribution_resolution(&mut self, item_id: &str, resolution: &DistributionResolution) {
        let first_latest_url_history_index = self.latest_url_history_urls.len();
        for url in &resolution.latest_url_history {
            let slice = self.push_url(Some(url));
            self.latest_url_history_urls.push(slice);
        }
        let (has_redirect_count, redirect_count) = optional_u32(resolution.redirect_count);
        let entry = SmkDistributionResolutionEntry {
            item_id: self.push_string(Some(item_id)),
            latest_version: self.push_string(resolution.latest_version.as_deref()),
            latest_page_url: self.push_url(resolution.latest_page_url.as_ref()),
            final_page_url: self.push_url(Some(&resolution.final_page_url)),
            first_latest_url_history_index,
            latest_url_history_count: resolution.latest_url_history.len(),
            checked_at: resolution.checked_at,
            is_unresolved: bool_byte(resolution.is_unresolved),
            note: self.push_string(resolution.note.as_deref()),
            has_redirect_count,
            redirect_count,
        };
        self.update_resolutions.push(entry);
    }

    fn push_update_failure(&mut self, item_id: &str, failure: &UpdateFailure) {
        let entry = SmkUpdateFailureEntry {
            item_id: self.push_string(Some(item_id)),
            code: self.push_string(Some(failure.code.as_str())),
            message: self.push_string(Some(failure.message.as_str())),
            file_path: self.push_path(&failure.file_path),
            script_id: self.push_string(Some(failure.script_id.as_str())),
            current_version: self.push_string(failure.current_version.as_deref()),
            meta_url: self.push_url(failure.meta_url.as_ref()),
            source_url: self.push_url(failure.source_url.as_ref()),
            checked_at: failure.checked_at,
        };
        self.update_failures.push(entry);
    }

    fn push_string(&mut self, value: Option<&str>) -> SmkUtf8Slice {
        let Some(value) = value.filter(|value| !value.is_empty()) else {
            return SmkUtf8Slice::empty();
        };
        debug_assert!(
            self.string_storage.len().saturating_add(value.len()) <= self.string_storage.capacity()
        );
        let start = self.string_storage.len();
        self.string_storage.extend_from_slice(value.as_bytes());
        borrowed_slice(&self.string_storage[start..])
    }

    fn push_path(&mut self, path: &Path) -> SmkUtf8Slice {
        let value = path.to_string_lossy();
        self.push_string(Some(value.as_ref()))
    }

    fn push_url(&mut self, url: Option<&Url>) -> SmkUtf8Slice {
        self.push_string(url.map(Url::as_str))
    }

    fn string(&self, slice: SmkUtf8Slice) -> String {
        if slice.ptr.is_null() || slice.len == 0 {
            return String::new();
        }
        // SAFETY: the slice points into `self.string_storage`.
        let bytes = unsafe { slice::from_raw_parts(slice.ptr, slice.len) };
        String::from_utf8_lossy(bytes).into_owned()
    }
}

fn total_string_bytes(
    scan_result: &ScanResult,
    update_result: Option<&UpdateCheckResult>,
) -> usize {
    let root_bytes = scan_result
        .roots
        .iter()
        .map(root_string_bytes)
        .sum::<usize>();
    let file_list_bytes = scan_result
        .file_list_snapshots
        .iter()
        .flat_map(|snapshot| snapshot.children.as_deref().unwrap_or_default())
        .map(file_entry_string_bytes)
        .sum::<usize>();
    let item_bytes = scan_result.catalog_snapshot.as_ref().map_or(0, |snapshot| {
        snapshot
            .all_items
            .iter()
            .chain(snapshot.file_items.iter())
            .map(item_string_bytes)
            .sum::<usize>()
    });
    let change_bytes = scan_result
        .change_summary
        .as_ref()
        .map_or(0, scan_change_summary_string_bytes);
    let update_bytes = update_result.map_or(0, update_result_string_bytes);
    root_bytes
        .saturating_add(file_list_bytes)
        .saturating_add(item_bytes)
        .saturating_add(change_bytes)
        .saturating_add(update_bytes)
}

fn root_string_bytes(root: &RootSnapshot) -> usize {
    root.root_id
        .len()
        .saturating_add(root.path.to_string_lossy().len())
        .saturating_add(root_status(root.status).len())
        .saturating_add(root.error.as_ref().map_or(0, |error| {
            error.code.len().saturating_add(error.message.len())
        }))
}

fn file_entry_string_bytes(entry: &FileSystemEntry) -> usize {
    entry
        .display_path
        .to_string_lossy()
        .len()
        .saturating_add(entry.resolved_path.to_string_lossy().len())
        .saturating_add(entry.path_kind.as_str().len())
        .saturating_add(entry.resolution_status.as_str().len())
        .saturating_add(entry.resolution_message.as_deref().map_or(0, str::len))
        .saturating_add(
            entry
                .runtime_kind
                .map_or(0, |kind| script_runtime_kind(kind).len()),
        )
        .saturating_add(entry.shebang.as_deref().map_or(0, str::len))
        .saturating_add(entry.scriptmeta_edit_state.as_str().len())
        .saturating_add(
            entry
                .children
                .iter()
                .map(file_entry_string_bytes)
                .sum::<usize>(),
        )
}

fn item_string_bytes(item: &ScriptMetaItem) -> usize {
    [
        item.root_id.len(),
        item.file_path.to_string_lossy().len(),
        item.identity_path.to_string_lossy().len(),
        item.runtime_kind
            .map_or(0, |kind| script_runtime_kind(kind).len()),
        item.shebang.as_deref().map_or(0, str::len),
        item.script_id.len(),
        item.version.as_deref().map_or(0, str::len),
        item.name.as_deref().map_or(0, str::len),
        item.description.as_deref().map_or(0, str::len),
        item.target_app.as_deref().map_or(0, str::len),
        item.meta_url.as_ref().map_or(0, |url| url.as_str().len()),
        item.author.as_deref().map_or(0, str::len),
        item.release_date.as_deref().map_or(0, str::len),
        item.edit_password_sha256.as_deref().map_or(0, str::len),
        item.scriptmeta_edit_state.as_str().len(),
    ]
    .into_iter()
    .fold(0usize, usize::saturating_add)
}

fn scan_change_summary_string_bytes(summary: &ScanChangeSummary) -> usize {
    summary
        .changes
        .iter()
        .map(file_entry_change_string_bytes)
        .sum::<usize>()
}

fn file_entry_change_string_bytes(change: &FileEntryChange) -> usize {
    change
        .root_id
        .len()
        .saturating_add(file_entry_change_kind(change.kind).len())
        .saturating_add(change.display_path.to_string_lossy().len())
        .saturating_add(change.resolved_path.to_string_lossy().len())
        .saturating_add(change.path_kind.as_str().len())
        .saturating_add(change.resolution_status.as_str().len())
        .saturating_add(change.resolution_message.as_deref().map_or(0, str::len))
        .saturating_add(
            change
                .runtime_kind
                .map_or(0, |kind| script_runtime_kind(kind).len()),
        )
        .saturating_add(change.shebang.as_deref().map_or(0, str::len))
        .saturating_add(change.scriptmeta_edit_state.as_str().len())
}

fn update_result_string_bytes(update_result: &UpdateCheckResult) -> usize {
    let statuses = update_result
        .statuses_by_item_id
        .iter()
        .map(|(item_id, status)| item_id.len().saturating_add(update_status(*status).len()))
        .sum::<usize>();
    let resolutions = update_result
        .resolutions_by_item_id
        .iter()
        .map(|(item_id, resolution)| {
            item_id
                .len()
                .saturating_add(resolution.latest_version.as_deref().map_or(0, str::len))
                .saturating_add(
                    resolution
                        .latest_page_url
                        .as_ref()
                        .map_or(0, |url| url.as_str().len()),
                )
                .saturating_add(resolution.final_page_url.as_str().len())
                .saturating_add(
                    resolution
                        .latest_url_history
                        .iter()
                        .map(|url| url.as_str().len())
                        .sum::<usize>(),
                )
                .saturating_add(resolution.note.as_deref().map_or(0, str::len))
        })
        .sum::<usize>();
    let failures = update_result
        .failures_by_item_id
        .iter()
        .map(|(item_id, failure)| {
            item_id
                .len()
                .saturating_add(failure.code.len())
                .saturating_add(failure.message.len())
                .saturating_add(failure.file_path.to_string_lossy().len())
                .saturating_add(failure.script_id.len())
                .saturating_add(failure.current_version.as_deref().map_or(0, str::len))
                .saturating_add(
                    failure
                        .meta_url
                        .as_ref()
                        .map_or(0, |url| url.as_str().len()),
                )
                .saturating_add(
                    failure
                        .source_url
                        .as_ref()
                        .map_or(0, |url| url.as_str().len()),
                )
        })
        .sum::<usize>();
    let errors = update_result
        .errors_by_item_id
        .iter()
        .map(|(item_id, message)| item_id.len().saturating_add(message.len()))
        .sum::<usize>();
    statuses
        .saturating_add(resolutions)
        .saturating_add(failures)
        .saturating_add(errors)
}

impl SmkEngine {
    fn clear_error(&mut self) {
        self.last_error.clear();
    }

    fn set_error(&mut self, message: &str) {
        self.last_error.clear();
        self.last_error.extend_from_slice(message.as_bytes());
    }
}

fn ffi_guard<F>(operation: F) -> SmkStatus
where
    F: FnOnce() -> Result<(), (SmkStatus, String)>,
{
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => SmkStatus::Ok,
        Ok(Err((status, _message))) => status,
        Err(_) => SmkStatus::Panic,
    }
}

fn out_mut<'a, T>(ptr: *mut T) -> Result<&'a mut T, (SmkStatus, String)> {
    if ptr.is_null() {
        return Err((
            SmkStatus::NullArgument,
            "output pointer is null".to_string(),
        ));
    }
    // SAFETY: the caller provided a non-null output pointer and C ABI requires it to be writable.
    Ok(unsafe { &mut *ptr })
}

fn engine_mut<'a>(engine: *mut SmkEngine) -> Result<&'a mut SmkEngine, (SmkStatus, String)> {
    if engine.is_null() {
        return Err((SmkStatus::NullArgument, "engine handle is null".to_string()));
    }
    // SAFETY: the caller must pass a live `SmkEngine` returned by this crate.
    Ok(unsafe { &mut *engine })
}

fn engine_ref<'a>(engine: *const SmkEngine) -> Result<&'a SmkEngine, (SmkStatus, String)> {
    if engine.is_null() {
        return Err((SmkStatus::NullArgument, "engine handle is null".to_string()));
    }
    // SAFETY: the caller must pass a live `SmkEngine` returned by this crate.
    Ok(unsafe { &*engine })
}

fn scan_result_ref<'a>(
    result: *const SmkScanResult,
) -> Result<&'a SmkScanResult, (SmkStatus, String)> {
    if result.is_null() {
        return Err((
            SmkStatus::NullArgument,
            "scan result handle is null".to_string(),
        ));
    }
    // SAFETY: the caller must pass a live `SmkScanResult` returned by this crate.
    Ok(unsafe { &*result })
}

fn utf8_path_slices(
    ptr: *const SmkUtf8Slice,
    len: usize,
) -> Result<Vec<PathBuf>, (SmkStatus, String)> {
    if len == 0 {
        return Ok(Vec::new());
    }
    if ptr.is_null() {
        return Err((SmkStatus::NullArgument, "path slice is null".to_string()));
    }
    // SAFETY: the caller promises `ptr` points to `len` readable path slices.
    let slices = unsafe { slice::from_raw_parts(ptr, len) };
    let mut paths = Vec::with_capacity(slices.len());
    for value in slices {
        let path = utf8_from_raw(value.ptr, value.len)?;
        if !path.is_empty() {
            paths.push(PathBuf::from(path));
        }
    }
    Ok(paths)
}

fn utf8_from_raw<'a>(ptr: *const u8, len: usize) -> Result<&'a str, (SmkStatus, String)> {
    if len == 0 {
        return Ok("");
    }
    if ptr.is_null() {
        return Err((SmkStatus::NullArgument, "input slice is null".to_string()));
    }
    // SAFETY: the caller promises that `ptr` points to `len` readable bytes for this call.
    let bytes = unsafe { slice::from_raw_parts(ptr, len) };
    str::from_utf8(bytes).map_err(|error| (SmkStatus::InvalidUtf8, error.to_string()))
}

fn borrowed_slice(bytes: &[u8]) -> SmkUtf8Slice {
    if bytes.is_empty() {
        SmkUtf8Slice::empty()
    } else {
        SmkUtf8Slice {
            ptr: bytes.as_ptr(),
            len: bytes.len(),
        }
    }
}

fn borrowed_str_slice(value: Option<&str>) -> SmkUtf8Slice {
    value.map_or_else(SmkUtf8Slice::empty, |value| {
        borrowed_slice(value.as_bytes())
    })
}

fn emit_update_progress(
    callback: SmkUpdateProgressCallback,
    context: *mut c_void,
    progress: &UpdateCheckProgress,
) {
    let Some(callback) = callback else {
        return;
    };
    let phase = update_progress_phase(progress.phase);
    let ffi_progress = SmkUpdateProgress {
        completed_items: progress.completed_items,
        total_items: progress.total_items,
        item_id: borrowed_str_slice(progress.item_id.as_deref()),
        script_id: borrowed_str_slice(progress.script_id.as_deref()),
        phase: borrowed_slice(phase.as_bytes()),
        message: borrowed_slice(progress.message.as_bytes()),
    };
    callback(&ffi_progress, context);
}

fn set_engine_error(engine: *mut SmkEngine, message: &str) {
    if !engine.is_null() {
        // SAFETY: best-effort error recording for a caller-provided live handle.
        unsafe {
            (*engine).set_error(message);
        }
    }
}

fn bool_byte(value: bool) -> u8 {
    u8::from(value)
}

fn optional_u64(value: Option<u64>) -> (u8, u64) {
    value.map_or((0, 0), |value| (1, value))
}

fn optional_u32(value: Option<u32>) -> (u8, u32) {
    value.map_or((0, 0), |value| (1, value))
}

fn root_status(status: RootStatus) -> &'static str {
    match status {
        RootStatus::NotLoaded => "not_loaded",
        RootStatus::Ready => "ready",
        RootStatus::Dirty => "dirty",
        RootStatus::Loading => "loading",
        RootStatus::Missing => "missing",
        RootStatus::Unreadable => "unreadable",
        RootStatus::TimedOut => "timed_out",
        RootStatus::Overflowed => "overflowed",
    }
}

fn script_runtime_kind(kind: ScriptRuntimeKind) -> &'static str {
    match kind {
        ScriptRuntimeKind::AppleScript => "apple_script",
        ScriptRuntimeKind::JavaScriptForAutomation => "javascript_for_automation",
        ScriptRuntimeKind::AdobeJavaScript => "adobe_java_script",
        ScriptRuntimeKind::AdobeUxp => "adobe_uxp",
    }
}

fn file_entry_change_kind(kind: FileEntryChangeKind) -> &'static str {
    match kind {
        FileEntryChangeKind::Added => "added",
        FileEntryChangeKind::Removed => "removed",
        FileEntryChangeKind::Modified => "modified",
    }
}

fn update_status(status: UpdateStatus) -> &'static str {
    match status {
        UpdateStatus::Idle => "idle",
        UpdateStatus::Checking => "checking",
        UpdateStatus::UpToDate => "up_to_date",
        UpdateStatus::UpdateAvailable => "update_available",
        UpdateStatus::Failed => "failed",
        UpdateStatus::NotCheckable => "not_checkable",
    }
}

fn update_progress_phase(phase: UpdateCheckProgressPhase) -> &'static str {
    match phase {
        UpdateCheckProgressPhase::Started => "started",
        UpdateCheckProgressPhase::Checking => "checking",
        UpdateCheckProgressPhase::Retrying => "retrying",
        UpdateCheckProgressPhase::FinishedItem => "finished_item",
        UpdateCheckProgressPhase::FailedItem => "failed_item",
        UpdateCheckProgressPhase::Finished => "finished",
    }
}
