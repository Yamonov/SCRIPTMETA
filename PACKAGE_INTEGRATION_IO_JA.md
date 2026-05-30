# SCRIPTMETAKit パッケージ組み込み用 入出力仕様

この文書は、SCRIPTMETAKit を Scripta と ACEMenu に Sparkle のような外部パッケージとして組み込むための、第一マイルストーンの入出力をまとめたものです。

第一マイルストーンでは、SCRIPTMETA の parser、root scan、file list snapshot、metadata catalog、update check、監視、cache 管理をパッケージ側に集めます。UI、通知表示、設定画面、UserDefaults、アプリ固有の表示ルールは各アプリ側に残します。

Phase 1 の public API と Rust module 構成は `PHASE1_PUBLIC_API_AND_MODULES_JA.md` に固定します。

## 参照したコード

Scripta:

- `/Users/yamo/Desktop/GIT/Scripta/Scripta/FileListMonitor.swift`
- `/Users/yamo/Desktop/GIT/Scripta/Scripta/FileListCacheStore.swift`
- `/Users/yamo/Desktop/GIT/Scripta/Scripta/FileListModels.swift`
- `/Users/yamo/Desktop/GIT/Scripta/Scripta/FileListSupport.swift`
- `/Users/yamo/Desktop/GIT/Scripta/Scripta/ScriptMetaSupport.swift`
- `/Users/yamo/Desktop/GIT/Scripta/Scripta/SettingsView.swift`
- `/Users/yamo/Desktop/GIT/Scripta/Scripta/EdgePanelSessionCoordinator.swift`

ACEMenu:

- `/Users/yamo/Desktop/GIT/ACEMenu/ACEmenu/ACEmenu/ScriptMetaModels.swift`
- `/Users/yamo/Desktop/GIT/ACEMenu/ACEmenu/ACEmenu/ScriptMetaParsing.swift`
- `/Users/yamo/Desktop/GIT/ACEMenu/ACEmenu/ACEmenu/ScriptMetaFetching.swift`
- `/Users/yamo/Desktop/GIT/ACEMenu/ACEmenu/ACEmenu/ScriptMetaService.swift`
- `/Users/yamo/Desktop/GIT/ACEMenu/ACEmenu/ACEmenu/ScriptUpdateModels.swift`
- `/Users/yamo/Desktop/GIT/ACEMenu/ACEmenu/ACEmenu/ScriptUpdateWorkflowSupport.swift`
- `/Users/yamo/Desktop/GIT/ACEMenu/ACEmenu/ACEmenu/ScriptUpdateSupport.swift`
- `/Users/yamo/Desktop/GIT/ACEMenu/ACEmenu/ACEmenu/ScriptUpdatePersistenceSupport.swift`
- `/Users/yamo/Desktop/GIT/ACEMenu/ACEmenu/ACEmenu/ScriptUpdateBackgroundSupport.swift`
- `/Users/yamo/Desktop/GIT/ACEMenu/ACEmenu/ACEmenu/ScriptUpdatePanelStoreWorkflowSupport.swift`
- `/Users/yamo/Desktop/GIT/ACEMenu/ACEmenu/ACEmenu/ACEFileMonitorService.swift`

## 現在の挙動

### Scripta

Scripta は、登録された script root を profile ごとに持っています。`EdgePanelSessionCoordinator` が全 profile の root を `FileListCacheStore` に登録し、表示中の root だけを visible root として扱います。

現在の `FileListCacheStore` は root 単位の memory cache を持ち、`Snapshot(children, isDirty, lastLoadedAt)` を返します。表示側は cache がある場合すぐ表示し、cache がない場合は空表示または loading 状態になります。

ファイル監視は `FileSystemMonitor` が担当しています。macOS では FSEvents を使い、複数 root URL を 1 つの stream に渡せる実装です。ただし現在の `FileListCacheStore.deduplicatedMonitorRoots()` は visible root だけを返しているため、実際に監視されるのは表示中 root だけです。

変更イベントは debounce されます。FSEvents 側で短い間隔の変更をまとめ、さらに main queue delivery でも 0.5 秒程度まとめます。path 数が上限を超えた場合は overflow として扱い、対象 root 全体を dirty にします。

