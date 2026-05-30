# SCRIPTMETAKit Rust 実装要件

この文書は、Scripta の SCRIPTMETA 実装を Rust ライブラリとして作り直すために必要な項目をまとめたものです。

最初の開発対象は macOS です。ただし、将来的に Windows でも同じライブラリを使う前提で、コア部分は OS 非依存の Rust として設計します。

Scripta と ACEMenu へ外部パッケージとして組み込むための入出力、監視、cache 方針は `PACKAGE_INTEGRATION_IO_JA.md` にまとめます。Phase 1 の public API と Rust module 構成は `PHASE1_PUBLIC_API_AND_MODULES_JA.md` にまとめます。

## 参照する実装

現在の挙動は次の Scripta ファイルを基準にします。

- `/Users/yamo/Desktop/GIT/Scripta/Scripta/ScriptMetaSupport.swift`
- `/Users/yamo/Desktop/GIT/Scripta/Scripta/ScriptMetaEditorSupport.swift`
- `/Users/yamo/Desktop/GIT/Scripta/Scripta/ScriptMetaSettingsSupport.swift`
- `/Users/yamo/Desktop/GIT/Scripta/Scripta/SettingsView.swift`
- `/Users/yamo/Desktop/GIT/Scripta/Scripta/FileListModels.swift`
- `/Users/yamo/Desktop/GIT/Scripta/Scripta/FileListSupport.swift`
- `/Users/yamo/Desktop/GIT/Scripta/Scripta/FileListCacheStore.swift`
- `/Users/yamo/Desktop/GIT/Scripta/SCRIPTMETA.md`
- `/Users/yamo/Desktop/GIT/SCRIPTMETA/GITHUB/SCRIPTMETA.md`
- `/Users/yamo/Desktop/GIT/SCRIPTMETA/GITHUB/SCRIPTMETA_AI_IMPLEMENTATION_GUIDE.md`

最も重要な参照元は `ScriptMetaSupport.swift` です。ここには、ローカルメタデータのモデル、scan、cache、script 側 parser、配布ページ parser、Meta-URL の読み込み、update check、progress event、retry、Latest-URL 解決が含まれています。

## Rust パッケージの目的

再利用できる Rust crate `scriptmetakit` を作ります。

crate が提供するもの:

- script 側 SCRIPTMETA parser
- 配布ページ側 SCRIPTMETA parser
- version の正規化と比較
- Meta-URL の正規化と validation
- ローカル script file scan
- 複数 root の scan result
- 差分再 scan 用 candidate cache
- update check と distribution resolution
- UI consumer 向け progress event
- AppKit に依存しない editor helper
- 必要に応じた cache serialization helper

crate が提供しないもの:

- AppKit / SwiftUI UI
- script の自動 download
- script の自動 install
- Sparkle による app update
- Scripta 固有の UserDefaults
- Scripta 固有の settings screen layout
- core module 内の macOS 専用処理

## Rust module 分割案

実装中にもっと簡単な分け方が明確にならない限り、次の分割を基本にします。

### `core`

- 共通 model
- 既知 key と block marker
- text decoding
- script 側 parser
- distribution 側 parser
- version 正規化と比較
- URL 正規化
- validation helper

### `scanner`

- 対応拡張子の判定
- local file read limit
- recursive root scan
- hidden / package skip policy
- symlink-aware identity path
- candidate record
- deduplication
- incremental root refresh

### `resolver`

- Meta-URL source classification
- gist raw `SCRIPTMETA.txt` loading
- GitHub repository raw `SCRIPTMETA.txt` loading
- GitHub directory raw `SCRIPTMETA.txt` loading
- generic text loading
- streamed read limit
- retry policy
- Latest-URL chain resolution
- 1回の check 内で使う source / parsed-record cache

### `catalog`

- multi-root state
- root-aware scan result
- persisted cache schema
- update result preservation
- app-controlled storage hook

### `editor`

- editable document parsing
- field validation
- block rendering
- block insertion / replacement
- password hash generation / verification
- backup metadata helper

`editor` module は parser、scanner、resolver の後で実装して構いません。UI なしでライブラリを使う consumer のために、別 module に分けます。

## platform 方針

共有 API と data model は macOS と Windows の両方で compile できる必要があります。一方で、実際の script metadata 抽出には OS 依存処理が必要になります。OS 依存を避けるのではなく、platform adapter として境界を分けます。

