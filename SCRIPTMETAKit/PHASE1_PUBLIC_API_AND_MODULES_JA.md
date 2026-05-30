# Phase 1 Public API と Rust Module 構成

この文書は、SCRIPTMETAKit を Scripta と ACEMenu に外部パッケージとして組み込むための Phase 1 計画です。

Phase 1 の目的は、先に実装量を増やすことではなく、Swift アプリ側から見える API、Rust core の責務、cache と監視の制御点を固定することです。

## Phase 1 の成果物

Phase 1 で作るもの:

- Rust crate の module 構成
- Rust core の public model
- Swift adapter の public API 仕様
- Scripta 用の初期 configuration
- ACEMenu 用の初期 configuration
- cache / watcher / scan の policy model
- parser と version compare の最小テスト
- scan result と event model の fixture test

Phase 1 でまだ作らないもの:

- Scripta 本体への差し替え実装
- ACEMenu 本体への差し替え実装
- script file の download / replace。現仕様では対象外とし、必要性は後で再検討します。
- Sparkle 連携
- UI
- NotificationCenter への直接投稿
- macOS alias 解決の完全実装
- Windows watcher の実装完了

## 全体構成

最初は 3 層に分けます。

```text
Scripta / ACEMenu
  |
  | Swift typed API
  v
ScriptMetaKitSwift
  |
  | FFI / bridge
  v
scriptmetakit
```

`scriptmetakit` は純粋な Rust crate です。ここは `unsafe_code = "forbid"` を維持します。

`ScriptMetaKitSwift` は Swift 側の adapter です。Scripta と ACEMenu はこの adapter を Sparkle の controller のように保持します。

FFI が必要になった時点で、`scriptmetakit_ffi` のような小さい bridge crate を別に作ります。unsafe が必要な場合も core crate には入れません。

## Rust module 構成

Phase 1 の module は次の構成にします。

```text
src/
  lib.rs
  core/
    mod.rs
    metadata.rs
    distribution.rs
    parser.rs
    text.rs
    version.rs
    url.rs
    error.rs
  scanner/
    mod.rs
    options.rs
    root.rs
    walk.rs
    file_list.rs
    metadata_scan.rs
    identity.rs
  catalog/
    mod.rs
    config.rs
    root_registry.rs
    snapshot.rs
    dirty.rs
    cache.rs
    event.rs
  resolver/
    mod.rs
    source.rs
    fetch.rs
    latest.rs
    update_check.rs
    progress.rs
  watcher/
    mod.rs
    plan.rs
    event.rs
    policy.rs
  storage/
    mod.rs
    schema.rs
    codec.rs
  platform/
    mod.rs
    macos.rs
    windows.rs
```

### `core`

SCRIPTMETA 仕様そのものを扱います。

責務:

- script 側 SCRIPTMETA parser
- distribution 側 SCRIPTMETA parser
- text decoding
- known key / marker
- version compare
- URL 正規化
- validation
- 共通 error

ここには file system scan、cache、watcher、network を入れません。

### `scanner`

local file system から対象 script を探します。

責務:

- root scan
- hidden / package skip
- supported extension 判定
- symlink-aware identity path
- metadata candidate record 作成
- file list tree 作成
- directory state 作成
- 差分 scan

`scanner` は単発の scan を担当します。複数 root の状態管理は `catalog` が持ちます。

### `catalog`

アプリが使う stateful controller の core です。

責務:

- configuration 保持
- root registration 管理
- root ごとの snapshot
- dirty root / dirty directory 管理
- cache policy 適用
- scan scheduling の判断
- event 作成

`catalog` は直接 UI を更新しません。結果は `ScriptMetaKitEvent` と snapshot で返します。

### `resolver`

Meta-URL と distribution update check を扱います。

責務:

- Meta-URL source 判定
- generic text source 読み込み
- gist / GitHub raw source 読み込み
- Latest-URL 追跡
- update check
- progress event
- retry
- timeout