リスト再作成は `FileSystemEntryLoader` が担当します。前回の `DirectoryState` と変更対象 directory を使い、変更がない directory は再利用します。深さ上限は 24、node 数上限は 20,000 です。memory cache 全体は非表示 root を中心に整理され、20 分未使用、または 60,000 node 超過で破棄されます。

SCRIPTMETA の metadata cache は `ScriptMetaCacheStore` と `ScriptMetaCacheRefreshService` が担当します。candidate cache を使い、file size と modification date が同じ file は再 parse しません。file list 側で SCRIPTMETA に関係する変更を検出した場合、root 単位で metadata cache refresh を予約します。

### ACEMenu

ACEMenu の SCRIPTMETA 機能は、登録済み folder path を scan し、`DiscoveredScript` の一覧を作り、Meta-URL ごとに update check を行います。

ACEMenu の登録 folder は現在、継続監視されていません。更新確認ボタン、panel attach 時の cache restore、24 時間間隔の automatic update check が主な入口です。

ACEMenu にある `ACEFileMonitorService` は、Adobe の ACE config file 1 件を監視するための service です。SCRIPTMETA 登録 folder の監視ではありません。SCRIPTMETAKit では、この single file monitor は第一マイルストーンの対象外にします。

ACEMenu の scan は `performScriptUpdateFolderScan(folderPaths:)` で行われます。各 folder を順番に enumerator で再帰 scan し、hidden file を除外します。SCRIPTMETAKit では ACEMenu 専用の拡張子定義は持たず、host app 共通の script 拡張子を使います。1 folder あたりの timeout は host app 側の設定で決めます。

cache は `ScriptUpdateCache/scripts.json` と `ScriptUpdateCache/results.json` に保存されます。scan が timed out した場合、または missing folder がないのに scripts が空になった場合は、automatic update check では既存 cached scripts を保持します。

## パッケージ境界

SCRIPTMETAKit が持つもの:

- SCRIPTMETA parser
- distribution metadata parser
- version compare
- Meta-URL / Latest-URL resolution
- recursive root scan
- file list snapshot 作成
- SCRIPTMETA catalog 作成
- candidate cache
- update check
- root 監視の抽象 API
- dirty root / dirty directory 管理
- cache の serialize / deserialize 用 model
- progress event model

アプリ側に残すもの:

- SwiftUI / AppKit UI
- NotificationCenter への投稿名
- user notification の表示
- UserDefaults key
- security scoped bookmark
- root 登録画面
- Scripta の profile 判定
- ACEMenu の menu bar badge / indicator
- Sparkle による app update
- script file の install / replace。現仕様では対象外とし、必要性は後で再検討します。
- Adobe app の起動状態や front app 判定

重要な方針:

- Rust core は NotificationCenter や AppKit に依存しません。
- Swift adapter は、Rust core の event を受けて各アプリの NotificationCenter や UI state に変換します。
- 監視するか、cache するか、自動 refresh するかはアプリ側が設定できます。

## アプリから渡す入力

### `ScriptMetaKitConfiguration`

パッケージ作成時に渡す基本設定です。

```text
app_id: String
cache_namespace: String
supported_extensions: Set<String>
parser: ParserOptions
scanner: ScannerOptions
watcher: WatcherOptions
cache: CacheOptions
update_check: UpdateCheckOptions
```

`app_id` と `cache_namespace` は、host app ごとの cache を混ぜないために必要です。

Illustrator / Photoshop / InDesign などの profile は Swift 側の設定で管理します。Rust core は profile を知らず、登録された root の file list と SCRIPTMETA 情報だけを返します。

### `RootRegistration`

監視、scan、cache の最小単位です。

```text
root_id: String
path: PathBuf
display_name: Option<String>
purpose: RootPurpose
watch_policy: WatchPolicy
cache_policy: CachePolicy
refresh_policy: RefreshPolicy
priority: RootPriority
```

`purpose`:

- `file_list`
- `metadata_catalog`
- `update_check`
- `file_list_and_metadata`

Scripta は `file_list_and_metadata` を使います。ACEMenu は基本的に `metadata_catalog` と `update_check` を使います。

アプリ側が安定した `root_id` を持つ場合は `set_roots` に明示的な `RootRegistration` を渡します。検証用 CLI や簡易 host では `set_root_paths` / `scan_root_paths` を使い、path から `root_id` を作ることもできます。この場合も結果は root 単位のまま保持します。

`watch_policy`:

- `disabled`
- `visible_only`
- `all_registered`
- `manual`

