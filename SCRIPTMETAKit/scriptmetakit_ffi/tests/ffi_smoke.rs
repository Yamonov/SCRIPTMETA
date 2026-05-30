use std::{ffi::c_void, ptr, slice};
#[cfg(feature = "native-watch")]
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

use scriptmetakit_ffi::{
    SmkDistributionResolutionEntrySlice, SmkEngine, SmkFileEntryChangeSlice, SmkFileEntrySlice,
    SmkFileListSnapshotSlice, SmkRootSnapshotSlice, SmkScanChangeInfo, SmkScanResult,
    SmkScriptItemSlice, SmkStatus, SmkUpdateCheckInfo, SmkUpdateProgress,
    SmkUpdateStatusEntrySlice, SmkUtf8Slice, smk_engine_create_default, smk_engine_free,
    smk_engine_last_error, smk_engine_scan_folder, smk_engine_scan_folders,
    smk_engine_scan_folders_with_progress, smk_engine_set_resolve_macos_alias,
    smk_scan_result_change_info, smk_scan_result_file_entries, smk_scan_result_file_entry_changes,
    smk_scan_result_file_items, smk_scan_result_file_lists, smk_scan_result_free,
    smk_scan_result_items, smk_scan_result_roots, smk_scan_result_update_info,
    smk_scan_result_update_resolutions, smk_scan_result_update_statuses,
};
#[cfg(feature = "native-watch")]
use scriptmetakit_ffi::{
    smk_engine_poll_watcher_scan, smk_engine_start_watching,
    smk_engine_start_watching_with_callback, smk_engine_stop_watching,
};

#[test]
fn scans_items_through_opaque_handle_and_slice() {
    let temp = tempfile::tempdir().expect("tempdir");
    let script_path = temp.path().join("Example.jsx");
    std::fs::write(
        &script_path,
        r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.ffi
// Version: 1.2.3
// Name: FFI Example
// SCRIPTMETA-END
"#,
    )
    .expect("script");

    let mut engine: *mut SmkEngine = ptr::null_mut();
    // SAFETY: `engine` is a valid out pointer for the duration of this call.
    assert_eq!(
        unsafe { smk_engine_create_default(&mut engine) },
        SmkStatus::Ok
    );
    assert!(!engine.is_null());

    let path = temp.path().to_string_lossy().into_owned();
    let mut scan_result: *mut SmkScanResult = ptr::null_mut();
    // SAFETY: `engine` is live, path bytes are valid for the call, and `scan_result` is writable.
    assert_eq!(
        unsafe { smk_engine_scan_folder(engine, path.as_ptr(), path.len(), &mut scan_result) },
        SmkStatus::Ok
    );
    assert!(!scan_result.is_null());

    let mut items = SmkScriptItemSlice {
        ptr: ptr::null(),
        len: 0,
    };
    // SAFETY: `scan_result` is live and `items` is a valid out pointer.
    assert_eq!(
        unsafe { smk_scan_result_items(scan_result, &mut items) },
        SmkStatus::Ok
    );
    assert_eq!(items.len, 1);

    // SAFETY: `items` is borrowed from `scan_result` and the result is still alive.
    let items = unsafe { slice::from_raw_parts(items.ptr, items.len) };
    assert_eq!(utf8(items[0].script_id), "com.example.ffi");
    assert_eq!(utf8(items[0].version), "1.2.3");
    assert_eq!(utf8(items[0].name), "FFI Example");
    assert_eq!(utf8(items[0].runtime_kind), "adobe_java_script");
    assert_eq!(items[0].has_scriptmeta, 1);
    assert_eq!(items[0].can_edit_scriptmeta, 1);
    assert_eq!(items[0].can_append_scriptmeta, 0);
    assert_eq!(utf8(items[0].scriptmeta_edit_state), "editable");

    let mut roots = SmkRootSnapshotSlice {
        ptr: ptr::null(),
        len: 0,
    };
    // SAFETY: `scan_result` is live and `roots` is a valid out pointer.
    assert_eq!(
        unsafe { smk_scan_result_roots(scan_result, &mut roots) },
        SmkStatus::Ok
    );
    assert_eq!(roots.len, 1);

    let mut file_lists = SmkFileListSnapshotSlice {
        ptr: ptr::null(),
        len: 0,
    };
    // SAFETY: `scan_result` is live and `file_lists` is a valid out pointer.
    assert_eq!(
        unsafe { smk_scan_result_file_lists(scan_result, &mut file_lists) },
        SmkStatus::Ok
    );
    assert_eq!(file_lists.len, 1);

    let mut file_entries = SmkFileEntrySlice {
        ptr: ptr::null(),
        len: 0,
    };
    // SAFETY: `scan_result` is live and `file_entries` is a valid out pointer.
    assert_eq!(
        unsafe { smk_scan_result_file_entries(scan_result, &mut file_entries) },
        SmkStatus::Ok
    );
    assert_eq!(file_entries.len, 1);
    // SAFETY: `file_entries` is borrowed from `scan_result` and the result is still alive.
    let file_entries = unsafe { slice::from_raw_parts(file_entries.ptr, file_entries.len) };
    assert_eq!(file_entries[0].has_scriptmeta, 1);
    assert_eq!(file_entries[0].can_edit_scriptmeta, 1);
    assert_eq!(file_entries[0].can_append_scriptmeta, 0);
    assert_eq!(utf8(file_entries[0].scriptmeta_edit_state), "editable");

    let mut update_info = SmkUpdateCheckInfo {
        has_update_check: 1,
        checked_at: 1,
    };
    // SAFETY: `scan_result` is live and `update_info` is a valid out pointer.
    assert_eq!(
        unsafe { smk_scan_result_update_info(scan_result, &mut update_info) },
        SmkStatus::Ok
    );
    assert_eq!(update_info.has_update_check, 0);

    let mut statuses = SmkUpdateStatusEntrySlice {
        ptr: ptr::null(),
        len: 0,
    };
    // SAFETY: `scan_result` is live and `statuses` is a valid out pointer.
    assert_eq!(
        unsafe { smk_scan_result_update_statuses(scan_result, &mut statuses) },
        SmkStatus::Ok
    );
    assert_eq!(statuses.len, 0);

    // SAFETY: both handles were returned by this FFI crate and have not been freed.
    unsafe {
        smk_scan_result_free(scan_result);
        smk_engine_free(engine);
    }
}

