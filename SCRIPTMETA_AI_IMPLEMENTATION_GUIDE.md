# SCRIPTMETA AI Implementation Guide for Scripta

Use this document as the implementation instruction for a SCRIPTMETA-compatible
parser/resolver. Treat `SCRIPTMETA.md` as the human-facing specification and this
file as the precise behavior guide for AI-generated code.

This guide targets SCRIPTMETA v1.4.

Do not invent extra syntax. Do not add automatic download or installation behavior.

Primary implementation reference in this repository:

- `Scripta/ScriptMetaSupport.swift`

Related integration points:

- `Scripta/SettingsView.swift`
- `Scripta/FileListSupport.swift`
- `Scripta/FileListViews.swift`
- `Scripta/FileListModels.swift`

## 0. Required Result

Implement these capabilities:

1. Scan local script files for script-side SCRIPTMETA blocks.
2. Accept the Local Subset when only `Script-ID` is present.
3. Treat only items with both `Version` and `Meta-URL` as update-checkable.
4. Parse script descriptions safely and exactly.
5. Fetch and parse distribution metadata from `Meta-URL`.
6. Follow `Latest-URL` chains with loop and depth protection.
7. Prefer gist/GitHub `SCRIPTMETA.txt` raw text over HTML or page content.
8. Reproduce the settings screen update check, progress display, card display,
   status icons, and card buttons.

## 1. Vocabulary

### Script Metadata

Metadata embedded in a local script file.

Block markers:

```text
SCRIPTMETA-BEGIN
...
SCRIPTMETA-END
```

### Distribution Metadata

Metadata placed on a distribution page or in a `SCRIPTMETA.txt` file.

Preferred block markers:

```text
SCRIPTMETA-DIST-BEGIN
...
SCRIPTMETA-DIST-END
```

Legacy fallback markers:

```text
SCRIPTMETA-BEGIN
...
SCRIPTMETA-END
```

Description markers:

```text
Description-BEGIN
Description-END
```

Use marker and key spellings exactly as listed unless this guide explicitly says
otherwise. `META-URL` is accepted as an alias for `Meta-URL`. Network streaming
marker detection is ASCII case-insensitive for early truncation, but normal block
extraction and key parsing should use the canonical spellings above.

### Closed v1.4 Syntax

SCRIPTMETA v1.4 is a closed format. Do not invent new block markers, localized
markers, suffixed keys, aliases, or convenience URL fields.

Recognized block markers:

- `SCRIPTMETA-BEGIN`
- `SCRIPTMETA-END`
- `SCRIPTMETA-DIST-BEGIN`
- `SCRIPTMETA-DIST-END`
- `Description-BEGIN`
- `Description-END`

Recognized keys:

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

Do not create or interpret variants such as:

- `Description-en-BEGIN`
- `Description-ja-BEGIN`
- `Name-ja`
- `Author-en`
- `Target-App-ja`
- `Latest-URL-en`
- `Meta-URL-ja`
- `URL`
- `Page-URL`
- `Self-URL`
- `Distribution-URL`
- `Changelog-BEGIN`
- `Changelog-END`

Localization is not encoded in tag names. If a script author wants localized
text, it must be ordinary text inside a supported field, usually `Description`.
SCRIPTMETA v1.4 does not define a `Changelog` field or block.

### Update Profile

Script-side profile used for update checking.

Required by specification:

- `Script-ID`
- `Version`
- `Meta-URL`

Recommended:

- `Description`
- `Target-App`
- `Name`
- `Author`

Optional:

- `Edit-Password-SHA256`

Conditional requirement:

- If `Edit-Password-SHA256` is present, `Author` is required.

### Local Subset

Script-side profile used for local script list metadata and description popups.

Required:

- `Script-ID`

Recommended:

- `Description`
- `Version`
- `Target-App`
- `Name`

Optional:

- `Author`
- `Edit-Password-SHA256`

Conditional requirement:

- If `Edit-Password-SHA256` is present, `Author` is required.

Should not be written:

- `Meta-URL`

Implementation note:

- If a local file contains a valid `Meta-URL`, you may parse it.
- Do not update-check the item unless `Version` is also present.

### Distribution Page URL Rules

A distribution page does not need to describe its own URL. The source URL is
already known from the script-side `Meta-URL` or from a previous `Latest-URL`.

In a `SCRIPTMETA-DIST-BEGIN ... SCRIPTMETA-DIST-END` block:

- Write `Version` when this page contains the latest metadata.
- Write `Latest-URL` only when this page redirects update checking to another
  SCRIPTMETA information page.
- Do not write `Meta-URL` in a distribution record. `Meta-URL` is a script-side
  key.
- Do not write `URL`, `Page-URL`, `Self-URL`, `Distribution-URL`, or similar
  self-reference tags.
- Do not set `Latest-URL` to the current distribution page URL.
- Do not set `Latest-URL` to the same URL as the script-side `Meta-URL` being
  fetched.

If the latest metadata is already on the source page or its associated
`SCRIPTMETA.txt`, write `Version` and omit `Latest-URL`.

## 2. Local Scan Rules

Supported file extensions:

- `js`
- `jsx`
- `jsxbin`
- `jsxinc`
- `scpt`
- `applescript`
- `idjs`
- `psjs`

Read limit:

- Read only the first `128 * 1024` bytes from each local file.

Directory traversal:

- Skip hidden files.
- Skip hidden directories and their descendants.
- Skip package descendants.
- Skip non-regular files.
- Resolve symbolic links and alias files for duplicate detection.
- Avoid rescanning already visited directories.

For each candidate file:

1. Read up to 128 KB.
2. Decode text using the decoding order in section 3.
3. Extract the first script metadata block.
4. Require `Script-ID`.
5. Parse optional `Version`, `Description`, `Target-App`, `Meta-URL`, `Name`,
   `Author`, and `Edit-Password-SHA256`.
6. Store the item even if it is only a Local Subset item.

## 3. Text Decoding

Try encodings in this exact order:

1. UTF-8
2. Japanese EUC
3. Shift JIS
4. UTF-16 via Swift `.unicode`

The first successful decode wins.

If all decoding attempts fail, treat the file/source as unreadable.

## 4. Script Block Extraction

For local script metadata:

1. Preprocess text using section 7.
2. Find the first `SCRIPTMETA-BEGIN`.
3. Find the next `SCRIPTMETA-END` after it.
4. Use the text between those markers.
5. If either marker is missing, parsing fails with `blockNotFound`.

Use the first complete script metadata block only.

## 5. Script Metadata Parsing

Input:

- the script metadata block content from section 4.

Output fields:

- `scriptID: String`
- `version: String?`
- `descriptionText: String?`
- `targetApp: String?`
- `metaURL: URL?`
- `name: String?`
- `author: String?`
- `editPasswordSHA256: String?`