network 実装は後で差し替えできるように、fetcher trait を挟みます。

### `watcher`

監視 API の抽象層です。

責務:

- logical root から physical watch plan を作る
- nested root / duplicate root を整理する
- changed path を root_id に戻す
- overflow を表現する
- platform 固有 watcher の event を共通 event に変換する

Phase 1 では watch plan と event routing を先に実装します。platform watcher の完全実装は Phase 2 に回せます。

### `storage`

cache の永続化形式を扱います。

責務:

- schema version
- JSON 変換
- snapshot encode / decode
- cache compatibility check

実際にどの folder に保存するかはアプリ側が決めます。Rust core は path を渡された場合だけ読み書きできます。

### `platform`

OS 固有処理を分ける場所です。

Phase 1 の扱い:

- macOS alias 解決は標準で有効にし、`ScannerOptions.resolve_macos_alias` で無効化できます。
- Windows path 比較は helper を用意します。
- watcher の OS 実装は feature flag で分けられる形にします。

## Rust public model

### `ScriptMetaKitConfig`

```rust
pub struct ScriptMetaKitConfig {
    pub app_id: String,
    pub cache_namespace: String,
    pub supported_extensions: ExtensionPolicy,
    pub parser: ParserOptions,
    pub scanner: ScannerOptions,
    pub watcher: WatcherOptions,
    pub cache: CacheOptions,
    pub update_check: UpdateCheckOptions,
}
```

profile は Swift 側の設定で管理します。Rust core は profile を知らず、root と path を基準に file list と SCRIPTMETA 情報を返します。

### `RootRegistration`

```rust
pub struct RootRegistration {
    pub root_id: RootId,
    pub path: PathBuf,
    pub display_name: Option<String>,
    pub purpose: RootPurpose,
    pub watch_policy: WatchPolicy,
    pub cache_policy: CachePolicy,
    pub refresh_policy: RefreshPolicy,
    pub priority: RootPriority,
}
```

`root_id` は app 側が安定して持つ ID です。path だけを ID にしないことで、tab rename や表示名変更に対応できます。

### `RootPurpose`

```rust
pub enum RootPurpose {
    FileList,
    MetadataCatalog,
    UpdateCheck,
    FileListAndMetadata,
}
```

### `WatchPolicy`

```rust
pub enum WatchPolicy {
    Disabled,
    VisibleOnly,
    AllRegistered,
    Manual,
}
```

### `CachePolicy`

```rust
pub enum CachePolicy {
    Disabled,
    MemoryOnly,
    PersistentCatalogOnly,
    MemoryAndPersistent,
}
```

### `RefreshPolicy`

```rust
pub enum RefreshPolicy {
    ManualOnly,
    OnVisible,
    OnFileEvent,
    OnFileEventDeferred,
    Scheduled,
}
```

### `ScanMode`

```rust
pub enum ScanMode {
    FileListOnly,
    MetadataOnly,
    FileListAndMetadata,
}
```

### `ScriptMetaItem`

```rust
pub struct ScriptMetaItem {
    pub root_id: RootId,
    pub file_path: PathBuf,
    pub identity_path: PathBuf,
    pub runtime_kind: Option<ScriptRuntimeKind>,
    pub shebang: Option<String>,
    pub script_id: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub target_app: Option<String>,
    pub meta_url: Option<Url>,
    pub name: Option<String>,
    pub author: Option<String>,
    pub release_date: Option<String>,
    pub edit_password_sha256: Option<String>,
}
```

`name` は SCRIPTMETA の `Name` tag の値として保持します。file name fallback は app 側で決めます。

### `FileSystemEntry`

```rust
pub struct FileSystemEntry {
    pub display_path: PathBuf,
    pub resolved_path: PathBuf,
    pub is_directory: bool,
    pub runtime_kind: Option<ScriptRuntimeKind>,
    pub shebang: Option<String>,
    pub children: Vec<FileSystemEntry>,
}
```

Scripta の file list 互換用です。ACEMenu は基本的に使いません。