- file path は `Path` / `PathBuf` を使います。
- URL は `url::Url` を使います。
- shared module で `std::os::unix` や `std::os::windows` に直接依存しません。
- macOS alias file resolution、compiled AppleScript decompile、FSEvents は macOS adapter に分離します。
- Windows shortcut resolution、VBA / Office macro project 読み取り、ReadDirectoryChangesW は Windows adapter に分離します。
- symlink resolution は可能な範囲で `std::fs::canonicalize` を使います。
- Windows path comparison では `/` 区切りを前提にしません。
- cache schema は platform-specific type を含めずに serialize できる形にします。
- 日付は `time` / `chrono` などの portable type、または境界で ISO-8601 string として扱います。

現行 Scripta は macOS の alias file と symlink を解決しています。Rust 版ではまず cross-platform な symlink 対応を優先し、macOS alias support は target-specific adapter として扱います。

platform adapter が担当するもの:

- macOS: `.scpt` を `osadecompile` または OSA API で text 化して SCRIPTMETA を読む
- macOS: alias file 解決
- macOS: FSEvents 監視
- Windows: `.lnk` などの shortcut 解決
- Windows: `.bas`, `.vbs`, `.vba`, Office document / add-in 内の VBA module から SCRIPTMETA を読む
- Windows: ReadDirectoryChangesW 監視

shared core が担当するもの:

- root / file / metadata / cache / update check の共通 model
- text 化された SCRIPTMETA の parse
- extension policy、runtime hint、deduplicate、cache schema
- platform adapter から返された text と diagnostics の取り込み

## public data model

consumer が直接使う最小 model は次の通りです。

### Script Metadata Item

Scripta の `ScriptMetaListItem` に相当します。

fields:

- `root_id`
  - app が登録した root を識別する値
  - Illustrator / Photoshop / InDesign などの分類は Swift 側で管理する
- `file_path`
- `script_id`
- `version`
- `description`
- `target_app`
- `meta_url`
- `name`
- `author`
- `release_date`
- `edit_password_sha256`

derived fields:

- `id`
  - 通常は normalized file path
- `file_name`
- `folder_path`
- `is_update_checkable`
  - `version` と `meta_url` がある場合 true
- `has_edit_password`
  - `edit_password_sha256` が空でない場合 true

重要な挙動:

- `name` は `Name` tag の値として保持します。
- `name` を file name で置き換えません。
- display fallback は app consumer 側で決めます。

### Distribution Resolution

Scripta の `ScriptMetaDistributionResolution` に相当します。

fields:

- `latest_version`
- `latest_page_url`
- `final_page_url`
- `latest_url_history`
- `checked_at`
- `is_unresolved`
- `note`

補足:

- `latest_page_url` は最後に追跡した `Latest-URL` です。
- `final_page_url` は最終 record を返した source URL です。
- `latest_url_history` は検出した `Latest-URL` を順番に保持します。循環や same-page で追跡しなかった最後の URL も診断用に含めます。
- 最新版を確定できなかった場合、`is_unresolved` は true です。

### Update Check Result

Scripta の `ScriptMetaUpdateCheckResult` に相当します。

fields:

- `resolutions_by_item_id`
- `failures_by_item_id`
- `errors_by_item_id`
- `checked_at`

`failures_by_item_id` は UI と host app が扱うための構造化された失敗情報です。
各 record は `code`, `message`, `item_id`, `file_path`, `script_id`,
`current_version`, `meta_url`, `source_url`, `checked_at` を含みます。
`errors_by_item_id` は互換用の文字列 map として残します。

### Candidate Cache

Scripta の `ScriptMetaCandidateCache` に相当します。

fields:

- `schema_version`
- `built_at`
- `registered_directories`
- `records`

各 candidate record は次を持ちます。

- `root_path`
- `file_path`
- `identity_path`
- `file_size`
- `content_modified_at`
- parsed `item` または `None`

目的:

- 変更されていない file の再 parse を避ける
- SCRIPTMETA がない file も record として保持する
- 単一 root の incremental refresh を可能にする

### Root-Aware Scan Result

Rust API では、Scripta の flat result よりも context を多く返す形にします。

推奨 fields:

- `roots`
  - requested root ごとに1件
  - root_id、path、status、item count、errors を含める
- `all_items`
  - app display と update check 用の deduplicated item list
- `file_items`
  - file-list metadata lookup 用の per-file item list
- `candidate_cache`

これにより、Scripta と同じ使いやすさを保ちながら、root ごとの error や cache freshness を失わずに扱えます。

## 既知 tag と key

SCRIPTMETA v1.4 の closed syntax を使います。