Algorithm:

1. Parse `Description` using section 6.
2. Remove the description block before key-value parsing.
3. Normalize the remaining block using section 8.
4. Parse `Key=Value` pairs using section 9.
5. Require `Script-ID`.
6. Parse `Version` only if present.
7. Parse `Meta-URL` or `META-URL` only if present.
8. Parse `Target-App` only if present.
9. Parse `Name` only if present.
10. Parse `Author` only if present.
11. Parse `Edit-Password-SHA256` only if present.

Validation:

- Missing `Script-ID` is an error.
- Missing `Version` is not an error for local scanning.
- Missing `Meta-URL` is not an error for local scanning.
- Invalid present `Version` is an error.
- Invalid present `Meta-URL` is an error.
- Invalid present `Edit-Password-SHA256` is not an error for local scanning.
  It is an editor validation error.
- If `Edit-Password-SHA256` is present and `Author` is missing, local scanning
  may still keep the item. Writers and editor save validation must reject this
  state.

Compatibility note:

- The code may still define `versionMissing` and `metaURLMissing` errors.
- Do not throw them during local script metadata parsing.

## 6. Description Parsing

Description markers:

```text
Description-BEGIN
Description-END
```

Use this exact behavior:

1. Preprocess the script block using section 7.
2. Iterate line by line.
3. For marker comparison, trim surrounding whitespace and newlines.
4. Before collection starts:
   - If trimmed line equals `Description-BEGIN`, start collecting.
5. While collecting:
   - If trimmed line equals `Description-END`, stop and return collected lines.
   - Otherwise append the original line to collected text.
6. Join collected lines with `\n`.
7. Trim surrounding whitespace and newlines from the final description.
8. If the final string is empty, return `nil`.
9. If no complete block exists, return `nil`.

Important behavior:

- While collecting, `Description-BEGIN` is plain description text.
- While collecting, only `Description-END` can close the block.
- `Description-END` closes the block only when the whole trimmed line is exactly `Description-END`.
- `■ Description-END` is not a closing marker.
- `Description-END です` is not a closing marker.
- `本文 Description-END` is not a closing marker.
- No escape syntax for literal `Description-END` is defined.

Examples:

These lines close the description block:

```text
Description-END
   Description-END
Description-END   
```

These lines do not close it:

```text
■ Description-END
Description-END です
本文 Description-END
```

Description removal before key-value parsing:

- Remove from `Description-BEGIN` through the next closing `Description-END`.
- Do not parse `Key=Value` lines inside that removed range.

## 7. Script Text Preprocessing

Normalize line endings:

- `\r\n` to `\n`
- `\r` to `\n`

Then process each line:

1. Trim whitespace for inspection only.
2. If the trimmed line starts with `--`, remove that leading `--`.
3. Trim whitespace again after removing `--`.
4. Append the stripped line.
5. Otherwise append the original raw line.

Purpose:

- This allows AppleScript comment lines like `-- Script-ID=...`.

Do not strip other comment syntaxes such as `//`, `/*`, `#`, or JSX comments.

## 8. Block Normalization

After preprocessing, normalize line endings again.

Then insert a newline before known keys when they appear inline as:

```text
 whitespace + Key=
```

Known keys:

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
- `Description-BEGIN`
- `Description-END`
- `Name`
- `Author`
- `Edit-Password-SHA256`

This allows partially minified blocks to parse.

Do not add unknown keys to this normalization list. In particular, do not add
localized or suffixed keys such as `Name-ja` or `Latest-URL-en`, and do not add
`Changelog` keys for v1.4.

## 9. Key-Value Parsing

For each normalized line:

1. Trim surrounding whitespace and newlines.
2. Ignore empty lines.
3. Ignore lines without `=`.
4. Split at the first `=` only.
5. Trim whitespace around the key.
6. Trim whitespace around the value.
7. Store the value.

If the same key appears more than once in the same parsed record, the later value
overwrites the earlier value.

### 9.1 Author and Edit Password Values

`Author`:

- is a single-line text value
- is recommended for Update Profile
- is optional for Local Subset
- is required when `Edit-Password-SHA256` is present
- must not be used as the script identifier

`Edit-Password-SHA256`:

- is optional for both Update Profile and Local Subset
- is used only by SCRIPTMETA editor UIs
- must not affect update-check eligibility
- must not be used as authorship proof

Format:

```text
Edit-Password-SHA256=<salt>:<sha256>
```

Validation for editor save:

1. Split at the first `:`.
2. Require non-empty `salt`.
3. Require `sha256` to be exactly 64 lowercase hex characters.
4. Require non-empty `Author`.

Password verification:

1. Read `salt` from the stored value.
2. Build the UTF-8 string `salt + ":" + inputPassword`.
3. Compute SHA-256.
4. Lowercase the hex digest.
5. Accept only when the digest equals the stored `sha256`.

Do not store the plain password in SCRIPTMETA.

## 10. Version Handling

Normalize a version string as follows:

1. Trim surrounding whitespace and newlines.
2. Remove all internal whitespace.
3. Find the first substring matching:

```text
\d+(?:\.\d+)*
```

4. Validate every dot-separated component as an integer.
5. Return that numeric dotted substring.

Examples:

- `1.2.3` -> `1.2.3`
- `v1.2.3` -> `1.2.3`
- `1. 2 .3` -> `1.2.3`
- `version one` -> invalid

Compare versions as follows:

1. Split both values by `.`.
2. Parse every component as `Int`.
3. Compare component by component.
4. Treat missing trailing components as `0`.

Examples:

- `1.2` equals `1.2.0`
- `1.10` is greater than `1.2`

If either version cannot be parsed during comparison, treat them as equal.

## 11. URL Handling

Normalize a URL string as follows:

1. Trim surrounding whitespace and newlines.
2. Trim surrounding single and double quotes.
3. If `URL(string:)` has both scheme and host, accept it.
4. Repair `http:/` to `http://`.
5. Repair `https:/` to `https://`.
6. If the value does not contain `://`, prepend `https://`.
7. Otherwise reject it.

## 12. Local Deduplication

Deduplicate by resolved file identity first:

- Resolve alias files when possible.
- Resolve symbolic links.
- Standardize file URLs.
- If the resolved file path was already seen, ignore the later file.

Then deduplicate by `Script-ID`:

1. If no current item exists, keep the candidate.
2. If current has no version and candidate has version, replace current.
3. If current has version and candidate has no version, keep current.
4. If both have versions:
   - keep the higher version
   - if versions compare equal, keep the lexicographically earlier file path
5. If neither has version:
   - keep the lexicographically earlier file path

Sort final local items by file name using localized standard comparison.

## 13. Update Check Eligibility