### `RootSnapshot`

```rust
pub struct RootSnapshot {
    pub root_id: RootId,
    pub path: PathBuf,
    pub status: RootStatus,
    pub is_dirty: bool,
    pub last_loaded_at: Option<DateTime>,
    pub last_event_at: Option<DateTime>,
    pub item_count: usize,
    pub error: Option<RootError>,
}
```

### `FileListSnapshot`

```rust
pub struct FileListSnapshot {
    pub root: RootSnapshot,
    pub children: Option<Vec<FileSystemEntry>>,
    pub directory_states: DirectoryStateMap,
    pub truncated: bool,
}
```

`children = None` は、まだ表示可能な cache がない状態です。

### `ScriptMetaCatalogSnapshot`

```rust
pub struct ScriptMetaCatalogSnapshot {
    pub source_revision: Uuid,
    pub roots: Vec<RootSnapshot>,
    pub all_items: Vec<ScriptMetaItem>,
    pub file_items: Vec<ScriptMetaItem>,
    pub candidate_cache: CandidateCache,
    pub update_check_result: Option<UpdateCheckResult>,
}
```

`all_items` は script id ごとの代表 item です。`file_items` は path lookup 用に file ごとの item を保持します。

### `UpdateCheckResult`

```rust
pub struct UpdateCheckResult {
    pub checked_at: DateTime,
    pub resolutions_by_item_id: BTreeMap<ItemId, DistributionResolution>,
    pub failures_by_item_id: BTreeMap<ItemId, UpdateFailure>,
    pub errors_by_item_id: BTreeMap<ItemId, String>,
    pub statuses_by_item_id: BTreeMap<ItemId, UpdateStatus>,
}
```

`UpdateFailure` は `code`, `message`, `item_id`, `file_path`, `script_id`, `current_version`, `meta_url`, `source_url`, `checked_at` を持ちます。`errors_by_item_id` は互換用です。

### `ScriptMetaKitEvent`

```rust
pub enum ScriptMetaKitEvent {
    RootRegistered { root_id: RootId },
    RootRemoved { root_id: RootId },
    WatchStarted { plan: WatchPlan },
    WatchStopped,
    ChangeDetected { batch: RootChangeBatch },
    RootMarkedDirty { root_id: RootId },
    ScanStarted { root_ids: Vec<RootId>, mode: ScanMode },
    ScanFinished { result: ScanResult },
    FileListChanged { root_id: RootId },
    CatalogChanged { source_revision: Uuid },
    CacheLoaded { scope: CacheScope },
    CacheSaved { scope: CacheScope },
    CacheInvalidated { scope: CacheScope, reason: CacheInvalidationReason },
    UpdateCheckStarted { item_count: usize },
    UpdateCheckProgress { progress: ProgressUpdate },
    UpdateCheckFinished { result: UpdateCheckResult },
    RootMissing { root_id: RootId },
    RootUnreadable { root_id: RootId },
    ScanTimedOut { root_id: RootId },
    WatchOverflowed { affected_roots: Vec<RootId> },
}
```

## Rust controller API

Rust core には stateful な `ScriptMetaKitEngine` を置きます。