markers:

- `SCRIPTMETA-BEGIN`
- `SCRIPTMETA-END`
- `SCRIPTMETA-DIST-BEGIN`
- `SCRIPTMETA-DIST-END`
- `Description-BEGIN`
- `Description-END`

recognized keys:

- `Script-ID`
- `Version`
- `Meta-URL`
- `META-URL`
- `Latest-URL`
- `Latest-Version`
- `Latest-Page-URL`
- `Target-App`
- `Min-Target-Version`
- `Release-Date`
- `Description`
- `Name`
- `Author`
- `Edit-Password-SHA256`

localized key や追加 block type は作りません。

次のようなものは v1.4 key として解釈しません。

- `Description-ja-BEGIN`
- `Description-en-BEGIN`
- `Name-ja`
- `Author-en`
- `Target-App-ja`
- `Latest-URL-en`
- `URL`
- `Page-URL`
- `Self-URL`
- `Distribution-URL`
- `Changelog-BEGIN`
- `Changelog-END`

## script 側 parser 要件

input:

- script text 全体、または local file scan で読んだ先頭 chunk

block selection:

- 最初の `SCRIPTMETA-BEGIN` から `SCRIPTMETA-END` までを取得します。
- parse 前に AppleScript line comment を preprocess します。
- trim 後に `--` で始まる行は、先頭の `--` と周辺 whitespace を取り除きます。
- line-based parse の前に CRLF と CR を LF に正規化します。

description parsing:

- trim した行全体が `Description-BEGIN` の場合だけ multiline description を開始します。
- trim した行全体が `Description-END` の場合だけ description を終了します。
- inner line break は保持します。
- final description は外側の whitespace と改行を trim します。
- description body 内では key parse をしません。
- 文中の `Description-END` は closing marker ではありません。

key-value parsing:

- description block を取り除いてから key-value を parse します。
- `=` を含む行だけ parse します。
- key は最初の `=` より前を trim したものです。
- value は最初の `=` より後を trim したものです。
- 空行は無視します。
- unknown key は通常 parser output では無視します。
- duplicate key は Scripta の dictionary behavior に合わせ、後勝ちで構いません。

line repair:

- Scripta は既知 key の前に改行が欠けている場合を regex で補正しています。
- Rust 版も同じ挙動にして、例えば `Script-ID=a Version=1.0 Meta-URL=https://example.com` のような text でも既知 key の前で分割できるようにします。

required fields:

- Local Subset は `Script-ID` が必須です。
- Update Profile は仕様上 `Script-ID`、`Version`、`Meta-URL` が必須です。
- local scanning は Local Subset を受け入れるため、`Version` や `Meta-URL` がないだけでは除外しません。

validation:

- `Script-ID` がなければ error
- 存在する `Version` が invalid なら error
- 存在する `Meta-URL` が invalid なら error
- `META-URL` は `Meta-URL` の alias として受け入れます。
- `Edit-Password-SHA256` は malformed でも text として返します。
- malformed `Edit-Password-SHA256` だけを理由に scan item を除外しません。

## distribution parser 要件

input:

- distribution page または `SCRIPTMETA.txt` の text

block selection:

- 最初の `SCRIPTMETA-DIST-BEGIN` から `SCRIPTMETA-DIST-END` までを取得します。
- distribution block がない場合は parse error とします。
- local script metadata 用の `SCRIPTMETA-BEGIN` から `SCRIPTMETA-END` は、distribution parser では使用しません。

record parsing:

- description block を取り除いてから parse します。
- `Script-ID` で record を分けます。
- 各 `Script-ID` が新しい record の開始です。
- key-value は最初の `=` で分割します。
- `Script-ID` のない record は無視します。
- `Script-ID` を key にして record map を返します。

version fields:

- `Version` を優先します。
- `Latest-Version` は fallback です。
- 返す前に version を正規化し、validate します。

next-page URL fields:

- `Latest-URL` を優先します。
- `Latest-Page-URL` は fallback です。
- 返す前に URL を正規化し、validate します。

distribution 側では output に使わない field:

- `Description`
- `Name`
- `Author`
- `Meta-URL`
- `Edit-Password-SHA256`

## text decoding 要件

local scan decoder は少なくとも次を試します。

- UTF-8
- UTF-16
- UTF-16LE
- UTF-16BE
- Japanese EUC
- Shift JIS

