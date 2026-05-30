#ifndef SCRIPTMETAKIT_FFI_H
#define SCRIPTMETAKIT_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum SmkStatus {
    SMK_STATUS_OK = 0,
    SMK_STATUS_NULL_ARGUMENT = 1,
    SMK_STATUS_INVALID_UTF8 = 2,
    SMK_STATUS_INVALID_ARGUMENT = 3,
    SMK_STATUS_ENGINE_ERROR = 4,
    SMK_STATUS_PANIC = 5,
} SmkStatus;

typedef struct SmkEngine SmkEngine;
typedef struct SmkScanResult SmkScanResult;

typedef struct SmkUtf8Slice {
    const uint8_t *ptr;
    size_t len;
} SmkUtf8Slice;

typedef struct SmkRootSnapshot {
    SmkUtf8Slice root_id;
    SmkUtf8Slice path;
    SmkUtf8Slice status;
    uint8_t is_dirty;
    uint8_t has_last_loaded_at;
    uint64_t last_loaded_at;
    uint8_t has_last_event_at;
    uint64_t last_event_at;
    size_t item_count;
    SmkUtf8Slice error_code;
    SmkUtf8Slice error_message;
} SmkRootSnapshot;

typedef struct SmkFileEntry {
    SmkUtf8Slice display_path;
    SmkUtf8Slice resolved_path;
    SmkUtf8Slice path_kind;
    SmkUtf8Slice resolution_status;
    SmkUtf8Slice resolution_message;
    uint8_t is_directory;
    uint8_t has_file_size;
    uint64_t file_size;
    uint8_t has_content_modified_at;
    uint64_t content_modified_at;
    SmkUtf8Slice runtime_kind;
    SmkUtf8Slice shebang;
    uint8_t has_scriptmeta;
    uint8_t has_scriptmeta_edit_password;
    uint8_t is_file_locked;
    uint8_t is_read_only;
    uint8_t can_edit_scriptmeta;
    uint8_t can_append_scriptmeta;
    SmkUtf8Slice scriptmeta_edit_state;
    size_t first_child_index;
    size_t child_count;
} SmkFileEntry;

typedef struct SmkFileListSnapshot {
    size_t root_index;
    size_t first_child_index;
    size_t child_count;
    uint8_t truncated;
} SmkFileListSnapshot;

typedef struct SmkScriptItem {
    SmkUtf8Slice root_id;
    SmkUtf8Slice file_path;
    SmkUtf8Slice identity_path;
    SmkUtf8Slice runtime_kind;
    SmkUtf8Slice shebang;
    SmkUtf8Slice script_id;
    SmkUtf8Slice version;
    SmkUtf8Slice name;
    SmkUtf8Slice description;
    SmkUtf8Slice target_app;
    SmkUtf8Slice meta_url;
    SmkUtf8Slice author;
    SmkUtf8Slice release_date;
    SmkUtf8Slice edit_password_sha256;
    uint8_t has_scriptmeta;
    uint8_t has_scriptmeta_edit_password;
    uint8_t is_file_locked;
    uint8_t is_read_only;
    uint8_t can_edit_scriptmeta;
    uint8_t can_append_scriptmeta;
    SmkUtf8Slice scriptmeta_edit_state;
} SmkScriptItem;

typedef struct SmkUpdateCheckInfo {
    uint8_t has_update_check;
    uint64_t checked_at;
} SmkUpdateCheckInfo;

typedef struct SmkUpdateStatusEntry {
    SmkUtf8Slice item_id;
    SmkUtf8Slice status;
} SmkUpdateStatusEntry;

typedef struct SmkDistributionResolutionEntry {
    SmkUtf8Slice item_id;
    SmkUtf8Slice latest_version;
    SmkUtf8Slice latest_page_url;
    SmkUtf8Slice final_page_url;
    size_t first_latest_url_history_index;
    size_t latest_url_history_count;
    uint64_t checked_at;
    uint8_t is_unresolved;
    SmkUtf8Slice note;
    uint8_t has_redirect_count;
    uint32_t redirect_count;
} SmkDistributionResolutionEntry;

typedef struct SmkUpdateFailureEntry {
    SmkUtf8Slice item_id;
    SmkUtf8Slice code;
    SmkUtf8Slice message;
    SmkUtf8Slice file_path;
    SmkUtf8Slice script_id;
    SmkUtf8Slice current_version;
    SmkUtf8Slice meta_url;
    SmkUtf8Slice source_url;
    uint64_t checked_at;
} SmkUpdateFailureEntry;

typedef struct SmkUpdateErrorEntry {
    SmkUtf8Slice item_id;
    SmkUtf8Slice message;
} SmkUpdateErrorEntry;

typedef struct SmkUpdateProgress {
    size_t completed_items;
    size_t total_items;
    SmkUtf8Slice item_id;
    SmkUtf8Slice script_id;
    SmkUtf8Slice phase;
    SmkUtf8Slice message;
} SmkUpdateProgress;

typedef void (*SmkUpdateProgressCallback)(
    const SmkUpdateProgress *progress,
    void *context
);
typedef void (*SmkWatchNotificationCallback)(void *context);

typedef struct SmkScanChangeInfo {
    uint8_t has_change_summary;
    size_t added_count;
    size_t removed_count;
    size_t modified_count;
} SmkScanChangeInfo;