```rust
pub struct ScriptMetaKitEngine {
    // private
}

impl ScriptMetaKitEngine {
    pub fn new(config: ScriptMetaKitConfig) -> Result<Self, ScriptMetaKitError>;
    pub fn set_roots(&mut self, roots: Vec<RootRegistration>) -> Result<Vec<ScriptMetaKitEvent>, ScriptMetaKitError>;
    pub fn set_root_paths(&mut self, paths: impl IntoIterator<Item = impl Into<PathBuf>>, purpose: RootPurpose) -> Result<Vec<ScriptMetaKitEvent>, ScriptMetaKitError>;
    pub fn roots(&self) -> &[RootRegistration];
    pub fn watch_plan(&self) -> WatchPlan;
    pub fn snapshot(&self, root_id: &RootId) -> Option<FileListSnapshot>;
    pub fn catalog_snapshot(&self) -> Option<ScriptMetaCatalogSnapshot>;
    pub fn scan_roots(&mut self, request: ScanRequest) -> Result<ScanResult, ScriptMetaKitError>;
    pub fn scan_root_paths(&mut self, paths: impl IntoIterator<Item = impl Into<PathBuf>>, mode: ScanMode) -> Result<ScanResult, ScriptMetaKitError>;
    pub fn mark_changed_paths(&mut self, batch: RawChangeBatch) -> Result<Vec<ScriptMetaKitEvent>, ScriptMetaKitError>;
    pub fn refresh_dirty_roots(&mut self, request: RefreshRequest) -> Result<ScanResult, ScriptMetaKitError>;
    pub async fn check_updates(&mut self, request: UpdateCheckRequest) -> Result<UpdateCheckResult, ScriptMetaKitError>;
    pub fn load_cache(&mut self, payload: CachePayload) -> Result<Vec<ScriptMetaKitEvent>, ScriptMetaKitError>;
    pub fn export_cache(&self, scope: CacheScope) -> Result<CachePayload, ScriptMetaKitError>;
    pub fn invalidate_cache(&mut self, scope: CacheScope, reason: CacheInvalidationReason) -> Vec<ScriptMetaKitEvent>;
}
```

Phase 1 では、watcher の OS event loop は engine に直接持たせません。まず `mark_changed_paths()` に raw event を渡して dirty state と event routing を検証します。

## Swift adapter public API

Swift アプリ側には `ScriptMetaKitController` を公開します。

```swift
public final class ScriptMetaKitController {
    public weak var delegate: ScriptMetaKitControllerDelegate?

    public init(configuration: ScriptMetaKitConfiguration)

    public func setRoots(_ roots: [ScriptMetaKitRootRegistration])
    public func roots() -> [ScriptMetaKitRootRegistration]

    public func startMonitoring()
    public func stopMonitoring()

    public func snapshot(rootID: ScriptMetaKitRootID) -> ScriptMetaKitFileListSnapshot?
    public func catalogSnapshot() -> ScriptMetaKitCatalogSnapshot?

    public func scanRoots(_ request: ScriptMetaKitScanRequest) async throws -> ScriptMetaKitScanResult
    public func refreshDirtyRoots(_ request: ScriptMetaKitRefreshRequest) async throws -> ScriptMetaKitScanResult
    public func checkUpdates(_ request: ScriptMetaKitUpdateCheckRequest) async throws -> ScriptMetaKitUpdateCheckResult

    public func loadCache(_ payload: ScriptMetaKitCachePayload) throws
    public func exportCache(scope: ScriptMetaKitCacheScope) throws -> ScriptMetaKitCachePayload
    public func invalidateCache(scope: ScriptMetaKitCacheScope, reason: ScriptMetaKitCacheInvalidationReason)

    public func shutdown()
}
```

Delegate:

```swift
public protocol ScriptMetaKitControllerDelegate: AnyObject {
    func scriptMetaKit(_ controller: ScriptMetaKitController, didEmit event: ScriptMetaKitEvent)
    func scriptMetaKit(_ controller: ScriptMetaKitController, fileListDidChange rootID: ScriptMetaKitRootID)
    func scriptMetaKit(_ controller: ScriptMetaKitController, catalogDidChange snapshot: ScriptMetaKitCatalogSnapshot)
    func scriptMetaKit(_ controller: ScriptMetaKitController, updateProgressDidChange progress: ScriptMetaKitProgressUpdate)
    func scriptMetaKit(_ controller: ScriptMetaKitController, didFail error: ScriptMetaKitError)
}
```

Swift adapter は NotificationCenter に直接投稿しません。Scripta / ACEMenu の互換層が delegate event を受けて既存の通知名へ変換します。

## Swift と Rust の bridge 方針

Phase 1 では、Rust core の typed API と Swift adapter 側の型名、field 名を先に固定します。