`cache_policy`:

- `disabled`
- `memory_only`
- `persistent_catalog_only`
- `memory_and_persistent`

`refresh_policy`:

- `manual_only`
- `on_visible`
- `on_file_event`
- `on_file_event_deferred`
- `scheduled`

これにより、アプリ側は「監視しない」「監視だけする」「cache だけ使う」「表示 root だけ即時 refresh」「全登録 root を監視して遅延 refresh」などを選べます。

### `ScannerOptions`

```text
max_depth: usize
max_nodes_per_root: usize
max_prefix_bytes: usize
skip_hidden: bool
skip_packages: bool
follow_symlinks: bool
resolve_macos_alias: bool
reuse_unchanged_records: bool
include_empty_directories: bool
scan_timeout_per_root: Option<Duration>
```

初期値:

- Scripta 互換の file list では `max_depth = 24`, `max_nodes_per_root = 20000`
- SCRIPTMETA parse は先頭 128 KiB
- hidden と package は除外
- file list では、表示対象 script file を含まない directory は既定で除外
- symlink は可能な範囲で identity path に反映
- macOS alias 解決は標準で有効。アプリ側の要求で `resolve_macos_alias = false` にできます。
- ACEMenu 互換の update scan では root ごとに 10 秒 timeout

### `WatcherOptions`

```text
enabled: bool
watch_policy: WatchPolicy
debounce_delay: Duration
max_delivery_delay: Duration
max_pending_paths: usize
overflow_policy: OverflowPolicy
monitor_root_strategy: MonitorRootStrategy
```

`monitor_root_strategy`:

- `exact_roots`
- `deduplicate_nested_roots`
- `platform_recommended`

第一マイルストーンでは `platform_recommended` を標準にします。

macOS では FSEvents が複数 root を 1 stream で監視できます。Windows では root ごとの watcher になる可能性があります。public API では platform の違いを見せず、`WatchPlan` と `RootChangeBatch` として返します。

### `CacheOptions`

```text
enabled: bool
memory_cache: bool
persistent_cache: bool
cache_directory: Option<PathBuf>
idle_lifetime: Duration
max_memory_nodes: usize
max_deferred_dirty_directories: usize
preserve_update_results: bool
```

初期値:

- Scripta file list memory cache は 20 分未使用で破棄
- Scripta file list memory cache は合計 60,000 node 超過で非表示 root から破棄
- deferred dirty directory は 128 件を超えると root 全体 dirty
- SCRIPTMETA catalog cache は file size と modification date で再利用
- update result は script id、version、meta URL が同じ場合だけ保持

### `UpdateCheckOptions`

```text
enabled: bool
max_concurrent_meta_url_checks: usize
request_timeout: Duration
resource_timeout: Duration
retry_policy: RetryPolicy
cache_network_responses: bool
```

初期値:

- max concurrent は 6
- ACEMenu 互換の request timeout は 15 秒
- Scripta 互換では Meta-URL ごとに group 化して check
- network response cache は使わない

## パッケージが返す出力

### `RootSnapshot`

root の現在状態です。

```text
root_id: String
path: PathBuf
status: RootStatus
is_dirty: bool
last_loaded_at: Option<DateTime>
last_event_at: Option<DateTime>
item_count: usize
error: Option<RootError>
```

`status`:

- `not_loaded`
- `ready`
- `dirty`
- `loading`
- `missing`
- `unreadable`
- `timed_out`
- `overflowed`

### `FileListSnapshot`

Scripta の file list 表示用です。

```text
root: RootSnapshot
children: Option<Vec<FileSystemEntry>>
directory_states: DirectoryStateMap
truncated: bool
```

`children` が `None` の場合は、まだ表示できる cache がありません。`children` がある場合は、dirty でも先に表示できます。各 `FileSystemEntry` は、file の場合に `runtime_kind` と `shebang` を持てます。

### `ScriptMetaCatalogSnapshot`

SCRIPTMETA 一覧と update check の基礎になる snapshot です。

```text
source_revision: Uuid
roots: Vec<RootSnapshot>
all_items: Vec<ScriptMetaItem>
file_items: Vec<ScriptMetaItem>
candidate_cache: CandidateCache
update_check_result: Option<UpdateCheckResult>
```