An item is update-checkable only when both are true:

- `version != nil`
- `metaURL != nil`

The following are valid Local Subset items but not update-checkable:

- `Script-ID` only
- `Script-ID` + `Description`
- `Script-ID` + `Version`
- `Script-ID` + `Meta-URL`
- `Script-ID` + `Name`
- `Script-ID` + `Description` + `Target-App`

Group update-checkable items by `metaURL.absoluteString`.

Concurrency:

- Check at most 6 distinct `Meta-URL` groups concurrently.
- Inside one group, resolve script IDs sequentially while sharing source and parse caches.

## 14. Update Task Lifecycle

Use this section to reproduce Scripta's update button behavior, task flow,
progress display, cancellation handling, and error state.

### 14.1 UI State

Maintain these state values:

- `scriptMetaItems: [ScriptMetaListItem]`
- `expandedScriptMetaGroups: Set<String>`
- `scriptMetaResolutionsByItemID: [String: ScriptMetaDistributionResolution]`
- `scriptMetaErrorsByItemID: [String: String]`
- `checkingScriptMetaItemIDs: Set<String>`
- `isCheckingScriptMeta: Bool`
- `scriptMetaProgressLines: [ScriptMetaProgressLine]`
- `scriptMetaUpdateTask: Task<Void, Never>?`
- `scriptMetaUpdateID: UUID?`
- `scriptMetaLastCheckedAtInterval: Double`

`ScriptMetaProgressLine` fields:

- `id: String`
- `text: String`
- `isError: Bool`
- `isPersistent: Bool`

### 14.2 Starting an Update Check

When the user presses the update check button:

1. Cancel any existing update task.
2. Create a new `UUID` as `updateID`.
3. Store it in `scriptMetaUpdateID`.
4. Start a new Swift `Task`.
5. In that task, call the update workflow with `updateID`.

Every async callback must verify that `scriptMetaUpdateID == updateID` before
modifying UI state. This prevents stale tasks from changing the current UI.

### 14.3 Main Update Workflow

The update workflow runs on the main actor for UI state changes.

Initial UI state:

1. If `updateID` is not current, return.
2. Set `isCheckingScriptMeta = true`.
3. Set `scriptMetaProgressLines` to one non-error non-persistent line:
   - `id = "scan"`
   - `text = scanning localized text`
4. Clear `scriptMetaErrorsByItemID`.
5. Clear `scriptMetaResolutionsByItemID`.
6. Build registered directory URLs by profile.

Scanning phase:

1. Run local scan in `Task.detached(priority: .utility)`.
2. Inside the detached task:
   - check cancellation
   - call local scanner
   - check cancellation again
   - deduplicate scanned items
3. On cancellation, finish as cancelled and return.
4. On scan failure, finish as cancelled and return.
5. If `updateID` is no longer current, return.
6. Set `scriptMetaItems = rescannedItems`.
7. Set `checkingScriptMetaItemIDs` to all rescanned item IDs.
8. Mark scan progress finished by applying:
   - `id = "scan"`
   - `text = ""`
   - `state = finished`

Update-check phase:

1. Count checkable items where `version != nil && metaURL != nil`.
2. If count is zero, add active progress:
   - `id = "summary"`
   - `text = no checkable items localized text`
   - `state = active`
3. Call the update checker with all rescanned items.
4. Pass a progress callback.
5. In the progress callback, dispatch back to the main actor.
6. In that main actor callback, ignore stale `updateID`, then apply progress.

If the update checker throws cancellation:

- finish as cancelled and return

If the update checker throws any other top-level error:

1. If stale, return.
2. Clear `checkingScriptMetaItemIDs`.
3. Set `isCheckingScriptMeta = false`.
4. Set `scriptMetaErrorsByItemID` for every rescanned item to the top-level error localized text.
5. Finish as not cancelled.
6. Do not save cache in this path.

Successful update-check phase:

1. If task is cancelled or stale, finish as cancelled and return.
2. Apply update state using section 24.
3. Save cache with:
   - `items = rescannedItems`
   - `updateCheckResult = result`
4. Finish as not cancelled.

### 14.4 Finishing and Cancelling

Cancel function:

1. Cancel `scriptMetaUpdateTask` if present.
2. Set `scriptMetaUpdateTask = nil`.
3. Set `scriptMetaUpdateID = nil`.

Finish function:

1. Return if `updateID` is stale.
2. Set `scriptMetaUpdateTask = nil`.
3. Set `scriptMetaUpdateID = nil`.
4. Reset update state.
5. If cancelled, remove all progress lines.
6. If not cancelled, keep persistent error progress lines but remove non-persistent lines.

Reset update state:

- Clear `checkingScriptMetaItemIDs`.
- Set `isCheckingScriptMeta = false`.
- If keeping persistent errors, remove progress lines where `isPersistent == false`.
- Otherwise clear all progress lines.

### 14.5 Progress Line Updates

Progress update states:

- `active`
- `activeError`
- `persistentError`
- `finished`

Apply progress updates as follows:

- `active`: upsert line with `isError = false`, `isPersistent = false`
- `activeError`: upsert line with `isError = true`, `isPersistent = false`
- `persistentError`: upsert line with `isError = true`, `isPersistent = true`
- `finished`: remove non-persistent line with the same ID

Upsert behavior:

- If a progress line with the same `id` exists, replace it.
- Otherwise append the new line.

In the header UI:

- Show progress lines when `isCheckingScriptMeta == true` or progress lines are not empty.
- Display progress text as caption.
- Use red text for error lines.
- Use secondary text color for non-error lines.
- Use one line with middle truncation.

### 14.6 Update Checker Worker Groups

This section describes network worker groups, not display sections.

The update checker receives all local items but filters to checkable items.

For each checkable item, build:

- `itemID = item.fileURL.path`
- `scriptID = item.scriptID`
- `metaURL = item.metaURL`

Group checkable items by:

- `metaURL.absoluteString`

Each group stores:

- `url: URL`
- `items: [(itemID: String, scriptID: String)]`

Run at most 6 `Meta-URL` groups concurrently using a throwing task group.

Inside each group task:

1. Set `progressID = group.url.absoluteString`.
2. Create one shared distribution resolution context for this group.
3. Iterate group items sequentially.
4. Before each item, send active progress:
   - `id = progressID`
   - `text = checking item localized text using item file name`
   - `state = active`
5. Resolve the item.
6. On success, append `(itemID, success(resolution))`.
7. On cancellation, rethrow cancellation.
8. On failure, append `(itemID, failure(error))` and remember final failure progress text.
9. Continue to the next item after a per-item failure.
10. After all items:
    - if any failure text was remembered, send `persistentError`
    - otherwise send `finished`
11. Return the group results.

Parent task behavior:

1. Start up to 6 group tasks.
2. As each group completes, merge returned results.
3. Success result goes to `resolutionsByItemID[itemID]`.
4. Failure result goes to `errorsByItemID[itemID] = error.localizedDescription`.
5. Start the next waiting group until all groups complete.

Return:

- `resolutionsByItemID`
- `errorsByItemID`
- `checkedAt = now`

### 14.7 Execution Context Summary

Use these execution contexts. Do not update SwiftUI state outside the main actor.

Main actor update task:

- owns `isCheckingScriptMeta`
- owns progress lines
- owns local item list
- owns card errors and resolutions
- starts the detached scanner
- starts the update checker
- applies cached or new update state
- saves the cache after a successful update check

Detached scanner task:

- runs with `.utility` priority
- performs file system enumeration
- reads local script files
- decodes and parses local metadata
- deduplicates parsed items
- checks cancellation before and after scanning
- returns parsed items to the main actor
- must not mutate SwiftUI state

Update checker task:

- receives all parsed items
- filters update-checkable items
- groups update-checkable items by `Meta-URL`
- runs up to 6 `Meta-URL` groups concurrently
- runs items inside one `Meta-URL` group sequentially
- creates one distribution resolution context per `Meta-URL` group
- uses that context to share loaded source text and parsed records
- returns per-item resolutions and per-item errors

Distribution resolution context:

- is an actor
- caches source text by URL string
- caches parsed distribution records by resolved source URL string
- loads network sources through URLSession
- must not mutate SwiftUI state

Progress callback:

- may be called from update checker worker tasks
- must dispatch to the main actor before touching UI state
- must ignore stale updates by checking `scriptMetaUpdateID == updateID`

Cache save:

- happens only after a successful update check result is applied
- writes the local items and update result
- posts the cache-changed notification

### 14.8 Progress and Error Text

Progress lines are header messages, not card messages.

Use these progress messages:

| Situation | English text | Japanese text |
| --- | --- | --- |
| scanning local scripts | `Scanning scripts...` | `スクリプトを読み込み中...` |
| no checkable items | `No scripts available for update check` | `更新確認できるスクリプトがありません` |
| checking one item | `Checking update info: {fileName}` | `更新情報を確認中: {fileName}` |
| checking candidate raw file | `Checking candidate: {urlText}` | `候補を確認中: {urlText}` |
| loading source | `Waiting for response: {urlText}` | `応答待ち: {urlText}` |
| retrying | `{reason}: retrying in {seconds} sec - {urlText}` | `{reason}: リトライ中（{seconds}秒） - {urlText}` |
| per-item failure | `{fileName}: failed - {errorText}` | `{fileName}: 失敗 - {errorText}` |
| unknown URL | `unknown URL` | `不明なURL` |

Retry reasons:

| URLError code | English text | Japanese text |
| --- | --- | --- |
| `secureConnectionFailed` | `TLS error` | `TLSエラー` |
| `timedOut` | `Timeout` | `タイムアウト` |
| `networkConnectionLost` | `Connection lost` | `通信切断` |

Retry timing:

- retry only `secureConnectionFailed`, `timedOut`, and `networkConnectionLost`
- retry delays are 1 second, then 3 seconds
- after retries are exhausted, throw the original error

Progress line state:

- retry messages use `activeError`
- per-item final failure messages use `persistentError`
- normal progress messages use `active`
- successful completion of a `Meta-URL` group uses `finished`

### 14.9 Error Mapping

Local script metadata parse errors:

| Error | English text | Japanese text |
| --- | --- | --- |
| `blockNotFound` | `No SCRIPTMETA block was found.` | `SCRIPTMETAブロックが見つかりません。` |
| `scriptIDMissing` | `Script-ID was not found.` | `Script-ID が見つかりません。` |
| `versionMissing` | `Version was not found.` | `Version が見つかりません。` |
| `versionInvalid` | `The Version format is invalid.` | `Version の形式が不正です。` |
| `metaURLMissing` | `Meta-URL was not found.` | `Meta-URL が見つかりません。` |
| `metaURLInvalid` | `The Meta-URL format is invalid.` | `Meta-URL の形式が不正です。` |

Current local scan behavior:

- missing `Version` is not thrown
- missing `Meta-URL` is not thrown
- unreadable files are ignored
- files with invalid present `Version` or invalid present `Meta-URL` are skipped
- local parse errors are not shown as cards, because skipped files are not in
  `scriptMetaItems`

Distribution metadata errors:

| Error | English text | Japanese text |
| --- | --- | --- |
| invalid response | `The distribution page response is invalid.` | `配布ページの応答が不正です。` |
| invalid response with status | `The distribution page response is invalid. ({statusCode})` | `配布ページの応答が不正です。({statusCode})` |
| text decoding failed | `The distribution page could not be read as text.` | `配布ページをテキストとして読めません。` |
| block not found | `No SCRIPTMETA block was found on the distribution page.` | `配布ページにSCRIPTMETAブロックが見つかりません。` |
| record not found | `The target Script-ID was not found on the distribution page.` | `配布ページに対象のScript-IDが見つかりません。` |
| version invalid | `The Version format on the distribution page is invalid.` | `配布ページの Version の形式が不正です。` |
| latest page URL invalid | `The Latest-Page-URL format is invalid.` | `Latest-Page-URL の形式が不正です。` |
| gist meta file not found | `SCRIPTMETA.txt was not found in the gist.` | `gist 内に SCRIPTMETA.txt が見つかりません。` |
| GitHub repository invalid | `The GitHub repository URL is invalid.` | `GitHub リポジトリ URL の形式が不正です。` |
| GitHub meta file not found | `SCRIPTMETA.txt was not found at the expected GitHub location.` | `GitHub の想定位置に SCRIPTMETA.txt が見つかりません。` |

Distribution notes:

| Note | English text | Japanese text |
| --- | --- | --- |
| same-page latest URL | `Ignored because Latest-URL points to the same page.` | `Latest-URL が同一ページを指しているため無視しました。` |
| circular latest URL | `Ignored because Latest-URL became a circular reference.` | `Latest-URL が循環参照になっているため打ち切りました。` |
| version missing | `The latest version could not be determined because Version was not found.` | `Version が見つからず、最新版を確定できません。` |
| too many redirects | `The number of Latest-URL redirects exceeded the limit.` | `Latest-URL の追跡回数が上限に達しました。` |

Card error display:

- per-item distribution errors are stored in `scriptMetaErrorsByItemID[item.id]`
- the card status becomes `failed`
- the card displays the error text in red caption text
- if a top-level update checker error occurs, every rescanned item receives the
  same localized error text

Non-error unresolved result display:

- when resolution has no `latestVersion`, card status becomes `failed`
- if the resolution has a note, show the note as secondary caption2 text
- this is used for missing final version, same-page `Latest-URL`, circular
  `Latest-URL`, or too many redirects