現在の CLI / JSON 接続は、挙動確認と fixture 比較のための仮接続です。本格的に Scripta / ACEMenu へ組み込む段階では、JSON payload を主経路にせず、opaque handle と C ABI の typed bridge に切り替えます。

本実装の構成:

1. `scriptmetakit` は Rust core のまま保ち、`unsafe_code = "forbid"` を維持します。
2. FFI は `scriptmetakit_ffi` のような別 crate に分けます。
3. `scriptmetakit_ffi` は `cdylib` / static library から `xcframework` を作れる形にします。
4. Swift package は `binaryTarget` で `scriptmetakit_ffi.xcframework` を同梱し、Swift adapter はそこだけを呼びます。
5. Scripta / ACEMenu は Sparkle と同じように `ScriptMetaKitController` を保持し、Rust FFI を直接触りません。

FFI の公開型は、Rust 内部型をそのまま出さず、C ABI 用の薄い型を別に定義します。

- `#[repr(C)]` を付けた構造体だけを Swift から読ませます。
- `String`, `Vec`, `PathBuf`, Rust enum, `Option<T>` は C ABI に直接出しません。
- 文字列は UTF-8 の `ptr + len` として渡します。
- 大きな file list / metadata list は `ptr + len` の slice として渡します。
- Rust enum は `u32` などの integer tag に変換します。
- nullable は null pointer、または `has_value` field で表現します。

基本形:

```c
typedef struct {
    const uint8_t *ptr;
    uintptr_t len;
} SMKBytes;

typedef struct {
    const SMKFileEntry *ptr;
    uintptr_t len;
} SMKFileEntrySlice;

typedef struct SMKEngine SMKEngine;
typedef struct SMKSnapshot SMKSnapshot;

int32_t smk_engine_new(SMKConfig config, SMKEngine **out_engine);
void smk_engine_free(SMKEngine *engine);

int32_t smk_scan_roots(SMKEngine *engine, SMKScanRequest request, SMKSnapshot **out_snapshot);
void smk_snapshot_free(SMKSnapshot *snapshot);

SMKFileEntrySlice smk_snapshot_file_entries(const SMKSnapshot *snapshot);
SMKScriptMetaItemSlice smk_snapshot_meta_items(const SMKSnapshot *snapshot);
SMKError smk_last_error(void);
```

所有権:

- `SMKEngine` と `SMKSnapshot` は opaque handle とし、生成と解放を Rust 側の関数で行います。
- Swift が受け取る slice の pointer は borrowed data です。
- borrowed pointer は、対応する snapshot / engine を解放するまでだけ有効です。
- Swift 側が長期保持する必要がある値だけ、Swift の `String` や `Array` へ copy します。
- Rust 側で確保した memory を Swift 側の `free()` で解放しません。

panic と error:

- FFI 境界を Rust panic が越えると未定義動作になるため、公開 FFI 関数は panic を外へ出しません。
- `scriptmetakit_ffi` の release profile では `panic = "abort"` を指定します。
- 可能な範囲で FFI entry point 内は `catch_unwind` し、panic は error code に変換します。
- Swift 側には戻り値の status code と `smk_last_error()` で失敗理由を返します。

Swift binding:

- 本番 package では header / modulemap / `binaryTarget` を基本にします。
- `@_silgen_name` は非公式属性のため、試作や最小検証に限定します。
- Swift public API は `ScriptMetaKitController` が受け持ち、FFI 関数名や pointer はアプリ本体へ露出させません。

Windows binding:

- Windows でも Rust core と FFI の境界方針は同じです。
- `scriptmetakit_ffi` は `cdylib` として `scriptmetakit_ffi.dll` を作り、必要に応じて import library と C header を同梱します。
- 呼び出し側が .NET / C# の場合は P/Invoke、C++ の場合は C header、Swift on Windows を使う場合は C module として binding します。
- ABI は `extern "C"` に統一し、Swift 専用の前提を置きません。
- `#[repr(C)]`、opaque handle、`ptr + len` slice、Rust 側 free 関数、panic を ABI 境界外へ出さない方針は macOS と同じです。
- path は OS 差分が大きいため、FFI 境界では UTF-8 byte string として受け渡し、Rust 側で `PathBuf` に変換します。Windows 固有の UTF-16 API は FFI 外へ漏らしません。
- watcher は Rust 側の platform 実装に閉じ込め、public API では `WatchPlan` と root event の形を維持します。