`all_items` は script id ごとに deduplicate した代表 item です。`file_items` は file list の path lookup 用に、file ごとの item を残します。各 `ScriptMetaItem` は `runtime_kind` と `shebang` を持てます。

### `RootChangeBatch`

監視から返す変更通知です。

```text
paths: Vec<PathBuf>
overflowed: bool
affected_roots: Vec<RootChange>
```

`RootChange`:

```text
root_id: String
dirty_directories: Vec<PathBuf>
metadata_may_have_changed: bool
requires_full_rescan: bool
```

overflow、root 自体の変更、dirty directory が多すぎる場合は `requires_full_rescan = true` です。

### `ScanResult`

手動 scan、初回 scan、file event 後の refresh に共通で使います。

```text
roots: Vec<RootSnapshot>
file_list_snapshots: Vec<FileListSnapshot>
catalog_snapshot: Option<ScriptMetaCatalogSnapshot>
events: Vec<ScriptMetaKitEvent>
```

### `UpdateCheckResult`

Scripta と ACEMenu の更新確認で共通利用します。

```text
checked_at: DateTime
resolutions_by_item_id: Map<String, DistributionResolution>
failures_by_item_id: Map<String, UpdateFailure>
errors_by_item_id: Map<String, String>
statuses_by_item_id: Map<String, UpdateStatus>
```

`failures_by_item_id` は更新確認が失敗した item の構造化された理由です。`errors_by_item_id` は既存実装との互換用に残します。
`DistributionResolution.latest_url_history` は `Latest-URL` の検出順を保持します。更新確認の UI では、ジャンプ先の履歴や循環・same-page 判定の説明に使えます。

```text
code: String
message: String
item_id: String
file_path: Path
script_id: String
current_version: Option<String>
meta_url: Option<URL>
source_url: Option<URL>
checked_at: DateTime
```

`UpdateStatus`:

- `idle`
- `checking`
- `up_to_date`
- `update_available`
- `failed`
- `not_checkable`

### `ScriptMetaKitEvent`

Swift adapter が NotificationCenter や UI state に変換する event です。

```text
root_registered
root_removed
watch_started
watch_stopped
change_detected
root_marked_dirty
scan_started
scan_finished
catalog_changed
file_list_changed
cache_loaded
cache_saved
cache_invalidated
update_check_started
update_check_progress
update_check_finished
root_missing
root_unreadable
scan_timed_out
watch_overflowed
```

## 監視の仕様

複数 root が渡された場合、パッケージは `WatchPlan` を作ります。

`WatchPlan` 作成ルール:

- path は標準化し、可能なら symlink を解決します。
- 同じ physical path は 1 件にまとめます。
- ある root が別の root の内側にある場合、recursive watch なら外側 root の watcher で内側 root の変更も検出できます。
- ただし、アプリが登録していない親 directory まで広げて監視することは標準では行いません。
- platform の制約で watcher 数を減らす必要がある場合だけ、adapter がより広い root を選べるようにします。
- event routing は logical root 単位で行います。physical watcher が 1 件でも、結果は root_id ごとに返します。

Scripta 用の初期設定:

- 表示速度を優先する場合は `watch_policy = visible_only`
- 登録 root の変更検出を優先する場合は `watch_policy = all_registered`
- file event 後は visible root を短い delay で refresh
- 非表示 root は dirty として記録し、必要な場合だけ background refresh

ACEMenu 用の初期設定:

- 現行互換は `watch_policy = disabled`, `refresh_policy = manual_only`
- 改善版は `watch_policy = all_registered`, `refresh_policy = on_file_event_deferred`
- folder 変更時は cached scripts をすぐ破棄せず、dirty として表示し、短い delay 後に scan
- update check は scan 完了後にアプリ側の設定に応じて実行

## リスト再作成の仕様

Scripta では file tree が必要です。ACEMenu では file tree は不要で、SCRIPTMETA のある script list が必要です。そのため scan request は mode を持ちます。

```text
ScanMode:
  file_list_only
  metadata_only
  file_list_and_metadata
```

`file_list_only`:

- directory と script file を含む tree を返します。
- hidden / package / Trash 内は除外します。
- 既定では、表示対象の script file を子孫に持たない directory は返しません。
- alias / symlink は display path と resolved path を分けて保持します。
- 前回の `DirectoryState` が使える場合、変更のない directory は再利用します。

`metadata_only`:

- 対応拡張子の file を再帰 scan します。
- 先頭 128 KiB から SCRIPTMETA を parse します。
- SCRIPTMETA がない file も candidate record として保存し、次回の再 parse を避けます。

`file_list_and_metadata`:

- file list を作った後、metadata に関係する変更がある場合だけ catalog refresh を予約できます。
- UI は file list を先に表示し、metadata は後から更新できます。

## cache を使うタイミング

cache を使う場面:

- アプリ起動直後に前回の catalog を表示する
- visible root を切り替えた直後に前回の file list を表示する
- file event 後、scan 完了まで古い list を表示する
- update check 結果を次回起動時に復元する
- 同じ file の SCRIPTMETA 再 parse を避ける
- 同じ directory の file list 再作成を避ける

cache を保持する場面:

- scan が timed out した
- root が一時的に missing / unreadable
- file event は来たが refresh がまだ終わっていない
- update check が失敗したが local script の identity は変わっていない

cache を破棄または無効化する場面:

- root registration が削除された
- root path が変わった
- supported_extensions が変わった
- parser schema version が変わった
- cache schema version が変わった
- root の recursive / hidden / package policy が変わった
- file event が overflow した
- dirty directory 数が上限を超えた
- root 自体が移動、削除、rename された
- item の `script_id`, `version`, `meta_url` のいずれかが変わった
- update check の対象でなくなった
- memory cache が idle lifetime を超えた
- memory cache が max nodes を超えた
- app が明示的に invalidate した
- permission または security scoped bookmark が失効した

## Scripta で必要な入出力

Scripta から渡す入力:

- 全 `ScriptsProfile` の登録 root
- 各 root の tab id または root id
- visible root
- foreground work を実行できるか
- file list が必要か
- SCRIPTMETA metadata を file list に表示するか
- 対応拡張子
- cache directory
- background refresh を許可するか
- app 側 NotificationCenter に変換する event sink

SCRIPTMETAKit から返す出力:

- root ごとの `FileListSnapshot`
- root ごとの dirty 状態
- `file_list_changed` event
- `ScriptMetaCatalogSnapshot`
- `catalog_changed` event
- update check progress
- `UpdateCheckResult`
- cache load / save / invalidate event
- root missing / unreadable / timed out event

Scripta の第一マイルストーンで置き換える候補:

- `FileSystemMonitor`
- `FileListCacheStore` の監視、dirty 管理、refresh scheduling
- `FileSystemEntryLoader`
- `ScriptMetaScanner`
- `ScriptMetaCacheStore`
- `ScriptMetaCacheRefreshService`
- `ScriptMetaUpdateChecker`

Scripta 側に残すもの:

- `EdgePanelController`
- `OutlineView` の表示構築
- favorite / shortcut / search / promoted group
- `ScriptsDirectoryAccess`
- `ForegroundWorkEligibility`
- NotificationCenter 名の互換 layer
- settings UI

## ACEMenu で必要な入出力

ACEMenu から渡す入力:

- 登録済み script update folder paths
- automatic update check が有効か
- last checked date
- scan timeout
- update check concurrency
- update notification を出すか
- cache directory
- watch を有効にするか
- watch 有効時に folder 変更で自動 scan するか
- app 側 indicator に変換する event sink

SCRIPTMETAKit から返す出力:

- `ScriptMetaCatalogSnapshot`
- `DiscoveredScript` 相当の item list
- missing paths
- timed out paths
- unreadable paths
- update check progress
- `UpdateCheckResult`
- item ごとの status
- available update count
- cache changed event

ACEMenu の第一マイルストーンで置き換える候補:

- `ScriptMetaService`
- `ScriptMetaParsing`
- `ScriptMetaFetching`
- `performScriptUpdateFolderScan`
- `resolveScriptUpdateStatuses`
- `ScriptUpdatePersistenceSupport` の cache model
- `ScriptUpdateBackgroundManager` の scan / resolve 実行部分

ACEMenu 側に残すもの:

- `ScriptUpdatePanelView`
- `ScriptUpdatePanelStore` の UI state
- registered folder の設定 UI
- menu bar indicator
- user notification
- 24 時間間隔の automatic update schedule
- Adobe ACE config file 用の `ACEFileMonitorService`

## public API 案

Swift 側からは、Sparkle の controller に近い形で扱います。