## 15. Distribution Source Loading

When loading a `Meta-URL`, choose one of these modes.

### 15.1 Gist Mode

Use gist mode when:

- `url.host` contains `gist.github.com`, case-insensitive.

Extract:

- first path component as user
- second path component as gist ID

Try candidate raw URLs in this exact order:

1. `https://gist.githubusercontent.com/{user}/{gistID}/raw/SCRIPTMETA.txt`
2. `https://gist.githubusercontent.com/{user}/{gistID}/raw/scriptmeta.txt`
3. `https://gist.github.com/{user}/{gistID}/raw/SCRIPTMETA.txt`
4. `https://gist.github.com/{user}/{gistID}/raw/scriptmeta.txt`

For each candidate:

1. Fetch as plain text.
2. Accept it only if it contains either:
   - `SCRIPTMETA-DIST-BEGIN` and `SCRIPTMETA-DIST-END`
   - or `SCRIPTMETA-BEGIN` and `SCRIPTMETA-END`
3. Use the first accepted candidate.

If none are accepted, throw `gistMetaFileNotFound`.

Return the original gist URL as the source `resolvedURL`, not the raw candidate URL.

### 15.2 GitHub Repository Mode

Use GitHub repository mode only when:

- host is exactly `github.com`, case-insensitive
- path has exactly two components: `{owner}/{repo}`

Try candidate raw URLs in this exact order:

1. `https://raw.githubusercontent.com/{owner}/{repo}/HEAD/SCRIPTMETA.txt`
2. `https://raw.githubusercontent.com/{owner}/{repo}/main/SCRIPTMETA.txt`
3. `https://raw.githubusercontent.com/{owner}/{repo}/master/SCRIPTMETA.txt`

For each candidate:

1. Fetch as plain text.
2. Accept it only if it contains either:
   - `SCRIPTMETA-DIST-BEGIN` and `SCRIPTMETA-DIST-END`
   - or `SCRIPTMETA-BEGIN` and `SCRIPTMETA-END`
3. Use the first accepted candidate.

If none are accepted, throw `githubMetaFileNotFound`.

Return the original GitHub repository URL as the source `resolvedURL`, not the raw candidate URL.

Do not use GitHub Releases, README HTML, or repository page HTML as the canonical update source.

### 15.3 GitHub Directory Mode

Use GitHub directory mode only when:

- host is exactly `github.com`, case-insensitive
- path has at least five components: `{owner}/{repo}/tree/{branch}/{directory...}`
- third path component is exactly `tree`
- `{branch}` is a single path component
- `{directory...}` is one or more path components

Extract:

- first path component as owner
- second path component as repository name
- fourth path component as branch name
- remaining path components as directory path

Build this candidate raw URL:

```text
https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{directory...}/SCRIPTMETA.txt
```

Fetch the candidate as plain text.

Accept it only if it contains either:

- `SCRIPTMETA-DIST-BEGIN` and `SCRIPTMETA-DIST-END`
- or `SCRIPTMETA-BEGIN` and `SCRIPTMETA-END`

If the candidate is not accepted, throw `githubMetaFileNotFound`.

Return the original GitHub directory URL as the source `resolvedURL`, not the raw candidate URL.

The original GitHub directory URL is the human-facing distribution page. Opening
it should show the script directory, allowing users to select, inspect, and
download the `.jsx` file manually. Do not parse the GitHub directory page HTML as
the canonical update source.

Example:

```text
Meta-URL=https://github.com/Yamonov/Iwashiya_Scripts/tree/main/Illustrator
```

Fetch:

```text
https://raw.githubusercontent.com/Yamonov/Iwashiya_Scripts/main/Illustrator/SCRIPTMETA.txt
```

Operational example:

`https://github.com/Yamonov/Iwashiya_Scripts/` is a repository-level entrance
for users. If scripts are grouped under app-specific directories such as
`Photoshop` and `Illustrator`, each script-side `Meta-URL` should point to its
own app directory:

```text
Meta-URL=https://github.com/Yamonov/Iwashiya_Scripts/tree/main/Photoshop
Meta-URL=https://github.com/Yamonov/Iwashiya_Scripts/tree/main/Illustrator
```

Fetch each directory's metadata independently:

```text
https://raw.githubusercontent.com/Yamonov/Iwashiya_Scripts/main/Photoshop/SCRIPTMETA.txt
https://raw.githubusercontent.com/Yamonov/Iwashiya_Scripts/main/Illustrator/SCRIPTMETA.txt
```

Do not treat `https://github.com/Yamonov/Iwashiya_Scripts/tree/main` as a
directory index to crawl for child `SCRIPTMETA.txt` files.

### 15.4 Plain URL Mode

Use plain URL mode for all other URLs.

Rules:

- Fetch the URL directly.
- Request timeout is 15 seconds.
- Response must be an HTTP response.
- HTTP status must be 200 through 299.
- Otherwise throw `invalidResponse`.

## 16. Streaming Network Fetch

Network fetch constants:

- maximum bytes: `256 * 1024`
- request timeout: 15 seconds
- marker scan interval: `16 * 1024`
- retry delays: 1 second, then 3 seconds

Retry only these `URLError.Code` values:

- `.secureConnectionFailed`
- `.networkConnectionLost`
- `.timedOut`

Begin markers:

- `SCRIPTMETA-DIST-BEGIN`
- `SCRIPTMETA-BEGIN`

End markers:

- `SCRIPTMETA-DIST-END`
- `SCRIPTMETA-END`

Marker matching:

- ASCII case-insensitive
- byte-by-byte

Streaming algorithm:

1. Append bytes until the stream ends or maximum bytes is reached.
2. Every 16 KB, if no begin marker has been seen:
   - search the buffer for any begin marker
   - if found, drop bytes before the begin marker
   - if not found, keep only enough tail bytes to survive split-marker boundaries
3. Once a begin marker has been seen:
   - search for any end marker
   - if found, truncate after that end marker, decode, and return immediately
4. After stream completion, run begin/end marker checks one final time.
5. If no complete block is found, decode and return the remaining buffer.

## 17. Distribution Block Selection

Given distribution source text:

1. Look for `SCRIPTMETA-DIST-BEGIN ... SCRIPTMETA-DIST-END`.
2. If any preferred distribution blocks exist, use the first one by source order.
3. Otherwise look for legacy `SCRIPTMETA-BEGIN ... SCRIPTMETA-END` blocks.
4. Exclude legacy blocks containing `Meta-URL` or `META-URL`.
5. Among remaining legacy blocks, choose the highest priority block.

Legacy priority, highest first:

1. contains `Latest-URL` or `Latest-Page-URL`
2. contains `Release-Date`
3. contains `Version` or `Latest-Version`
4. none of the above

Tie-break:

- If priorities are equal, the later source block wins.

If no distribution block is found, throw `blockNotFound`.

## 18. Distribution Record Parsing

Input:

- one selected distribution block

Algorithm:

1. Remove any description block.
2. Normalize the block.
3. Parse key-value lines.
4. Split records at each new `Script-ID`.

Record rules:

- A record without `Script-ID` is ignored.
- `latestVersion` comes from `Version`, else `Latest-Version`.
- If a version field exists, it must normalize successfully.
- `latestPageURL` comes from `Latest-URL`, else `Latest-Page-URL`.
- If a latest page URL exists, it must normalize successfully.
- `Meta-URL` inside a distribution record is ignored. It is a script-side key,
  not a distribution-side key.
- Unknown fields are ignored. Do not reinterpret unknown fields as canonical
  SCRIPTMETA fields.
- If duplicate `Script-ID` records exist, the later parsed record overwrites the earlier one.

## 19. Distribution Resolution

Resolve updates for one local script item as follows:

Initial state:

- `currentURL = item.metaURL`
- `visitedURLs = { item.metaURL.absoluteString }`
- `lastRedirectURL = nil`
- `finalSourceURL = item.metaURL`
- max redirect count: 8

Loop:

1. Load distribution source from `currentURL`.
2. Set `finalSourceURL` to the loaded source's resolved URL.
3. Parse distribution records.
4. Find the record for `item.scriptID`.
5. If absent, throw `recordNotFound`.
6. If `latestPageURL` exists, is not equal to `currentURL`, and has not been visited:
   - set `lastRedirectURL = latestPageURL`
   - add it to `visitedURLs`
   - set `currentURL = latestPageURL`
   - continue
7. Otherwise stop and return a resolution.

Returned resolution:

- `latestVersion`: final record version, or nil
- `latestPageURL`: last followed redirect URL, or nil
- `finalPageURL`: final fetched source URL
- `checkedAt`: current date/time
- `isUnresolved`: true when final latest version is nil
- `note`: optional note

Stop notes:

- same-page `Latest-URL`: same-page ignored
- circular `Latest-URL`: circular reference
- no final version: version missing
- redirect count exceeded: too many redirects

If the 8-step limit is reached:

- return unresolved result
- set `latestVersion = nil`
- set note to too many redirects

## 20. Cache Behavior

Local metadata cache stores:

- parsed local items
- latest update check result

Cache file:

- application support directory
- bundle identifier directory
- `ScriptMetaUpdateCache.json`

On cache save:

- create directory if needed
- write JSON atomically
- post `.scriptMetaCacheDidChange`

On cache remove:

- remove cache file if present
- post `.scriptMetaCacheDidChange`

Distribution resolution context caches within one check group:

- source cache by `url.absoluteString`
- parsed record cache by `source.resolvedURL.absoluteString`

## 21. File List Popup Integration

The file list must not parse script files on hover.

Use the existing script metadata cache:

1. Load `ScriptMetaCacheStore`.
2. Build metadata by normalized file path.
3. Use only items with non-empty `descriptionText` for popups.

Popup metadata contains:

- file name
- name if present
- description
- version if present
- target app if present
- profile kind:
  - `SCRIPTMETA Update Profile` when both `version` and `metaURL` are present
  - `SCRIPTMETA Local Subset` otherwise

Show description popup only when:

- the profile's meta button setting is enabled
- cached metadata exists for the file
- cached description is non-empty

Local Subset items may show popups even when they are not update-checkable.

In the popup metadata line:

- show `Version {version}` when version exists
- always show the profile kind
- show `Target {targetApp}` when target app exists

Popup title area:

- show the file name first
- if `Name` is present and non-empty, show the `Name` value below the file name

## 22. Settings Display Grouping

The settings screen displays parsed script metadata as sections.

Use this section to reproduce the current Scripta grouping behavior. This is
different from the network update-check grouping by `Meta-URL`.

### 22.1 Section Model

Each display section has:

- `title: String`
- `representativeTargetApp: String?`
- `items: [ScriptMetaListItem]`

### 22.2 Normal Grouping

Start with all parsed local items.

Group items by target app:

1. Trim whitespace and newlines from `item.targetApp`.
2. If the trimmed value is non-empty, use it as the group title.
3. If missing or empty, use `otherGroupTitle`.

For a normal target-app section:

- `title = normalized target app`
- `representativeTargetApp = normalized target app`

For the other section:

- `title = otherGroupTitle`
- `representativeTargetApp = nil`

### 22.3 Update Section

Build a separate update section from all items whose display status is
`updateAvailable`.

If there are no `updateAvailable` items:

- do not create an update section

If there are update items:

- create a section at the top of the list
- `title = updatesGroupTitle`
- `representativeTargetApp = nil`
- `items = all updateAvailable items`

Also remove updateAvailable items from their normal target-app sections.

This means the same item must not appear both in the update section and in its
target-app section.

### 22.4 Section Ordering

Order sections as follows:

1. Update section first, only when it exists.
2. Normal target-app sections sorted by localized standard comparison of `title`.
3. Other section last.

### 22.5 Item Ordering Inside Sections

Sort items inside each section by display status priority, then file name.

Status priority:

1. `updateAvailable`
2. `failed`
3. `loading`
4. `upToDate`
5. `idle`

If priority is equal:

- sort by `fileName` using localized standard comparison

Because updateAvailable items are removed from normal sections, normal sections
usually begin with failed/loading/up-to-date/idle items.

### 22.6 Display Status

Compute card status from current UI state:

1. If `checkingScriptMetaItemIDs` contains `item.id`, status is `loading`.
2. Else if `scriptMetaErrorsByItemID[item.id]` exists, status is `failed`.
3. Else if no resolution exists for `item.id`, status is `idle`.
4. Else if the resolution has no `latestVersion`, status is `failed`.
5. Else if `item.version` is nil, status is `idle`.
6. Else compare `item.version` with `latestVersion`.
   - if current version is lower, status is `updateAvailable`
   - otherwise status is `upToDate`

Local Subset items without update results normally remain `idle`.

### 22.7 Expansion Behavior

The update section is always expanded.

When applying cached or newly fetched update state:

1. Rebuild display sections.
2. Add every section containing an `updateAvailable` item to the expanded group set.

Because updateAvailable items are placed in the update section, this expands the
update section when updates exist.

Other sections use the user's stored expansion state.

### 22.8 Section Icons

Use these icons:

- update section: `arrow.up.circle.fill`
- known target app section: app icon resolved from bundle identifier
- unknown/other section: `questionmark.app.fill`

Target app bundle matching:

- target app containing `illustrator` -> `com.adobe.illustrator`
- target app containing `photoshop` -> `com.adobe.Photoshop`
- target app containing `indesign` -> `com.adobe.InDesign`
- otherwise unknown app icon

Cache app icons by section target/title to avoid repeated lookups.

## 23. Settings Card UI

Use this section to reproduce the visible card layout and button visibility.

### 23.1 Header UI

When no registered folders exist:

- show an empty state instead of the section list

When folders exist:

- show a fixed header above the scrollable section list

Header content:

- link labeled `SCRIPTMETA`
- registered folder count
- profile count line: `Update Profile: {updateProfileCount}    Local Profile: {localProfileCount}`
- last checked date, or "never checked" text
- progress lines when checking or when persistent progress errors exist
- update check button
- update count text when not checking and `lastCheckedAt != nil`

Profile counts:

- `updateProfileCount` is the number of local items with both non-empty `version` and non-nil `metaURL`
- `localProfileCount` is `scriptMetaItems.count - updateProfileCount`
- the profile count line uses the same font size as the registered folder count, no bold weight, and primary text color

Update check button:

- disabled when `folderCount == 0`
- disabled while `isChecking == true`
- shows a small progress view while `isChecking == true`

Update count text:

- hidden while checking
- hidden when there is no last checked date
- if update count is greater than zero, show update count text
- otherwise show no-updates text

### 23.2 Section UI

Each section is a disclosure group.

Section label:

- 20 x 20 icon
- bold section title
- item count caption

Section body:

- if no items, show a secondary caption "no items" text
- otherwise show cards in a lazy vertical stack

### 23.3 Card Layout

Each card:

- fixed height: 138
- rounded rectangle background, corner radius 8
- border color and width based on card status
- right command column width: 34
- vertical separator before command column

Left content:

1. Top row:
   - status icon
   - file name
   - script ID
   - metadata line with `Target: {targetApp}` when target app is present and `Name: {name}` when name is present
   - version view on the right
2. Bottom area:
   - description area
   - note line if present
   - error line if present

Description area:

- scrollable
- fixed height: 52
- if `descriptionText` is non-empty, show it as callout text
- otherwise render empty clear space

### 23.4 Status Icons

Status icon by card status:

- `idle`: `s.circle.fill`, no explicit color
- `loading`: small progress view, 18 x 18
- `upToDate`: `checkmark.circle.fill`, green
- `updateAvailable`: `arrow.up.circle.fill`, blue opacity 0.95
- `failed`: `xmark.circle.fill`, red

### 23.5 Card Colors

Background:

- `updateAvailable`: blue opacity 0.08
- all other statuses: control background color

Border:

- `updateAvailable`: blue opacity 0.9
- `failed`: red opacity 0.35
- all other statuses: secondary opacity 0.28

Border width:

- `updateAvailable`: 3
- all other statuses: 1.5

### 23.6 Version View

If status is `updateAvailable` and both current and latest versions exist:

- show `currentVersion -> latestVersion`
- use `arrowshape.forward.circle.fill` between the two versions
- use subheadline font
- make both version texts bold

If `updateURL(for:)` returns a URL:

- make the version view a button
- opening it should open the update URL
- use blue opacity 0.95
- use plain button style

If no update URL exists:

- show the same version comparison as non-button text

For every other status:

- show `item.version` if present
- otherwise show `-`
- use subheadline bold text
- fixed horizontal size

### 23.7 Right Column Buttons

The right column always has fixed width 34.

Buttons are vertically stacked and use plain button style.

Finder button:

- always visible
- icon: `document`
- frame: 18 x 18
- action: reveal `item.fileURL` in Finder
- help text: `item.folderPath`

Meta URL button:

- visible only when `metaURLForItem(item)` returns a URL
- icon: `house.fill`
- frame: 18 x 18
- action: open meta URL
- help text: meta URL absolute string

Update URL button:

- visible only when status is `updateAvailable` and `updateURLForItem(item)` returns a URL
- icon: `arrowshape.up.circle.fill`
- icon color: blue opacity 0.95
- frame: 18 x 18
- action: open update URL
- help text: update URL absolute string

`updateURL(for:)` returns:

1. `resolution.latestPageURL` if present
2. otherwise `resolution.finalPageURL` if present
3. otherwise nil

`metaURLForItem(item)` returns:

- `item.metaURL`

### 23.8 Note and Error Lines

Note line:

- visible only when `noteForItem(item)` returns text
- font: caption2
- color: secondary

Error line:

- visible only when `errorForItem(item)` returns text
- font: caption
- color: red

`errorForItem(item)` reads:

- `scriptMetaErrorsByItemID[item.id]`

`noteForItem(item)` reads:

- `scriptMetaResolutionsByItemID[item.id]?.note`

## 24. Applying Cached or New Update State

Use this section to reproduce how cached and newly fetched update results affect
card status, sections, last checked date, and expansion.

When there is no cached result:

1. Clear `errorsByItemID`.
2. Clear `resolutionsByItemID`.
3. Set `lastCheckedAtInterval = 0`.
4. Do not modify parsed local items.

When there is a result:

1. Build `validItemIDs` from current `items.map(\.id)`.
2. Keep only resolutions whose item ID is still valid.
3. Keep only errors whose item ID is still valid.
4. Set `lastCheckedAtInterval = result.checkedAt.timeIntervalSince1970`.
5. Rebuild display sections with current status provider.
6. Add section titles containing at least one `updateAvailable` item to `expandedGroups`.

When loading cache:

1. Do nothing if an update check is currently running.
2. If cache does not exist:
   - clear items
   - clear resolutions
   - clear errors
   - clear last checked date
   - clear expanded groups
3. If cache exists:
   - deduplicate cached items
   - apply cached update state as above

## 25. SCRIPTMETA Editor Edit Lock

This section applies only to SCRIPTMETA editor UI behavior. It does not affect
local scanning, update checking, distribution resolution, or update trust.

When opening a SCRIPTMETA editor for a script metadata block:

1. Parse the block.
2. If `Edit-Password-SHA256` is absent, open the editor normally.
3. If `Edit-Password-SHA256` is present and valid, show a password prompt before
   opening the editable window.
4. If the password matches, open the editor normally.
5. If the password does not match or the user cancels, do not open the editor.
6. If `Edit-Password-SHA256` is present but malformed, show an error and do not
   open the editor.

When saving from a SCRIPTMETA editor:

1. If password protection is disabled, do not write `Edit-Password-SHA256`.
2. If password protection is enabled, require non-empty `Author`.
3. Generate a non-empty salt.
4. Write `Edit-Password-SHA256` as `<salt>:<sha256>`.
5. The SHA-256 input is the UTF-8 string `salt + ":" + password`.
6. Never write the plain password.