移行順:

1. Rust core の model と behavior を今の test app と CLI で固定します。
2. Swift adapter の public model を fixture JSON で検証します。
3. `scriptmetakit_ffi` を作り、engine / snapshot / error の opaque handle を公開します。
4. scan result の大きい配列を `#[repr(C)]` slice として返します。
5. watcher と update check の callback / async 境界を Swift adapter に閉じ込めます。
6. JSON 経路は debug / compatibility 用に残すか、不要になった時点で削除します。

## host app configuration

```swift
let configuration = ScriptMetaKitConfiguration(
    appID: hostAppID,
    cacheNamespace: "\(hostAppID).ScriptMetaKit",
    supportedExtensions: [
        "js", "jsx", "jsxbin", "jsxinc",
        "scpt", "applescript", "jxa",
        "idjs", "psjs"
    ],
    parser: .scriptaCompatible,
    scanner: .init(
        maxDepth: 24,
        maxNodesPerRoot: 20000,
        maxPrefixBytes: 128 * 1024,
        skipHidden: true,
        skipPackages: true,
        followSymlinks: true,
        resolveMacOSAlias: false,
        reuseUnchangedRecords: true,
        includeEmptyDirectories: false,
        scanTimeoutPerRoot: nil
    ),
    watcher: .init(
        enabled: true,
        watchPolicy: .visibleOnly,
        debounceDelay: .milliseconds(500),
        maxDeliveryDelay: .seconds(2),
        maxPendingPaths: 1024,
        monitorRootStrategy: .platformRecommended
    ),
    cache: .init(
        enabled: true,
        memoryCache: true,
        persistentCache: true,
        idleLifetime: .minutes(20),
        maxMemoryNodes: 60000,
        maxDeferredDirtyDirectories: 128,
        preserveUpdateResults: true
    ),
    updateCheck: .init(
        enabled: true,
        maxConcurrentMetaURLChecks: 6,
        requestTimeout: nil,
        resourceTimeout: nil,
        cacheNetworkResponses: false
    )
)
```

アプリ名、cache namespace、監視有無、cache 方針、root 登録は host app が決めます。kit は Scripta / ACEMenu / 将来の utility を特別扱いしません。

runtime hint は kit が返します。`.js` と `.applescript` は Scripta と同じく、先頭 bytes が `#!/usr/bin/osascript -l JavaScript` に一致する場合だけ JXA として扱い、それ以外の `.js` は Adobe JavaScript、`.applescript` と `.scpt` は AppleScript として扱います。

file list と metadata catalog の両方を使う root:

```swift
ScriptMetaKitRootRegistration(
    rootID: directory.id.uuidString,
    path: directory.url.path,
    displayName: directory.title,
    purpose: .fileListAndMetadata,
    watchPolicy: .visibleOnly,
    cachePolicy: .memoryAndPersistent,
    refreshPolicy: .onVisible,
    priority: .visibleWhenSelected
)
```

metadata catalog だけを使う root:

```swift
ScriptMetaKitRootRegistration(
    rootID: normalizedFolderPath,
    path: normalizedFolderPath,
    displayName: URL(fileURLWithPath: normalizedFolderPath).lastPathComponent,
    purpose: .metadataCatalog,
    watchPolicy: .disabled,
    cachePolicy: .persistentCatalogOnly,
    refreshPolicy: .manualOnly,
    priority: .background
)
```

folder 監視を有効にする場合は、app 側設定で次に変更します。

```swift
watchPolicy: .allRegistered
refreshPolicy: .onFileEventDeferred
```