Scripta の scan は `ScriptMetaTextDecoder` で UTF-8、Japanese EUC、Shift JIS、UTF-16 を試しています。editor loading は BOM 付き UTF-8、UTF-16LE、UTF-16BE も確認しています。Rust 版ではこれらを統合した decoder にします。

`encoding_rs` などを使います。

decoder behavior:

- editor support が write-back する場合は text と encoding を返します。
- scan だけなら text を返せば足ります。
- empty file は metadata item なしです。
- decode できない data は parse/read error とし、panic しません。

## version 要件

normalization:

- 周辺 whitespace を trim
- 内部 whitespace を削除
- `\d+(?:\.\d+)*` に一致する最初の numeric dotted sequence を使う
- その matched sequence を返す
- empty input は reject
- numeric dotted sequence がなければ reject
- component が unsigned integer として parse できなければ reject

examples:

- `1.2.3` は `1.2.3`
- `v1.2.3` は `1.2.3`
- ` 1 . 2 ` は `1.2`

comparison:

- `.` で分割
- integer component として比較
- 足りない component は 0 で埋めます。
- `1.2` と `1.2.0` は同じです。
- invalid version は defensive fallback として同じ扱いで構いません。

## URL 要件

script-side parser normalization:

- whitespace と改行を trim
- surrounding single quote / double quote を取り除く
- scheme と host がある URL を受け入れる
- `http:/...` を `http://...` に修正
- `https:/...` を `https://...` に修正
- scheme がない場合は `https://` を付ける

editor validation はより厳密にします。

- usable scheme は `http` と `https` のみ
- host 必須
- `Meta-URL` に direct file link は不可

direct Meta-URL として禁止する suffix:

- `.zip`
- `.js`
- `.jsx`
- `.jsxinc`
- `.applescript`
- `.idjs`
- `.jxa`
- `.psjs`

## scanner 要件

対応拡張子:

- `js`
- `jsx`
- `jsxbin`
- `jsxinc`
- `scpt`
- `applescript`
- `jxa`
- `idjs`
- `psjs`

runtime hint:

- `.js` と `.applescript` は Scripta と同じく、先頭 bytes が `#!/usr/bin/osascript -l JavaScript` に一致する場合だけ JXA として扱います。
- shebang がない `.js` は Adobe JavaScript として扱います。
- shebang がない `.applescript` と `.scpt` は AppleScript として扱います。
- `.scpt` は compiled AppleScript なので、通常の text prefix scan だけでは SCRIPTMETA comment を取得できません。macOS adapter で `osadecompile` または OSA API を使って text 化します。
- Windows では VBA / Office macro project も script source の一種として扱います。binary container から module text を取り出す処理は Windows adapter に置き、core parser には text 化済み SCRIPTMETA を渡します。

local file read limit:

- scan では candidate file ごとに最大 128 KiB だけ読みます。

directory traversal:

- registered directory を recursive scan します。
- hidden file と hidden directory を skip します。
- package directory を skip します。
- non-regular file を skip します。
- 同じ resolved directory を重複 scan しません。
- scan 中に cancellation check を受け付けます。

path identity:

- resolved identity path で file を deduplicate します。
- 可能な範囲で symlink を解決します。
- macOS alias-file resolution は parity item ですが、target-specific で構いません。

candidate cache reuse:

- identity path、file size、content modification date が一致する場合、parsed item を再利用します。
- file size と modification date の両方が取得できる場合だけ再利用します。
- parsed item がない file も cache record として保持します。
- schema version を持たせます。

changed path filter:

- hidden intermediate component を含む path は無視します。
- last path component が `.` で始まる file は無視します。
- changed path に extension がない場合、metadata に影響する可能性ありとして扱います。
- extension がある場合、対応拡張子だけ relevant とします。

deduplication:

- まず identity path で file を deduplicate します。
- その後 `Script-ID` ごとに最良 item を1つ選びます。
- version がある item を version なし item より優先します。
- 両方 version がある場合は高い version を優先します。
- version が同じ、または両方ない場合は path が早い item を優先します。

sort order:

- display items は file name の自然順で sort します。
- file-items は full file path で sort します。

## update checker 要件

eligibility:

- `version` と `meta_url` の両方がある item だけ update-checkable です。
- Local Subset は scan / display しますが、update check からは除外します。

grouping:

- update-checkable item を `Meta-URL` ごとに group 化します。
- group 内で1つの network source context を共有します。
- Scripta は最大 6 Meta-URL group を並列 check します。

result:

- success 時は item id ごとに resolution を記録します。
- failure 時は item id ごとに構造化された失敗情報と互換用 error string を記録します。
- 1 item の失敗で全体を失敗させません。
- cancellation は task 全体を止めます。