#[test]
fn configures_alias_resolution_through_ffi() {
    let mut engine: *mut SmkEngine = ptr::null_mut();
    // SAFETY: `engine` is a valid out pointer for the duration of this call.
    assert_eq!(
        unsafe { smk_engine_create_default(&mut engine) },
        SmkStatus::Ok
    );
    assert!(!engine.is_null());

    // SAFETY: `engine` is live and owned by this test.
    assert_eq!(
        unsafe { smk_engine_set_resolve_macos_alias(engine, 0) },
        SmkStatus::Ok
    );
    // SAFETY: `engine` is live and owned by this test.
    assert_eq!(
        unsafe { smk_engine_set_resolve_macos_alias(engine, 1) },
        SmkStatus::Ok
    );

    // SAFETY: `engine` was returned by `smk_engine_create_default` and has not been freed.
    unsafe {
        smk_engine_free(engine);
    }
}

#[test]
fn keeps_file_list_direct_children_contiguous() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join("Alpha").join("Nested")).expect("alpha dir");
    std::fs::create_dir_all(temp.path().join("Beta")).expect("beta dir");
    std::fs::create_dir_all(temp.path().join("Gamma")).expect("gamma dir");
    std::fs::write(
        temp.path().join("Alpha").join("Nested").join("Alpha.jsx"),
        "alert('alpha');",
    )
    .expect("alpha script");
    std::fs::write(temp.path().join("Beta").join("Beta.jsx"), "alert('beta');")
        .expect("beta script");
    std::fs::write(
        temp.path().join("Gamma").join("Gamma.jsx"),
        "alert('gamma');",
    )
    .expect("gamma script");

    let mut engine: *mut SmkEngine = ptr::null_mut();
    // SAFETY: `engine` is a valid out pointer for the duration of this call.
    assert_eq!(
        unsafe { smk_engine_create_default(&mut engine) },
        SmkStatus::Ok
    );

    let path = temp.path().to_string_lossy().into_owned();
    let path_slice = SmkUtf8Slice {
        ptr: path.as_ptr(),
        len: path.len(),
    };
    let mut scan_result: *mut SmkScanResult = ptr::null_mut();
    // SAFETY: `engine` is live, path slice is valid for the call, and `scan_result` is writable.
    assert_eq!(
        unsafe { smk_engine_scan_folders(engine, &path_slice, 1, 0, &mut scan_result) },
        SmkStatus::Ok
    );
    assert!(!scan_result.is_null());

    let mut file_lists = SmkFileListSnapshotSlice {
        ptr: ptr::null(),
        len: 0,
    };
    // SAFETY: `scan_result` is live and `file_lists` is a valid out pointer.
    assert_eq!(
        unsafe { smk_scan_result_file_lists(scan_result, &mut file_lists) },
        SmkStatus::Ok
    );
    assert_eq!(file_lists.len, 1);
    // SAFETY: `file_lists` is borrowed from `scan_result` and the result is still alive.
    let file_lists = unsafe { slice::from_raw_parts(file_lists.ptr, file_lists.len) };

    let mut file_entries = SmkFileEntrySlice {
        ptr: ptr::null(),
        len: 0,
    };
    // SAFETY: `scan_result` is live and `file_entries` is a valid out pointer.
    assert_eq!(
        unsafe { smk_scan_result_file_entries(scan_result, &mut file_entries) },
        SmkStatus::Ok
    );
    assert!(file_entries.len >= 3);
    // SAFETY: `file_entries` is borrowed from `scan_result` and the result is still alive.
    let file_entries = unsafe { slice::from_raw_parts(file_entries.ptr, file_entries.len) };
    let root = file_lists[0];
    let root_children =
        &file_entries[root.first_child_index..root.first_child_index + root.child_count];
    let root_child_names: Vec<_> = root_children
        .iter()
        .map(|entry| {
            std::path::Path::new(&utf8(entry.display_path))
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(root_child_names, ["Alpha", "Beta", "Gamma"]);

    let alpha = &root_children[0];
    let alpha_children =
        &file_entries[alpha.first_child_index..alpha.first_child_index + alpha.child_count];
    assert_eq!(alpha_children.len(), 1);
    assert!(utf8(alpha_children[0].display_path).ends_with("Alpha/Nested"));

    // SAFETY: both handles were returned by this FFI crate and have not been freed.
    unsafe {
        smk_scan_result_free(scan_result);
        smk_engine_free(engine);
    }
}

#[test]
fn returns_file_items_for_overlapping_registered_roots() {
    let temp = tempfile::tempdir().expect("tempdir");
    let parent = temp.path().join("Parent");
    let child = parent.join("JSX");
    std::fs::create_dir_all(&child).expect("child dir");
    std::fs::write(
        child.join("Shared.jsx"),
        r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.ffi-overlap
// Version: 1.0.0
// SCRIPTMETA-END
"#,
    )
    .expect("script");

    let mut engine: *mut SmkEngine = ptr::null_mut();
    // SAFETY: `engine` is a valid out pointer for the duration of this call.
    assert_eq!(
        unsafe { smk_engine_create_default(&mut engine) },
        SmkStatus::Ok
    );

    let parent_path = parent.to_string_lossy().into_owned();
    let child_path = child.to_string_lossy().into_owned();
    let path_slices = [
        SmkUtf8Slice {
            ptr: parent_path.as_ptr(),
            len: parent_path.len(),
        },
        SmkUtf8Slice {
            ptr: child_path.as_ptr(),
            len: child_path.len(),
        },
    ];
    let mut scan_result: *mut SmkScanResult = ptr::null_mut();
    // SAFETY: `engine` is live, path slices are valid for the call, and `scan_result` is writable.
    assert_eq!(
        unsafe {
            smk_engine_scan_folders(
                engine,
                path_slices.as_ptr(),
                path_slices.len(),
                0,
                &mut scan_result,
            )
        },
        SmkStatus::Ok
    );
    assert!(!scan_result.is_null());

    let mut all_items = SmkScriptItemSlice {
        ptr: ptr::null(),
        len: 0,
    };
    // SAFETY: `scan_result` is live and `all_items` is a valid out pointer.
    assert_eq!(
        unsafe { smk_scan_result_items(scan_result, &mut all_items) },
        SmkStatus::Ok
    );
    assert_eq!(all_items.len, 1);

    let mut file_items = SmkScriptItemSlice {
        ptr: ptr::null(),
        len: 0,
    };
    // SAFETY: `scan_result` is live and `file_items` is a valid out pointer.
    assert_eq!(
        unsafe { smk_scan_result_file_items(scan_result, &mut file_items) },
        SmkStatus::Ok
    );
    assert_eq!(file_items.len, 2);
    // SAFETY: `file_items` is borrowed from `scan_result` and the result is still alive.
    let file_items = unsafe { slice::from_raw_parts(file_items.ptr, file_items.len) };
    let root_ids: std::collections::BTreeSet<_> =
        file_items.iter().map(|item| utf8(item.root_id)).collect();
    assert_eq!(root_ids.len(), 2);

    // SAFETY: both handles were returned by this FFI crate and have not been freed.
    unsafe {
        smk_scan_result_free(scan_result);
        smk_engine_free(engine);
    }
}

#[test]
fn scans_updates_through_multi_folder_ffi() {
    let temp = tempfile::tempdir().expect("tempdir");
    let script_path = temp.path().join("Example.jsx");
    let dist_path = temp.path().join("SCRIPTMETA.txt");
    let dist_url = url::Url::from_file_path(&dist_path).expect("file url");
    std::fs::write(
        &script_path,
        format!(
            r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.ffi.update
// Version: 1.0.0
// Meta-URL: {dist_url}
// Name: FFI Update Example
// SCRIPTMETA-END
"#
        ),
    )
    .expect("script");
    std::fs::write(
        &dist_path,
        r#"
SCRIPTMETA-DIST-BEGIN
Script-ID: com.example.ffi.update
Latest-Version: 2.0.0
SCRIPTMETA-DIST-END
"#,
    )
    .expect("dist");

    let mut engine: *mut SmkEngine = ptr::null_mut();
    // SAFETY: `engine` is a valid out pointer for the duration of this call.
    assert_eq!(
        unsafe { smk_engine_create_default(&mut engine) },
        SmkStatus::Ok
    );

    let path = temp.path().to_string_lossy().into_owned();
    let path_slice = SmkUtf8Slice {
        ptr: path.as_ptr(),
        len: path.len(),
    };
    let mut scan_result: *mut SmkScanResult = ptr::null_mut();
    // SAFETY: `engine` is live, path slice is valid for the call, and `scan_result` is writable.
    assert_eq!(
        unsafe { smk_engine_scan_folders(engine, &path_slice, 1, 1, &mut scan_result) },
        SmkStatus::Ok
    );
    assert!(!scan_result.is_null());

    let mut update_info = SmkUpdateCheckInfo {
        has_update_check: 0,
        checked_at: 0,
    };
    // SAFETY: `scan_result` is live and `update_info` is a valid out pointer.
    assert_eq!(
        unsafe { smk_scan_result_update_info(scan_result, &mut update_info) },
        SmkStatus::Ok
    );
    assert_eq!(update_info.has_update_check, 1);

    let mut statuses = SmkUpdateStatusEntrySlice {
        ptr: ptr::null(),
        len: 0,
    };
    // SAFETY: `scan_result` is live and `statuses` is a valid out pointer.
    assert_eq!(
        unsafe { smk_scan_result_update_statuses(scan_result, &mut statuses) },
        SmkStatus::Ok
    );
    assert_eq!(statuses.len, 1);
    // SAFETY: `statuses` is borrowed from `scan_result` and the result is still alive.
    let statuses = unsafe { slice::from_raw_parts(statuses.ptr, statuses.len) };
    assert_eq!(utf8(statuses[0].status), "update_available");

    let mut resolutions = SmkDistributionResolutionEntrySlice {
        ptr: ptr::null(),
        len: 0,
    };
    // SAFETY: `scan_result` is live and `resolutions` is a valid out pointer.
    assert_eq!(
        unsafe { smk_scan_result_update_resolutions(scan_result, &mut resolutions) },
        SmkStatus::Ok
    );
    assert_eq!(resolutions.len, 1);
    // SAFETY: `resolutions` is borrowed from `scan_result` and the result is still alive.
    let resolutions = unsafe { slice::from_raw_parts(resolutions.ptr, resolutions.len) };
    assert_eq!(utf8(resolutions[0].latest_version), "2.0.0");

    // SAFETY: both handles were returned by this FFI crate and have not been freed.
    unsafe {
        smk_scan_result_free(scan_result);
        smk_engine_free(engine);
    }
}

#[test]
fn reports_update_progress_through_ffi_callback() {
    let temp = tempfile::tempdir().expect("tempdir");
    let script_path = temp.path().join("Example.jsx");
    let dist_path = temp.path().join("SCRIPTMETA.txt");
    let dist_url = url::Url::from_file_path(&dist_path).expect("file url");
    std::fs::write(
        &script_path,
        format!(
            r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.ffi.progress
// Version: 1.0.0
// Meta-URL: {dist_url}
// SCRIPTMETA-END
"#
        ),
    )
    .expect("script");
    std::fs::write(
        &dist_path,
        r#"
SCRIPTMETA-DIST-BEGIN
Script-ID: com.example.ffi.progress
Latest-Version: 2.0.0
SCRIPTMETA-DIST-END
"#,
    )
    .expect("dist");

    let mut engine: *mut SmkEngine = ptr::null_mut();
    // SAFETY: `engine` is a valid out pointer for the duration of this call.
    assert_eq!(
        unsafe { smk_engine_create_default(&mut engine) },
        SmkStatus::Ok
    );

    let path = temp.path().to_string_lossy().into_owned();
    let path_slice = SmkUtf8Slice {
        ptr: path.as_ptr(),
        len: path.len(),
    };
    let mut phases = Vec::new();
    let mut scan_result: *mut SmkScanResult = ptr::null_mut();
    // SAFETY: `engine` is live, path slice and callback context are valid for this call.
    assert_eq!(
        unsafe {
            smk_engine_scan_folders_with_progress(
                engine,
                &path_slice,
                1,
                1,
                Some(collect_progress_phase),
                (&mut phases as *mut Vec<String>).cast::<c_void>(),
                &mut scan_result,
            )
        },
        SmkStatus::Ok
    );
    assert!(!scan_result.is_null());
    assert!(phases.iter().any(|phase| phase == "started"));
    assert!(phases.iter().any(|phase| phase == "checking"));
    assert!(phases.iter().any(|phase| phase == "finished_item"));
    assert!(phases.iter().any(|phase| phase == "finished"));

    // SAFETY: both handles were returned by this FFI crate and have not been freed.
    unsafe {
        smk_scan_result_free(scan_result);
        smk_engine_free(engine);
    }
}

#[test]
fn reports_file_changes_through_reused_engine() {
    let temp = tempfile::tempdir().expect("tempdir");
    let removed_path = temp.path().join("Removed.jsx");
    let modified_path = temp.path().join("Modified.jsx");
    let added_path = temp.path().join("Added.jsx");
    std::fs::write(&removed_path, "alert('removed');").expect("removed");
    std::fs::write(&modified_path, "alert('before');").expect("modified before");

    let mut engine: *mut SmkEngine = ptr::null_mut();
    // SAFETY: `engine` is a valid out pointer for the duration of this call.
    assert_eq!(
        unsafe { smk_engine_create_default(&mut engine) },
        SmkStatus::Ok
    );

    let path = temp.path().to_string_lossy().into_owned();
    let path_slice = SmkUtf8Slice {
        ptr: path.as_ptr(),
        len: path.len(),
    };

    let mut first_result: *mut SmkScanResult = ptr::null_mut();
    // SAFETY: `engine` is live, path slice is valid for the call, and `first_result` is writable.
    assert_eq!(
        unsafe { smk_engine_scan_folders(engine, &path_slice, 1, 0, &mut first_result) },
        SmkStatus::Ok
    );
    assert!(!first_result.is_null());
    let mut first_change_info = SmkScanChangeInfo::default();
    // SAFETY: `first_result` is live and `first_change_info` is writable.
    assert_eq!(
        unsafe { smk_scan_result_change_info(first_result, &mut first_change_info) },
        SmkStatus::Ok
    );
    assert_eq!(first_change_info.has_change_summary, 0);
    // SAFETY: result handle was returned by this FFI crate and has not been freed.
    unsafe {
        smk_scan_result_free(first_result);
    }

    std::fs::remove_file(&removed_path).expect("remove");
    std::fs::write(&modified_path, "alert('after after');").expect("modified after");
    std::fs::write(&added_path, "alert('added');").expect("added");

    let mut second_result: *mut SmkScanResult = ptr::null_mut();
    // SAFETY: `engine` is live, path slice is valid for the call, and `second_result` is writable.
    assert_eq!(
        unsafe { smk_engine_scan_folders(engine, &path_slice, 1, 0, &mut second_result) },
        SmkStatus::Ok
    );
    assert!(!second_result.is_null());

    let mut change_info = SmkScanChangeInfo::default();
    // SAFETY: `second_result` is live and `change_info` is writable.
    assert_eq!(
        unsafe { smk_scan_result_change_info(second_result, &mut change_info) },
        SmkStatus::Ok
    );
    assert_eq!(change_info.has_change_summary, 1);
    assert_eq!(change_info.added_count, 1);
    assert_eq!(change_info.removed_count, 1);
    assert_eq!(change_info.modified_count, 1);

    let mut changes = SmkFileEntryChangeSlice {
        ptr: ptr::null(),
        len: 0,
    };
    // SAFETY: `second_result` is live and `changes` is writable.
    assert_eq!(
        unsafe { smk_scan_result_file_entry_changes(second_result, &mut changes) },
        SmkStatus::Ok
    );
    assert_eq!(changes.len, 3);
    // SAFETY: `changes` is borrowed from `second_result` and the result is still alive.
    let changes = unsafe { slice::from_raw_parts(changes.ptr, changes.len) };
    let kinds: std::collections::BTreeSet<_> =
        changes.iter().map(|change| utf8(change.kind)).collect();
    assert!(kinds.contains("added"));
    assert!(kinds.contains("removed"));
    assert!(kinds.contains("modified"));

    // SAFETY: both handles were returned by this FFI crate and have not been freed.
    unsafe {
        smk_scan_result_free(second_result);
        smk_engine_free(engine);
    }
}

#[test]
fn preserves_update_result_through_non_update_ffi_scan() {
    let temp = tempfile::tempdir().expect("tempdir");
    let dist_path = temp.path().join("SCRIPTMETA.txt");
    let dist_url = url::Url::from_file_path(&dist_path).expect("dist url");
    std::fs::write(
        &dist_path,
        r#"
SCRIPTMETA-DIST-BEGIN
Script-ID: com.example.ffi.update.cache
Latest-Version: 2.0.0
SCRIPTMETA-DIST-END
"#,
    )
    .expect("dist");
    std::fs::write(
        temp.path().join("Example.jsx"),
        format!(
            r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.ffi.update.cache
// Version: 1.0.0
// Meta-URL: {dist_url}
// SCRIPTMETA-END
"#
        ),
    )
    .expect("script");

    let mut engine: *mut SmkEngine = ptr::null_mut();
    // SAFETY: `engine` is a valid out pointer for the duration of this call.
    assert_eq!(
        unsafe { smk_engine_create_default(&mut engine) },
        SmkStatus::Ok
    );

    let path = temp.path().to_string_lossy().into_owned();
    let path_slice = SmkUtf8Slice {
        ptr: path.as_ptr(),
        len: path.len(),
    };

    let mut update_result: *mut SmkScanResult = ptr::null_mut();
    // SAFETY: `engine` is live, path slice is valid for the call, and `update_result` is writable.
    assert_eq!(
        unsafe { smk_engine_scan_folders(engine, &path_slice, 1, 1, &mut update_result) },
        SmkStatus::Ok
    );
    assert!(!update_result.is_null());
    let mut update_info = SmkUpdateCheckInfo::default();
    // SAFETY: `update_result` is live and `update_info` is writable.
    assert_eq!(
        unsafe { smk_scan_result_update_info(update_result, &mut update_info) },
        SmkStatus::Ok
    );
    assert_eq!(update_info.has_update_check, 1);
    // SAFETY: result handle was returned by this FFI crate and has not been freed.
    unsafe {
        smk_scan_result_free(update_result);
    }

    let mut scan_result: *mut SmkScanResult = ptr::null_mut();
    // SAFETY: `engine` is live, path slice is valid for the call, and `scan_result` is writable.
    assert_eq!(
        unsafe { smk_engine_scan_folders(engine, &path_slice, 1, 0, &mut scan_result) },
        SmkStatus::Ok
    );
    assert!(!scan_result.is_null());
    let mut preserved_info = SmkUpdateCheckInfo::default();
    // SAFETY: `scan_result` is live and `preserved_info` is writable.
    assert_eq!(
        unsafe { smk_scan_result_update_info(scan_result, &mut preserved_info) },
        SmkStatus::Ok
    );
    assert_eq!(preserved_info.has_update_check, 1);

    // SAFETY: handles were returned by this FFI crate and have not been freed.
    unsafe {
        smk_scan_result_free(scan_result);
        smk_engine_free(engine);
    }
}

#[cfg(feature = "native-watch")]
#[test]
fn starts_and_polls_native_watcher_through_ffi() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("Example.jsx"), "alert('ok');").expect("script");

    let mut engine: *mut SmkEngine = ptr::null_mut();
    // SAFETY: `engine` is a valid out pointer for the duration of this call.
    assert_eq!(
        unsafe { smk_engine_create_default(&mut engine) },
        SmkStatus::Ok
    );

    let path = temp.path().to_string_lossy().into_owned();
    let path_slice = SmkUtf8Slice {
        ptr: path.as_ptr(),
        len: path.len(),
    };
    let mut scan_result: *mut SmkScanResult = ptr::null_mut();
    // SAFETY: `engine` is live, path slice is valid for the call, and `scan_result` is writable.
    assert_eq!(
        unsafe { smk_engine_scan_folders(engine, &path_slice, 1, 0, &mut scan_result) },
        SmkStatus::Ok
    );
    assert!(!scan_result.is_null());
    // SAFETY: result handle was returned by this FFI crate and has not been freed.
    unsafe {
        smk_scan_result_free(scan_result);
    }

    // SAFETY: `engine` is live and already has roots from the scan above.
    assert_eq!(unsafe { smk_engine_start_watching(engine) }, SmkStatus::Ok);

    let mut changed = 1_u8;
    let mut changed_result: *mut SmkScanResult = ptr::null_mut();
    // SAFETY: `engine` is live and output pointers are writable.
    assert_eq!(
        unsafe { smk_engine_poll_watcher_scan(engine, &mut changed, &mut changed_result) },
        SmkStatus::Ok
    );
    assert_eq!(changed, 0);
    assert!(changed_result.is_null());

    // SAFETY: `engine` is live.
    assert_eq!(unsafe { smk_engine_stop_watching(engine) }, SmkStatus::Ok);

    // SAFETY: handle was returned by this FFI crate and has not been freed.
    unsafe {
        smk_engine_free(engine);
    }
}