## 呼び出し順

### Scripta

```text
App start
  -> create ScriptMetaKitController
  -> setRoots(all configured roots)
  -> loadCache(previous payload)
  -> startMonitoring()

Panel visible root changed
  -> set visible root in app state
  -> snapshot(rootID)
  -> if snapshot exists, show immediately
  -> scanRoots(file_list_and_metadata) when missing or dirty

File event
  -> controller receives event
  -> app receives fileListDidChange / catalogDidChange
  -> app reloads visible UI only when matching root changed

Settings update check
  -> scanRoots(metadata_only)
  -> checkUpdates(all_items)
  -> exportCache(catalog)
```

### ACEMenu

```text
App start
  -> create ScriptMetaKitController
  -> setRoots(registered folders)
  -> loadCache(previous payload)

Panel attach
  -> catalogSnapshot()
  -> show cached scripts and cached results

User clicks update check
  -> scanRoots(metadata_only)
  -> checkUpdates(all_items)
  -> exportCache(catalog and update results)
  -> update menu bar indicator

Automatic 24h check
  -> app schedule fires
  -> scanRoots(metadata_only)
  -> preserve old cache if timed out
  -> checkUpdates(all_items)
  -> update available count
```

## cache invalidation rules

Phase 1 で実装する invalidation:

- root removed
- root path changed
- supported extension changed
- parser schema changed
- scanner option changed
- cache schema changed
- overflow event
- dirty directory count exceeded
- file fingerprint changed
- item `script_id` changed
- item `version` changed
- item `meta_url` changed
- app requested manual invalidation

Phase 1 では、permission / security scoped bookmark invalidation は app 側 event として渡します。Rust core は `RootUnreadable` として扱います。

## watcher plan rules

`WatchPlan` は logical root と physical watcher を分けます。

```rust
pub struct WatchPlan {
    pub physical_roots: Vec<PhysicalWatchRoot>,
    pub logical_roots: Vec<LogicalWatchRoot>,
}
```

rules:

- 同じ normalized path は 1 件にまとめます。
- child root が parent root の内側にある場合、recursive watcher で covered として扱えます。
- 登録されていない親 directory へ勝手に広げません。
- macOS では複数 physical root を 1 FSEvents stream に渡せます。
- Windows では root ごとに watcher を持つ前提で計画します。
- event は必ず root_id ごとに返します。

## Phase 1 の実装順

1. `core` module を作成する。
2. metadata / distribution model を作成する。
3. parser skeleton と error model を作成する。
4. version compare を実装する。
5. `scanner` options と root model を作成する。
6. `catalog` config / root registry / snapshot model を作成する。
7. `watcher` watch plan model を作成する。
8. `storage` schema model を作成する。
9. `ScriptMetaKitEngine` の空実装を作成する。
10. host app に依存しない標準 configuration fixture を test に入れる。
11. parser fixture test を追加する。
12. root registration / watch plan test を追加する。

## Phase 1 の完了条件

- `cargo fmt --check` が通る
- `cargo check` が通る
- `cargo test` が通る
- Windows target の `cargo check --target x86_64-pc-windows-msvc` が、環境がある場合に通る
- 標準 configuration が Rust model に変換できる
- `WatchPlan` が duplicate root と nested root を期待通りに整理できる
- `ScriptMetaItem.name` が file name fallback に置き換わらない
- update check 対象は `version` と `meta_url` がある item のみになる

## Phase 2 へ渡すもの

Phase 1 完了後、Phase 2 では実際の parser / scanner を増やします。

Phase 2 の入口:

- Scripta の既存 SCRIPTMETA fixture を Rust parser test に移す
- ACEMenu の `DiscoveredScript` 相当を Rust model から作れるようにする
- `metadata_only` scan を実装する
- candidate cache の再利用を実装する
- update check の resolver を実装する

Phase 2 が終わるまで、Scripta / ACEMenu 本体の差し替えは行いません。まず package 単体で挙動を確認します。