progress events:

- network group の event id は Meta-URL string です。
- state values:
  - `active`
  - `active_error`
  - `persistent_error`
  - `finished`
- scan start / finish は app または catalog layer から emit します。
- 各 script id の resolution 前に item checking text を emit します。
- gist / GitHub candidate URL の loading text を emit します。
- retryable network failure では retry text を emit します。
- group が failure を含んで終わった場合は persistent error を emit します。
- group が failure なしで終わった場合は finished を emit します。

## distribution source loading 要件

`Meta-URL` は次の順で分類します。

1. gist URL
2. GitHub repository URL
3. GitHub directory URL
4. generic plain text URL

### gist URL

- host が `gist.github.com` を含む
- path に少なくとも `{user}/{gistID}` がある
- raw candidate を次の順に試します。
  - `https://gist.githubusercontent.com/{user}/{gistID}/raw/SCRIPTMETA.txt`
  - `https://gist.githubusercontent.com/{user}/{gistID}/raw/scriptmeta.txt`
  - `https://gist.github.com/{user}/{gistID}/raw/SCRIPTMETA.txt`
  - `https://gist.github.com/{user}/{gistID}/raw/scriptmeta.txt`
- SCRIPTMETA block を含む最初の candidate を採用します。
- 見つからなければ `gist_meta_file_not_found` を返します。

### GitHub repository URL

- host は `github.com`
- path は `{owner}/{repo}` の2要素
- raw candidate を次の順に試します。
  - `https://raw.githubusercontent.com/{owner}/{repo}/HEAD/SCRIPTMETA.txt`
  - `https://raw.githubusercontent.com/{owner}/{repo}/main/SCRIPTMETA.txt`
  - `https://raw.githubusercontent.com/{owner}/{repo}/master/SCRIPTMETA.txt`
- SCRIPTMETA block を含む最初の candidate を採用します。
- 見つからなければ `github_meta_file_not_found` を返します。

### GitHub directory URL

- host は `github.com`
- path は `{owner}/{repo}/tree/{branch}/{directory...}`
- 次を load します。
  - `https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{directory...}/SCRIPTMETA.txt`
- child directory は crawl しません。
- GitHub HTML は canonical update metadata として parse しません。

### Generic URL

- URL を text として fetch します。
- fetched text から SCRIPTMETA を parse します。

HTTP behavior:

- request timeout は 15 秒
- local cache を無視します。
- `Cache-Control: no-cache` を送ります。
- `Pragma: no-cache` を送ります。
- HTTP 2xx response のみ accepted
- non-HTTP response は invalid

streaming behavior:

- distribution metadata block が見つかるか EOF に到達するまで source stream を読みます。
- source 全体を固定長で打ち切りません。
- stream 中に begin / end marker を探します。
- marker が read chunk の境界で分割されても検出します。
- `SCRIPTMETA-DIST-BEGIN` / `SCRIPTMETA-DIST-END` のみを distribution metadata として扱います。
- `SCRIPTMETA-DIST` が EOF まで見つからない場合は parse error とします。
- begin marker 発見前の bytes は保持しません。
- end marker 発見後、その後の bytes は捨てます。
- 集めた metadata block bytes を text decoder で decode します。
- default の `max_metadata_block_bytes` 256 KiB は source 全体ではなく、保持する metadata block の最大サイズです。

retry behavior:

- retry 対象:
  - TLS / secure connection failure
  - network connection lost
  - timeout
- retry delay:
  - 1 秒
  - 3 秒
- retry を使い切ったら元の network error を返します。

## Latest-URL resolution 要件

input:

- start URL
- target script id

process:

- current URL から source を load
- distribution records を parse
- target `Script-ID` を探す
- なければ `record_not_found`
- record に `Latest-URL` があり、未訪問の新しい URL なら追跡
- follow 可能な `Latest-URL` がなくなるまで続ける

limits:

- maximum redirect count は 8
- same-page URL を検出
- circular reference を検出
- visited URL strings を保持

final resolution:

- final record に `latest_version` があれば返します。
- final record に version がなければ version-missing note 付き unresolved を返します。
- `Latest-URL` が同一 page を指す場合、Scripta の current resolution shape に合わせ、same-page ignored note を付けます。
- `Latest-URL` が循環する場合、circular-reference note を付けます。
- redirect limit を超えた場合、too-many-redirects note 付き unresolved を返します。