```swift
let controller = ScriptMetaKitController(configuration: configuration)
controller.delegate = self
controller.setRoots(rootRegistrations)
controller.startMonitoring()
```

必要な API:

```text
configure(configuration)
set_roots(root_registrations)
get_roots()
start_monitoring()
stop_monitoring()
watch_plan()
snapshot(root_id)
catalog_snapshot()
scan_roots(request)
refresh_dirty_roots(request)
check_updates(items, options)
load_cache()
save_cache(snapshot)
invalidate_cache(scope)
shutdown()
```

callback / delegate:

```text
on_event(ScriptMetaKitEvent)
on_file_list_changed(root_id, FileListSnapshot)
on_catalog_changed(ScriptMetaCatalogSnapshot)
on_update_progress(ProgressUpdate)
on_update_finished(UpdateCheckResult)
on_error(ScriptMetaKitError)
```

## 第一マイルストーンの確定事項

- SCRIPTMETAKit は、監視するかどうかをアプリ側の `WatcherOptions` で決められるようにします。
- cache するかどうかをアプリ側の `CacheOptions` で決められるようにします。
- scan mode は `file_list_only`, `metadata_only`, `file_list_and_metadata` の 3 種類を用意します。
- root は `root_id` と path だけで識別し、profile との対応は Swift 側で管理します。
- 監視は physical watcher と logical root を分けます。複数 folder は platform に合わせてまとめ、event は root_id ごとに返します。
- overflow や too many dirty directories は root 全体 dirty として扱います。
- fast response は cache snapshot を返し、同時に dirty / stale 状態を返します。
- cache 破棄は app が明示でき、schema、root、parser option、extension、permission、overflow でも自動 invalidation します。
- update check は `version` と `meta_url` がある item だけを対象にします。
- update result は `script_id`, `version`, `meta_url` が一致する間だけ保持します。
- script file の replace は現仕様では実装しません。将来必要になった場合だけ、別途仕様を見直します。
- Rust core は UI と NotificationCenter に依存しません。
- Swift adapter が Scripta / ACEMenu の既存 UI と通知へ接続します。
- CLI / JSON 接続は検証用です。本格組み込みでは `scriptmetakit_ffi` を別 crate として作り、opaque handle と `#[repr(C)]` の C ABI で Swift adapter へ接続します。
- FFI では Rust の `String`, `Vec`, `PathBuf`, Rust enum, `Option<T>` を直接公開せず、文字列と大きな配列は `ptr + len` で渡します。
- Rust 側で確保した snapshot は Rust 側の free 関数で解放します。Swift 側が borrowed pointer を長期保持しないようにします。
- `scriptmetakit_ffi` は `panic = "abort"` を指定し、公開 FFI 関数から panic を ABI 境界外へ出しません。
- Windows でも FFI 方針は同じです。配布物は `scriptmetakit_ffi.dll`、C header、必要な import library とし、.NET / C# からは P/Invoke、C++ からは C header、Swift on Windows からは C module として使える形にします。
- OS 固有の path / watcher / script source 抽出差分は platform adapter 内に閉じ込め、public API は macOS と Windows で同じ root / snapshot / event model を保ちます。
- macOS adapter は `.scpt` の `osadecompile` / OSA API、alias file 解決、FSEvents を担当します。
- Windows adapter は VBA / Office macro project からの module text 抽出、`.lnk` 解決、ReadDirectoryChangesW を担当します。
- platform adapter は SCRIPTMETA を含む text と diagnostics を core に渡し、core は parse / cache / update check を共通処理します。

## 第一マイルストーンの初期値

実装開始時の標準設定は、既存アプリの挙動を壊さないように現行互換を優先します。

Scripta:

- `watch_policy = visible_only`
- `cache_policy = memory_and_persistent`
- `refresh_policy = on_visible`
- `resolve_macos_alias = true`
- app 側の要求で `all_registered` と `on_file_event_deferred` に変更可能にします。

ACEMenu:

- `watch_policy = disabled`
- `cache_policy = persistent_catalog_only`
- `refresh_policy = manual_only`
- `resolve_macos_alias = true`
- automatic update check はアプリ側の 24 時間 schedule を維持します。
- app 側の要求で `all_registered` と `on_file_event_deferred` に変更可能にします。

この初期値により、パッケージ導入直後は既存挙動を維持し、監視範囲や cache 方針はアプリごとの設定で段階的に変更できます。
