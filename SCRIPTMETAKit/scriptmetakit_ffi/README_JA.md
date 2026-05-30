# scriptmetakit_ffi

`scriptmetakit_ffi` は、SCRIPTMETAKit core を C ABI で呼ぶための crate です。
Swift 側のテストアプリは CLI / JSON ではなく、この FFI を直接呼びます。

## 方針

- core crate の `unsafe_code = "forbid"` は維持する
- FFI 専用 crate だけで raw pointer を扱う
- engine と scan result は opaque handle として返す
- Swift / C 側へは `#[repr(C)]` の struct だけを見せる
- 大きい配列は `ptr + len` の borrowed slice で返す
- borrowed pointer は owner handle を free するまでだけ有効
- Rust 側で確保した handle は Rust 側の free 関数で解放する

## 現在の API

- `smk_engine_create_default`
- `smk_engine_free`
- `smk_engine_set_resolve_macos_alias`
- `smk_engine_scan_folder`
- `smk_engine_scan_folders`
- `smk_scan_result_roots`
- `smk_scan_result_file_lists`
- `smk_scan_result_file_entries`
- `smk_scan_result_items`
- `smk_scan_result_file_items`
- `smk_scan_result_update_info`
- `smk_scan_result_update_statuses`
- `smk_scan_result_update_resolutions`
- `smk_scan_result_update_failures`
- `smk_scan_result_update_errors`
- `smk_scan_result_latest_url_history_urls`
- `smk_scan_result_free`
- `smk_engine_last_error`

macOS Finderエイリアスの解決は標準で有効です。アプリ側で不要な場合は `smk_engine_set_resolve_macos_alias(engine, 0)` で無効化できます。

`smk_scan_result_items` は Script-ID ごとに重複排除した代表 item を返します。登録 root ごとの表示には `smk_scan_result_file_items` を使います。

`SmkScriptItem`、`SmkFileEntry`、`SmkFileEntryChange` は、Scripta のファイルリスト右クリックと同じ用途で使える編集可否情報を返します。

- `has_scriptmeta`: SCRIPTMETA ブロックを持つ
- `has_scriptmeta_edit_password`: `Edit-Password-SHA256` を持つ
- `is_file_locked`: macOS の locked / append-only フラグを検出した
- `is_read_only`: ファイルまたは親フォルダへ書き込めない
- `can_edit_scriptmeta`: 既存 SCRIPTMETA を編集できる
- `can_append_scriptmeta`: SCRIPTMETA を追記できる
- `scriptmeta_edit_state`: `unknown` / `unsupported` / `obfuscated` / `read_only` / `appendable` / `editable`

`.jsxbin` と、先頭が `@JSXBIN` の `.js` / `.jsx` は `obfuscated` として扱います。

各 list API は `ptr + len` の borrowed slice を返します。各文字列は `SmkUtf8Slice { ptr, len }` です。null terminated ではありません。

Swift 側では borrowed slice を必要な時点で `String` へ copy します。borrowed pointer は owner の `SmkScanResult` を `smk_scan_result_free` するまで有効です。

```swift
func string(from slice: SmkUtf8Slice) -> String {
    guard let ptr = slice.ptr, slice.len > 0 else { return "" }
    let buffer = UnsafeBufferPointer(start: ptr, count: slice.len)
    return String(decoding: buffer, as: UTF8.self)
}
```

## 確認コマンド

```sh
cargo test -p scriptmetakit_ffi
cargo build -p scriptmetakit_ffi --features blocking-http --release
```

header は `include/scriptmetakit_ffi.h` にあります。