1回の check 内の source cache:

- loaded source は original URL string で cache
- maximum source cache count: 4
- parsed distribution records は resolved source URL string で cache
- maximum parsed cache count: 8
- 古い entry から evict

## cache 要件

Scripta には2つの cache file があります。

- `ScriptMetaUpdateCache.json`
- `ScriptMetaDisplayCache.json`

Rust 版では cache storage は app-controlled にします。library は serializable structure と helper function を提供できますが、保存 directory は app が決めます。

snapshot cache:

- source revision id
- deduplicated items
- update check result
- candidate cache

display cache:

- schema version
- source revision
- source cache fingerprint
- registered directories
- deduplicated items
- file items
- update check result

Scripta の cache size limit:

- update cache maximum: 50 MiB
- display cache maximum: 8 MiB

update-result preservation:

- rescanned item id がまだ存在する場合だけ cached result を preserve
- script id が一致する必要あり
- version が一致する必要あり
- meta URL が一致する必要あり
- stale resolutions / errors は捨てる

save behavior:

- 可能な範囲で atomic write
- schema versioning
- corrupt cache で panic しない
- corrupt cache や schema mismatch cache は無視する

## editor helper 要件

この section は scanner と resolver の後で実装して構いません。ただし Scripta が現在持っている behavior として要件に残します。

editable extensions:

- `js`
- `jsx`
- `jsxinc`
- `applescript`
- `idjs`
- `jxa`
- `psjs`

obfuscated JSX detection:

- `js`、`jsx`、`jsxbin` を probe
- 先頭 8 bytes を読む
- prefix が `@JSXBIN` なら obfuscated として editable にしない
- `jsxbin` は editable ではない

editable file limit:

- 最大 10 MiB

editor modes:

- `local_subset`
- `update_profile`

visible fields:

- Local Subset:
  - `Script-ID`
  - `Name`
  - `Author`
  - `Version`
  - `Release-Date`
  - `Target-App`
  - `Description`
- Update Profile:
  - `Script-ID`
  - `Name`
  - `Author`
  - `Version`
  - `Release-Date`
  - `Target-App`
  - `Min-Target-Version`
  - `Meta-URL`
  - `Description`

rendering 時の required fields:

- Local Subset:
  - `Script-ID`
- Update Profile:
  - `Script-ID`
  - `Version`
  - `Meta-URL`

additional fields:

- Local Subset:
  - `Name`
  - `Author`
  - `Version`
  - `Release-Date`
  - `Target-App`
  - `Edit-Password-SHA256`
- Update Profile:
  - `Name`
  - `Author`
  - `Release-Date`
  - `Target-App`
  - `Min-Target-Version`
  - `Edit-Password-SHA256`

save validation:

- `Script-ID` は必須
- `Script-ID` は `^[A-Za-z0-9._-]+$` に一致する必要あり
- 新規または変更された `Script-ID` は他の registered script と conflict してはいけない
- Update Profile は valid `Version` 必須
- Update Profile は valid `Meta-URL` 必須
- Local Subset は `Version` を持てるが、存在するなら valid である必要あり
- Local Subset は `Meta-URL` と `Min-Target-Version` を削除
- `Edit-Password-SHA256` は `Author` を必須にする
- empty password update は invalid
- malformed stored password hash は editor password check では invalid

password hash:

- stored format は `<salt>:<sha256>`
- salt は 16 文字の alphanumeric
- sha256 は lowercase hex 64 文字
- digest input は `{salt}:{password}`
- hash algorithm は SHA-256
- これは editor guard であり、author proof ではない

generated local Script-ID:

- format: `local.{user_component}.{file_component}_{suffix}`
- component は letters、digits、underscore、hyphen を許可
- unsupported character は `_`
- sanitization 中に連続 underscore をまとめる
- 先頭と末尾の `_` と `-` を trim
- suffix は 12 文字の lowercase alphanumeric
- repeated attempts が conflict した場合は UUID-derived suffix に fallback

release date:

- write format は `yyyy-MM-dd`
- read で受け入れる format:
  - `yyyy-MM-dd`
  - `yyyy/MM/dd`
  - `yyyy.MM.dd`

block comments:

- JavaScript 系 script は `/* ... */`
- AppleScript は `(* ... *)`
- metadata text 内の closing delimiter は escape:
  - `*/` は `* /`
  - `*)` は `* )`

editing 用 block detection:

- 最初の `SCRIPTMETA-BEGIN` から `SCRIPTMETA-END` range を探す
- expected block comment syntax の中にあれば受け入れる
- standalone line 上にあれば受け入れる
- それ以外なら次を探す