#[cfg(feature = "native-watch")]
#[test]
fn notifies_when_native_watcher_receives_change() {
    extern "C" fn watch_callback(context: *mut c_void) {
        if context.is_null() {
            return;
        }
        // SAFETY: the test passes a live `AtomicUsize` pointer and keeps it
        // alive until after the watcher is stopped.
        let counter = unsafe { &*(context as *const AtomicUsize) };
        counter.fetch_add(1, Ordering::SeqCst);
    }

    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("Example.jsx"), "alert('ok');").expect("script");

    let mut engine: *mut SmkEngine = ptr::null_mut();
    // SAFETY: `engine` is a valid out pointer for the duration of this call.
    assert_eq!(
        unsafe { smk_engine_create_default(&mut engine) },
        SmkStatus::Ok
    );

    let path = temp.path().to_string_lossy().into_owned();
    let path_slice = SmkUtf8Slice {
        ptr: path.as_ptr(),
        len: path.len(),
    };
    let mut scan_result: *mut SmkScanResult = ptr::null_mut();
    // SAFETY: `engine` is live, path slice is valid for the call, and `scan_result` is writable.
    assert_eq!(
        unsafe { smk_engine_scan_folders(engine, &path_slice, 1, 0, &mut scan_result) },
        SmkStatus::Ok
    );
    assert!(!scan_result.is_null());
    // SAFETY: result handle was returned by this FFI crate and has not been freed.
    unsafe {
        smk_scan_result_free(scan_result);
    }

    let notification_count = AtomicUsize::new(0);
    // SAFETY: `engine` is live and already has roots from the scan above. The
    // context points to `notification_count`, which outlives the watcher.
    assert_eq!(
        unsafe {
            smk_engine_start_watching_with_callback(
                engine,
                Some(watch_callback),
                &notification_count as *const AtomicUsize as *mut c_void,
            )
        },
        SmkStatus::Ok
    );

    thread::sleep(Duration::from_millis(250));
    std::fs::write(temp.path().join("Added.jsx"), "alert('added');").expect("added");

    let deadline = Instant::now() + Duration::from_secs(5);
    while notification_count.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(50));
    }
    assert!(notification_count.load(Ordering::SeqCst) > 0);

    let mut changed = 0_u8;
    let mut changed_result: *mut SmkScanResult = ptr::null_mut();
    while changed == 0 && Instant::now() < deadline {
        // SAFETY: `engine` is live and output pointers are writable.
        assert_eq!(
            unsafe { smk_engine_poll_watcher_scan(engine, &mut changed, &mut changed_result) },
            SmkStatus::Ok
        );
        if changed == 0 {
            thread::sleep(Duration::from_millis(50));
        }
    }
    assert_eq!(changed, 1);
    assert!(!changed_result.is_null());

    let mut change_info = SmkScanChangeInfo::default();
    // SAFETY: `changed_result` is live and `change_info` is writable.
    assert_eq!(
        unsafe { smk_scan_result_change_info(changed_result, &mut change_info) },
        SmkStatus::Ok
    );
    assert_eq!(change_info.has_change_summary, 1);
    assert!(change_info.added_count >= 1);

    // SAFETY: handles were returned by this FFI crate and have not been freed.
    unsafe {
        smk_scan_result_free(changed_result);
        smk_engine_stop_watching(engine);
        smk_engine_free(engine);
    }
}