typedef struct SmkFileEntryChange {
    SmkUtf8Slice root_id;
    SmkUtf8Slice kind;
    SmkUtf8Slice display_path;
    SmkUtf8Slice resolved_path;
    SmkUtf8Slice path_kind;
    SmkUtf8Slice resolution_status;
    SmkUtf8Slice resolution_message;
    uint8_t is_directory;
    uint8_t has_file_size;
    uint64_t file_size;
    uint8_t has_content_modified_at;
    uint64_t content_modified_at;
    SmkUtf8Slice runtime_kind;
    SmkUtf8Slice shebang;
    uint8_t has_scriptmeta;
    uint8_t has_scriptmeta_edit_password;
    uint8_t is_file_locked;
    uint8_t is_read_only;
    uint8_t can_edit_scriptmeta;
    uint8_t can_append_scriptmeta;
    SmkUtf8Slice scriptmeta_edit_state;
} SmkFileEntryChange;

typedef struct SmkRootSnapshotSlice {
    const SmkRootSnapshot *ptr;
    size_t len;
} SmkRootSnapshotSlice;

typedef struct SmkFileListSnapshotSlice {
    const SmkFileListSnapshot *ptr;
    size_t len;
} SmkFileListSnapshotSlice;

typedef struct SmkFileEntrySlice {
    const SmkFileEntry *ptr;
    size_t len;
} SmkFileEntrySlice;

typedef struct SmkScriptItemSlice {
    const SmkScriptItem *ptr;
    size_t len;
} SmkScriptItemSlice;

typedef struct SmkUpdateStatusEntrySlice {
    const SmkUpdateStatusEntry *ptr;
    size_t len;
} SmkUpdateStatusEntrySlice;

typedef struct SmkDistributionResolutionEntrySlice {
    const SmkDistributionResolutionEntry *ptr;
    size_t len;
} SmkDistributionResolutionEntrySlice;

typedef struct SmkUpdateFailureEntrySlice {
    const SmkUpdateFailureEntry *ptr;
    size_t len;
} SmkUpdateFailureEntrySlice;

typedef struct SmkUpdateErrorEntrySlice {
    const SmkUpdateErrorEntry *ptr;
    size_t len;
} SmkUpdateErrorEntrySlice;

typedef struct SmkUtf8SliceSlice {
    const SmkUtf8Slice *ptr;
    size_t len;
} SmkUtf8SliceSlice;

typedef struct SmkFileEntryChangeSlice {
    const SmkFileEntryChange *ptr;
    size_t len;
} SmkFileEntryChangeSlice;

SmkStatus smk_engine_create_default(SmkEngine **out_engine);
void smk_engine_free(SmkEngine *engine);

SmkStatus smk_engine_last_error(const SmkEngine *engine, SmkUtf8Slice *out_message);

SmkStatus smk_engine_set_resolve_macos_alias(SmkEngine *engine, uint8_t enabled);

SmkStatus smk_engine_scan_folder(
    SmkEngine *engine,
    const uint8_t *path_ptr,
    size_t path_len,
    SmkScanResult **out_result
);

SmkStatus smk_engine_scan_folders(
    SmkEngine *engine,
    const SmkUtf8Slice *paths_ptr,
    size_t path_count,
    uint8_t check_updates,
    SmkScanResult **out_result
);

SmkStatus smk_engine_scan_folders_with_progress(
    SmkEngine *engine,
    const SmkUtf8Slice *paths_ptr,
    size_t path_count,
    uint8_t check_updates,
    SmkUpdateProgressCallback progress_callback,
    void *progress_context,
    SmkScanResult **out_result
);

SmkStatus smk_engine_start_watching(SmkEngine *engine);
SmkStatus smk_engine_start_watching_with_callback(
    SmkEngine *engine,
    SmkWatchNotificationCallback callback,
    void *context
);
SmkStatus smk_engine_stop_watching(SmkEngine *engine);

SmkStatus smk_engine_poll_watcher_scan(
    SmkEngine *engine,
    uint8_t *out_changed,
    SmkScanResult **out_result
);

SmkStatus smk_scan_result_roots(
    const SmkScanResult *result,
    SmkRootSnapshotSlice *out_roots
);

SmkStatus smk_scan_result_file_lists(
    const SmkScanResult *result,
    SmkFileListSnapshotSlice *out_file_lists
);

SmkStatus smk_scan_result_file_entries(
    const SmkScanResult *result,
    SmkFileEntrySlice *out_file_entries
);

SmkStatus smk_scan_result_items(
    const SmkScanResult *result,
    SmkScriptItemSlice *out_items
);

SmkStatus smk_scan_result_file_items(
    const SmkScanResult *result,
    SmkScriptItemSlice *out_items
);

SmkStatus smk_scan_result_update_info(
    const SmkScanResult *result,
    SmkUpdateCheckInfo *out_info
);

SmkStatus smk_scan_result_update_statuses(
    const SmkScanResult *result,
    SmkUpdateStatusEntrySlice *out_statuses
);

SmkStatus smk_scan_result_update_resolutions(
    const SmkScanResult *result,
    SmkDistributionResolutionEntrySlice *out_resolutions
);

SmkStatus smk_scan_result_update_failures(
    const SmkScanResult *result,
    SmkUpdateFailureEntrySlice *out_failures
);

SmkStatus smk_scan_result_update_errors(
    const SmkScanResult *result,
    SmkUpdateErrorEntrySlice *out_errors
);

SmkStatus smk_scan_result_latest_url_history_urls(
    const SmkScanResult *result,
    SmkUtf8SliceSlice *out_urls
);

SmkStatus smk_scan_result_change_info(
    const SmkScanResult *result,
    SmkScanChangeInfo *out_info
);

SmkStatus smk_scan_result_file_entry_changes(
    const SmkScanResult *result,
    SmkFileEntryChangeSlice *out_changes
);

void smk_scan_result_free(SmkScanResult *result);

#ifdef __cplusplus
}
#endif

#endif