This is not a security boundary. The source file is plain text, so a user can
remove or alter the tag in a text editor. Treat the tag as an editor-level guard
against accidental or unauthorized editing inside SCRIPTMETA-aware tools.

## 26. Must Not Do

Do not:

- require `Version` for local script scanning
- require `Meta-URL` for local script scanning
- treat Local Subset items as update-checkable
- reject a local scan item only because `Edit-Password-SHA256` is malformed
- store plain edit passwords
- use `Edit-Password-SHA256` as authorship proof
- use `Edit-Password-SHA256` for update-check eligibility
- parse `Description` from distribution pages for display
- treat `■ Description-END` as a closing marker
- add an escape syntax for `Description-END`
- create localized or suffixed markers such as `Description-en-BEGIN`
- create localized or suffixed keys such as `Name-ja` or `Latest-URL-en`
- silently reinterpret unknown keys as supported keys
- add `Changelog` support for v1.4
- write `Meta-URL` inside a distribution block
- write URL self-reference keys such as `URL`, `Page-URL`, `Self-URL`, or
  `Distribution-URL`
- set `Latest-URL` to the current distribution page
- set `Latest-URL` to the same URL as the script-side `Meta-URL` being fetched
- follow `Latest-URL` without a redirect limit
- ignore circular `Latest-URL` chains
- use GitHub Releases as the canonical update source
- parse GitHub repository or directory page HTML as the canonical update source
- download or install scripts automatically

## 27. Minimal Test Cases

A compatible implementation should pass these cases.

### Local Subset

Input:

```text
SCRIPTMETA-BEGIN
Script-ID=com.example.localTool
Description-BEGIN
Local description
Description-END
SCRIPTMETA-END
```

Expected:

- accepted as local item
- `scriptID = com.example.localTool`
- `version = nil`
- `metaURL = nil`
- not update-checkable

### Update Profile

Input:

```text
SCRIPTMETA-BEGIN
Script-ID=com.example.tool
Version=v1.2.3
Meta-URL=https://gist.github.com/example/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
SCRIPTMETA-END
```

Expected:

- accepted as local item
- `version = 1.2.3`
- `metaURL` parsed
- update-checkable

### Author and Edit Password

Input:

```text
SCRIPTMETA-BEGIN
Script-ID=com.example.lockedTool
Author=Example Author
Edit-Password-SHA256=salt123:41eb6f7f12752e69bb7c7a847624085953d719537fe558f7ce0105061f543a8d
Description-BEGIN
Locked editor example
Description-END
SCRIPTMETA-END
```

Expected:

- accepted as local item
- `author = Example Author`
- `editPasswordSHA256` is parsed as a raw value
- not update-checkable unless both `Version` and `Meta-URL` are also present
- editor UI must require password before editable open

### Description Marker Text

Input:

```text
SCRIPTMETA-BEGIN
Script-ID=com.example.desc
Description-BEGIN
■ Description-END
Description-END です
本文 Description-END
Description-END
SCRIPTMETA-END
```

Expected description:

```text
■ Description-END
Description-END です
本文 Description-END
```

### Unsupported Localized Tags

Input:

```text
SCRIPTMETA-BEGIN
Script-ID=com.example.localized
Description-en-BEGIN
This is not a v1.4 description block.
Description-en-END
Name-ja=日本語名
SCRIPTMETA-END
```

Expected:

- item is accepted because `Script-ID` exists
- `Description-en-BEGIN` is not treated as `Description-BEGIN`
- `Description-en-END` is not treated as `Description-END`
- `Name-ja` is ignored and is not treated as `Name`
- no localized or suffixed tag is normalized into a canonical tag

### Description BEGIN Inside Description

Input:

```text
SCRIPTMETA-BEGIN
Script-ID=com.example.begin
Description-BEGIN
first
Description-BEGIN
second
Description-END
SCRIPTMETA-END
```

Expected description:

```text
first
Description-BEGIN
second
```

### Distribution Redirect Preference

Input record:

```text
SCRIPTMETA-DIST-BEGIN
Script-ID=com.example.tool
Version=1.0
Latest-URL=https://example.com/latest-meta
SCRIPTMETA-DIST-END
```

Expected:

- follow `Latest-URL`
- do not stop at `Version=1.0`

### GitHub Directory Source Loading

Input `Meta-URL`:

```text
https://github.com/Yamonov/Iwashiya_Scripts/tree/main/Illustrator
```

Expected:

- detect GitHub directory mode
- fetch `https://raw.githubusercontent.com/Yamonov/Iwashiya_Scripts/main/Illustrator/SCRIPTMETA.txt`
- accept only valid SCRIPTMETA text from that raw URL
- keep the original GitHub directory URL as the user-facing `resolvedURL`
- do not parse the GitHub directory page HTML as metadata

### Multiple GitHub Directory Sources

Input `Meta-URL` values:

```text
https://github.com/Yamonov/Iwashiya_Scripts/tree/main/Photoshop
https://github.com/Yamonov/Iwashiya_Scripts/tree/main/Illustrator
```

Expected:

- treat them as two distinct update source groups
- fetch `https://raw.githubusercontent.com/Yamonov/Iwashiya_Scripts/main/Photoshop/SCRIPTMETA.txt`
- fetch `https://raw.githubusercontent.com/Yamonov/Iwashiya_Scripts/main/Illustrator/SCRIPTMETA.txt`
- do not fetch or parse `https://github.com/Yamonov/Iwashiya_Scripts/tree/main` to discover child directories
- keep each original GitHub directory URL as that source's user-facing `resolvedURL`

### Distribution Page Without Self URL

Correct latest-page distribution record:

```text
SCRIPTMETA-DIST-BEGIN
Script-ID=com.example.tool
Version=1.1
SCRIPTMETA-DIST-END
```

Incorrect generated fields:

```text
Meta-URL=https://example.com/current-scripmeta.txt
URL=https://example.com/current-scripmeta.txt
Page-URL=https://example.com/current-scripmeta.txt
Latest-URL=https://example.com/current-scripmeta.txt
```

Expected:

- no `Meta-URL` exists in the distribution record
- no self-reference URL field is required
- no `Latest-URL` exists when the latest version is already on the fetched page
- if an unknown URL field exists, ignore it
- if `Latest-URL` points to the current fetched page, treat it as same-page and
  do not follow it

### Legacy Distribution Block Filtering

Input page contains two legacy blocks:

```text
SCRIPTMETA-BEGIN
Script-ID=com.example.local
Version=1.0
Meta-URL=https://example.com/meta
SCRIPTMETA-END

SCRIPTMETA-BEGIN
Script-ID=com.example.local
Version=1.1
SCRIPTMETA-END
```

Expected:

- first block is excluded because it contains `Meta-URL`
- second block is used as distribution metadata