#[test]
fn stores_last_error_on_invalid_argument() {
    let mut engine: *mut SmkEngine = ptr::null_mut();
    // SAFETY: `engine` is a valid out pointer for the duration of this call.
    assert_eq!(
        unsafe { smk_engine_create_default(&mut engine) },
        SmkStatus::Ok
    );

    let mut scan_result: *mut SmkScanResult = ptr::null_mut();
    // SAFETY: empty input is allowed as a checked invalid argument, and `scan_result` is writable.
    assert_eq!(
        unsafe { smk_engine_scan_folder(engine, ptr::null(), 0, &mut scan_result) },
        SmkStatus::InvalidArgument
    );
    assert!(scan_result.is_null());

    let mut error = SmkUtf8Slice {
        ptr: ptr::null(),
        len: 0,
    };
    // SAFETY: `engine` is live and `error` is a valid out pointer.
    assert_eq!(
        unsafe { smk_engine_last_error(engine, &mut error) },
        SmkStatus::Ok
    );
    assert_eq!(utf8(error), "folder path is empty");

    // SAFETY: `engine` was returned by this FFI crate and has not been freed.
    unsafe {
        smk_engine_free(engine);
    }
}

fn utf8(value: SmkUtf8Slice) -> String {
    if value.ptr.is_null() || value.len == 0 {
        return String::new();
    }
    // SAFETY: tests only read slices returned by live FFI handles.
    let bytes = unsafe { slice::from_raw_parts(value.ptr, value.len) };
    String::from_utf8(bytes.to_vec()).expect("utf8")
}

extern "C" fn collect_progress_phase(progress: *const SmkUpdateProgress, context: *mut c_void) {
    if progress.is_null() || context.is_null() {
        return;
    }
    // SAFETY: the test passes a live `Vec<String>` context for the duration of the callback.
    let phases = unsafe { &mut *context.cast::<Vec<String>>() };
    // SAFETY: the callback receives a live progress pointer for this call.
    let progress = unsafe { &*progress };
    phases.push(utf8(progress.phase));
    assert!(progress.completed_items <= progress.total_items);
}