insertion:

- existing metadata block が leading header 内にあれば、その場で置き換える
- existing block が leading header 外なら、削除して上部へ挿入する
- leading block comment があれば closing delimiter の前に挿入
- なければ `#` で始まる leading target directive の後に挿入
- write 時は元の line ending を preserve

backups:

- save 前に backup
- restore 前に backup
- backup index は JSON
- file size と SHA-256 digest を含める
- current file を initial generation として保持したまま backup reset できる

backup storage は application behavior です。Rust library は helper type を出せますが、実際の directory は app 側で決めます。

## settings と file list integration 要件

ここは app-facing behavior ですが、Rust の result shape で実装しやすくします。

settings update check:

- 先に registered folders を scan
- scan progress を表示
- scan 後に current item list を更新
- rescanned items を checking として mark
- checkable items だけ update check
- stale item id を filter して result を適用
- refreshed snapshot を cache

card status:

- checking 中の item id は loading
- item id に error があれば failed
- resolution がなければ idle
- resolution はあるが `latest_version` がなければ failed
- item に current version がなければ idle
- current version が latest version より低ければ update available
- それ以外は up to date

update link:

- `latest_page_url` を優先
- なければ `final_page_url`

section grouping:

- regular item は trim 済み `target_app` で group 化
- target app がない場合は app-provided "other" title
- update-available items は priority update section に入れる
- failed items は priority error section に入れる
- normal target section は title で sort し、"other" は最後
- item は status priority と file name で sort

status priority:

1. update available
2. failed
3. loading
4. up to date
5. idle

file-list metadata:

- file list は hover 時に script file を parse しない
- display cache / catalog output から metadata を load
- normalized script path で metadata を key 化
- resolved URL を先に match し、display URL fallback を使う
- file name、script id、name、author、description、version、target app、release date、edit-password presence を含める
- search には script id、name、author、description、version、target app、release date を含める
- Local Subset でも update-checkable でなくても description と `Name` を表示できる

## AppleScript automation metadata

Scripta は metadata を AppleScript にも公開しています。Rust が AppleScript support を直接持つ必要はありませんが、host app が使えるよう public model に必要 field を残します。

- file path
- file name
- script id
- name
- author
- description
- version
- target app
- release date
- is update checkable
- has edit password

## error category

typed error を公開し、message の localization は app 側に任せます。

script metadata errors:

- `block_not_found`
- `script_id_missing`
- `version_missing`
- `version_invalid`
- `meta_url_missing`
- `meta_url_invalid`

local scan behavior:

- `block_not_found` は file を無視
- missing `Script-ID` は API により無視または report
- missing `Version` は Local Subset を reject しない
- missing `Meta-URL` は Local Subset を reject しない
- invalid present `Version` は parsed item を reject
- invalid present `Meta-URL` は parsed item を reject

distribution errors:

- `invalid_response`
- `text_decoding_failed`
- `block_not_found`
- `record_not_found`
- `version_invalid`
- `latest_page_url_invalid`
- `gist_meta_file_not_found`
- `github_repository_invalid`
- `github_meta_file_not_found`
- network error
- cancelled

resolution notes:

- same-page latest URL ignored
- circular latest URL
- version missing
- too many redirects

editor errors:

- unsupported file
- unreadable file
- file too large
- encoding failed
- backup failed
- write failed
- script id missing
- script id invalid
- script id conflict
- version missing
- version invalid
- meta URL missing
- meta URL invalid
- meta URL direct file
- author missing
- edit password missing
- edit password invalid
- edit password mismatch
- external change detected

## test 要件

### parser tests

- `Script-ID` だけの Local Subset
- required fields が揃った Update Profile
- block missing
- `Script-ID` missing
- Local Subset では `Version` missing を許容
- Local Subset では `Meta-URL` missing を許容
- invalid present version は reject
- invalid present Meta-URL は reject
- `META-URL` alias
- key に見える text を含む `Description`
- 文中の `Description-END` は block を閉じない
- CRLF / CR line ending
- AppleScript `--` comment preprocessing
- known key 前の missing-newline repair
- unknown key ignored
- duplicate known key は後勝ち

### distribution parser tests

- preferred `SCRIPTMETA-DIST` block
- missing `SCRIPTMETA-DIST` block
- multiple records split by `Script-ID`
- target record missing
- `Version`
- `Latest-Version`
- `Latest-URL`
- `Latest-Page-URL`
- invalid latest URL
- invalid version
- distribution description ignored

### version tests

- `1`
- `1.2`
- `1.2.0`
- `v1.2.3`
- version 前後と内部の whitespace
- invalid empty version
- invalid non-numeric version
- comparison の component padding

### URL tests

- regular https URL
- quoted URL
- missing scheme
- `http:/` repair
- `https:/` repair
- no host rejected
- direct file suffix は editor validation で reject

### scanner tests

- supported extensions
- unsupported extension ignored
- hidden file ignored
- hidden directory ignored
- package directory ignored where detectable
- read limit
- symlink identity deduplication
- duplicate `Script-ID` は higher version を選ぶ
- duplicate `Script-ID` tie は earlier path を選ぶ
- candidate cache reuse
- changed path relevance filter
- missing / unreadable root を含む root-aware result
- cancellation

### resolver tests

mocked HTTP で確認します。

- generic text URL
- gist candidate order
- gist not found
- GitHub repository HEAD / main / master order
- GitHub directory raw URL
- HTTP non-2xx
- SCRIPTMETA block のない response
- end marker 後の stream truncation
- retryable errors are retried
- non-retryable errors are not retried
- max byte limit
- cancellation

### Latest-URL tests

- direct latest version
- follow Latest-URL
- Latest-URL preferred over Version
- same-page URL note
- circular URL note
- too many redirects
- final version missing
- source cache reuse
- parsed records cache reuse

### cache tests

- snapshot serialize / deserialize
- schema mismatch ignored
- corrupt JSON ignored
- stale update result removed
- unchanged item preserves update result
- display cache has file items and deduplicated items

### editor tests

- editable extension detection
- JSXBIN signature detection
- mode ごとの field visibility
- Local Subset removes Meta-URL
- Update Profile requires Version and Meta-URL
- Script-ID regex
- edit password hash generation and verification
- malformed edit password rejected by editor
- description rendering
- unknown content preservation
- comment delimiter escaping
- insertion into leading block comment
- insertion after leading `#` directives
- replacement of existing top metadata block
- moving metadata block found later in file
- line ending preservation
- BOM and encoding detection

## 実装 phase

### Phase 1: Core Parser

- models
- known keys
- text normalization
- script-side parser
- distribution parser
- version normalization and comparison
- URL normalization
- unit tests

### Phase 2: Local Scanner

- supported extension filtering
- recursive scan
- per-file 128 KiB read limit
- candidate records
- deduplication
- root-aware result
- cache reuse
- temporary directory を使った tests

### Phase 3: Resolver

- HTTP abstraction
- source loader
- gist / GitHub URL handling
- streamed text loading
- retry behavior
- Latest-URL resolution
- progress events
- mocked HTTP tests

### Phase 4: Catalog and Cache

- serializable snapshot
- display cache shape
- update result preservation
- app-controlled storage helpers
- incremental root refresh

### Phase 5: Editor Helpers

- editable document loader
- field validation
- block rendering
- password helpers
- backup data helpers
- 必要なら write-back helpers

### Phase 6: App Integration

- `scriptmetakitApp` と接続
- scan result 表示
- update status 表示
- macOS UI flow の確認
- Rust core crate の外側に置く

### 後で再検討する項目

- script file の replace は現仕様では実装しません。必要性が出た場合だけ、仕様を見直してから検討します。

## dependency 候補

dependency は少なく、cross-platform なものを選びます。

- `serde`
- `serde_json`
- `thiserror`
- `url`
- `encoding_rs`
- `walkdir`
- `reqwest`
- `tokio`
- `sha2`
- `time` または `chrono`
- test 用 `tempfile`

networking や editor support を optional にする場合は feature flag を使います。

suggested features:

- `default = ["scanner", "resolver"]`
- `scanner`
- `resolver`
- `editor`
- `cache`

## 必ず守る挙動

- Local Subset scan inclusion に `Version` を要求しない。
- Local Subset scan inclusion に `Meta-URL` を要求しない。
- script を自動 download しない。
- script を自動 install しない。
- GitHub Releases を canonical update metadata として扱わない。
- repository / directory Meta-URL が given のとき、GitHub HTML を canonical update metadata として parse しない。
- localized keys や localized markers を作らない。
- `Edit-Password-SHA256` を author verification として扱わない。
- distribution page の description を app display 用に parse しない。
- loop / depth protection なしで `Latest-URL` を追跡しない。
- `Name` は metadata field として保持し、file name や display fallback と分ける。
