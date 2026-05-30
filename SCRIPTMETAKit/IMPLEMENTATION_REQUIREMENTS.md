# SCRIPTMETAKit Rust Implementation Requirements

This document lists the implementation items needed to rebuild Scripta's
SCRIPTMETA behavior as a Rust library.

The first supported development environment is macOS. The library must still be
designed as platform-neutral Rust so the same crate can later be used on
Windows.

See `PACKAGE_INTEGRATION_IO_JA.md` for the first package-integration milestone
covering Scripta and ACEMenu inputs, outputs, monitoring, and cache policy.
See `PHASE1_PUBLIC_API_AND_MODULES_JA.md` for the Phase 1 public API and Rust
module plan.

## Reference Implementation

Use these Scripta files as the current behavior reference.

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

The closest single source is `ScriptMetaSupport.swift`. It contains local
metadata models, scan logic, cache data, script-side parsing, distribution
parsing, Meta-URL loading, update checking, progress events, retry behavior, and
Latest-URL resolution.

## Rust Package Goal

Implement a reusable Rust crate named `scriptmetakit`.

The crate should provide:

- script-side SCRIPTMETA parsing
- distribution-side SCRIPTMETA parsing
- version normalization and comparison
- Meta-URL normalization and validation
- local script file scanning
- multi-root scan results
- candidate cache support for incremental rescans
- update checking and distribution resolution
- progress events for UI consumers
- editor helper behavior that is not tied to AppKit
- optional cache serialization helpers

The crate should not provide:

- AppKit or SwiftUI UI
- automatic script download
- automatic script install
- Sparkle app update behavior
- Scripta-specific UserDefaults behavior
- Scripta-specific settings screen layout
- macOS-only behavior in core modules

## Proposed Rust Module Split

Use this split unless implementation work proves another split is simpler.

- `core`
  - shared models
  - known keys and block markers
  - text decoding
  - script-side parser
  - distribution-side parser
  - version normalization and comparison
  - URL normalization
  - validation helpers
- `scanner`
  - supported extension filtering
  - local file reading limit
  - recursive root scan
  - hidden/package skip policy
  - symlink-aware identity paths
  - candidate records
  - deduplication
  - incremental root refresh
- `resolver`
  - Meta-URL source classification
  - gist raw SCRIPTMETA.txt loading
  - GitHub repository raw SCRIPTMETA.txt loading
  - GitHub directory raw SCRIPTMETA.txt loading
  - generic text loading
  - streamed read limit
  - retry policy
  - Latest-URL chain resolution
  - per-check source and parsed-record caches
- `catalog`
  - multi-root state
  - root-aware scan result
  - persisted cache schemas
  - update-result preservation
  - app-controlled storage hooks
- `editor`
  - editable document parsing
  - field validation
  - block rendering
  - block insertion and replacement
  - password hash generation and verification
  - backup metadata helpers

The `editor` module may be implemented after parser, scanner, and resolver.
Keep it separate so app consumers can use the library without editor behavior.

## Platform Policy

Shared APIs and data models must compile on both macOS and Windows. Actual
script metadata extraction will need platform-specific behavior. Do not avoid
that requirement; isolate it behind platform adapters.

- Use `Path` / `PathBuf` for file paths.
- Use `url::Url` for URLs.
- Avoid direct `std::os::unix` or `std::os::windows` dependencies in shared
  modules.
- Put macOS alias-file resolution, compiled AppleScript decompilation, and
  FSEvents behind a macOS adapter.
- Put Windows shortcut resolution, VBA / Office macro project extraction, and
  ReadDirectoryChangesW behind a Windows adapter.
- Symlink resolution can use `std::fs::canonicalize` where available.
- Windows path comparison must avoid assuming `/` separators.
- Cache schemas must serialize without platform-specific types.
- Dates should be represented with a portable type, for example `time` or
  `chrono`, or as ISO-8601 strings at the boundary.

Current Scripta resolves macOS alias files and symlinks. The Rust library should
support symlinks cross-platform first. macOS alias support belongs in a
target-specific adapter because it depends on Apple APIs.

Platform adapters own:

- macOS: decompile `.scpt` with `osadecompile` or OSA APIs before parsing SCRIPTMETA
- macOS: alias-file resolution
- macOS: FSEvents monitoring
- Windows: `.lnk` and related shortcut resolution
- Windows: extracting module text from `.bas`, `.vbs`, `.vba`, Office documents, and add-ins
- Windows: ReadDirectoryChangesW monitoring

Shared core owns:

- common root / file / metadata / cache / update-check models
- parsing text that already contains SCRIPTMETA
- extension policy, runtime hints, deduplication, and cache schema
- consuming text and diagnostics returned by platform adapters

## Public Data Models

These are the minimum consumer-facing models.

### Script Metadata Item

Equivalent to Scripta's `ScriptMetaListItem`.

Fields:

- `root_id`
  - app-defined root identifier
  - Swift owns Illustrator / Photoshop / InDesign classification
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

Derived fields:

- `id`
  - stable item id, normally normalized file path
- `file_name`
- `folder_path`
- `is_update_checkable`
  - true when `version` and `meta_url` are present
- `has_edit_password`
  - true when `edit_password_sha256` is non-empty

Important behavior:

- `name` must be the raw `Name` tag value.
- Do not replace `name` with the file name.
- Let app consumers decide display fallback.

### Distribution Resolution

Equivalent to Scripta's `ScriptMetaDistributionResolution`.

Fields:

- `latest_version`
- `latest_page_url`
- `final_page_url`
- `latest_url_history`
- `checked_at`
- `is_unresolved`
- `note`

Notes:

- `latest_page_url` is the last followed `Latest-URL`, not necessarily the final
  fetched source URL.
- `final_page_url` is the source URL that provided the final distribution
  record.
- `latest_url_history` stores every encountered `Latest-URL` in order, including
  the final diagnostic URL when a same-page or circular reference is ignored.
- `is_unresolved` is true when a final latest version could not be determined.

### Update Check Result

Equivalent to Scripta's `ScriptMetaUpdateCheckResult`.

Fields:

- `resolutions_by_item_id`
- `failures_by_item_id`
- `errors_by_item_id`
- `checked_at`

`failures_by_item_id` stores structured failure data for UI and host app handling.
Each record includes `code`, `message`, `item_id`, `file_path`, `script_id`,
`current_version`, `meta_url`, `source_url`, and `checked_at`.
`errors_by_item_id` remains as a compatibility string map.

### Candidate Cache

Equivalent to Scripta's `ScriptMetaCandidateCache`.

Fields:

- `schema_version`
- `built_at`
- `registered_directories`
- `records`

Each candidate record stores:

- `root_path`
- `file_path`
- `identity_path`
- `file_size`
- `content_modified_at`
- parsed `item` or `None`

Purpose:

- avoid reparsing unchanged files
- preserve records for files without SCRIPTMETA too
- allow incremental refresh of a single root

### Root-Aware Scan Result

The Rust API should expose more context than Scripta's current flat result.

Suggested fields:

- `roots`
  - one entry per requested root
  - includes root id, path, status, item count, and errors
- `all_items`
  - deduplicated item list for app display and update checks
- `file_items`
  - per-file item list for file-list metadata lookup
- `candidate_cache`

This preserves the app-facing convenience of Scripta while avoiding loss of
root-level errors or freshness state.

## Known Tags and Keys

Use the v1.4 closed syntax.

Markers:

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

Do not add localized keys or additional block types.

Do not interpret these as valid v1.4 keys:

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

## Script-Side Parser Requirements

Input:

- full script text or the first scanned chunk of a local file

Block selection:

- find the first `SCRIPTMETA-BEGIN` to `SCRIPTMETA-END` block
- preprocess AppleScript line comments before parsing
- for lines whose trimmed content starts with `--`, remove the leading `--` and
  surrounding whitespace
- normalize CRLF and CR to LF before line-based parsing

Description parsing:

- `Description-BEGIN` starts a multiline description only when the trimmed whole
  line equals `Description-BEGIN`
- `Description-END` ends the description only when the trimmed whole line equals
  `Description-END`
- keep inner line breaks
- trim outer whitespace and newlines from the final description
- do not parse keys inside the description body
- `Description-END` embedded in other text is not a closing marker

Key-value parsing:

- remove the description block before parsing key-value pairs
- parse only lines containing `=`
- key is the trimmed text before the first `=`
- value is the trimmed text after the first `=`
- empty lines are ignored
- unknown keys are ignored by normal parser output
- duplicate keys can be last-write-wins, matching Scripta's dictionary behavior

Line repair:

- Scripta normalizes some missing-newline cases before known keys using a known
  key regex.
- Rust must support the same behavior so text like
  `Script-ID=a Version=1.0 Meta-URL=https://example.com` can still be split
  before known keys.

Required fields:

- Local Subset requires `Script-ID`
- Update Profile requires `Script-ID`, `Version`, and `Meta-URL` by
  specification
- local scanning accepts Local Subset, so missing `Version` or `Meta-URL` is not
  an error for scan inclusion

Validation:

- missing `Script-ID` is an error
- present invalid `Version` is an error
- present invalid `Meta-URL` is an error
- `META-URL` is accepted as an alias for `Meta-URL`
- `Edit-Password-SHA256` is returned as text even if malformed
- malformed `Edit-Password-SHA256` must not exclude a scanned item

## Distribution Parser Requirements

Input:

- text from a distribution page or `SCRIPTMETA.txt`

Block selection:

- read the first `SCRIPTMETA-DIST-BEGIN` to `SCRIPTMETA-DIST-END` block
- if no distribution block exists, return a parse error
- do not use local script `SCRIPTMETA-BEGIN` to `SCRIPTMETA-END` blocks as distribution metadata

Record parsing:

- remove description blocks before parsing
- split records on `Script-ID`
- each `Script-ID` begins a new record
- parse key-value pairs with the same first-`=` rule
- ignore records without `Script-ID`
- map records by `Script-ID`

Version fields:

- `Version` is preferred
- `Latest-Version` is fallback
- normalize and validate version before returning it

Next-page URL fields:

- `Latest-URL` is preferred
- `Latest-Page-URL` is fallback
- normalize and validate URL before returning it

Distribution fields ignored for output:

- `Description`
- `Name`
- `Author`
- `Meta-URL`
- `Edit-Password-SHA256`

## Text Decoding Requirements

Local scan decoder should try at least:

- UTF-8
- UTF-16
- UTF-16LE
- UTF-16BE
- Japanese EUC
- Shift JIS

Scripta scan currently tries UTF-8, Japanese EUC, Shift JIS, and UTF-16 in
`ScriptMetaTextDecoder`. Editor loading also checks BOM-marked UTF-8, UTF-16LE,
and UTF-16BE. Rust should combine these into one robust decoder.

Use `encoding_rs` or equivalent.

Decoder behavior:

- return decoded text and encoding when editor support needs to write back
- for scanning, returning text is enough
- empty files produce no metadata item
- invalid undecodable data should be a parse/read error, not a panic

## Version Requirements

Normalization:

- trim surrounding whitespace
- remove internal whitespace
- find the first numeric dotted sequence matching `\d+(?:\.\d+)*`
- return that matched sequence
- reject empty input
- reject if no numeric dotted sequence exists
- reject if components cannot be parsed as unsigned integers

Examples:

- `1.2.3` becomes `1.2.3`
- `v1.2.3` becomes `1.2.3`
- ` 1 . 2 ` becomes `1.2`

Comparison:

- split by `.`
- compare integer components
- pad missing components with zero
- `1.2` equals `1.2.0`
- invalid versions compare as equal only as a defensive fallback

## URL Requirements

Script-side parser normalization:

- trim whitespace and newlines
- remove surrounding single or double quotes
- accept URLs with scheme and host
- repair `http:/...` to `http://...`
- repair `https:/...` to `https://...`
- if no scheme is present, prefix `https://`

Editor validation is stricter:

- only `http` and `https` schemes are usable
- host is required
- direct file links are not allowed for `Meta-URL`

Disallowed direct Meta-URL suffixes:

- `.zip`
- `.js`
- `.jsx`
- `.jsxinc`
- `.applescript`
- `.idjs`
- `.jxa`
- `.psjs`

## Scanner Requirements

Supported extensions:

- `js`
- `jsx`
- `jsxbin`
- `jsxinc`
- `scpt`
- `applescript`
- `jxa`
- `idjs`
- `psjs`

Runtime hint:

- `.js` and `.applescript` are treated as JXA only when their leading bytes match Scripta's `#!/usr/bin/osascript -l JavaScript` prefix.
- `.js` without a JXA shebang is treated as Adobe JavaScript.
- `.applescript` without a JXA shebang and `.scpt` are treated as AppleScript.
- `.scpt` is compiled AppleScript. Plain text prefix scanning usually cannot read SCRIPTMETA comments from it. A macOS adapter should use `osadecompile` or OSA APIs to turn it into text first.
- On Windows, VBA / Office macro projects are also script sources. Binary container extraction belongs in the Windows adapter; the shared parser receives text that already contains SCRIPTMETA.

Local file read limit:

- read at most 128 KiB per candidate file for scanning

Directory traversal:

- recurse into registered directories
- skip hidden files and hidden directories
- skip package directories
- skip non-regular files
- avoid scanning the same resolved directory more than once
- accept cancellation checks during scan

Path identity:

- deduplicate using resolved identity path
- resolve symlinks where possible
- on macOS, alias-file resolution is a parity item but can be target-specific

Candidate cache reuse:

- reuse parsed item when identity path, file size, and content modification date
  match
- only reuse when both file size and modification date are available
- cache records even when the file has no parsed item
- use schema versioning

Changed path filter:

- ignore paths containing hidden intermediate components
- ignore files whose last path component starts with `.`
- if changed path has no extension, treat it as possibly metadata-relevant
- otherwise only supported extensions are relevant

Deduplication:

- deduplicate files by identity path first
- then choose one best item per `Script-ID`
- prefer item with a version over item without a version
- when both have versions, prefer the higher normalized version
- when versions are equal or absent, prefer the lexicographically earlier file
  path using natural/localized ordering equivalent where practical

Sort order:

- display items sorted by file name using natural ordering
- file-items sorted by full file path

## Update Checker Requirements

Eligibility:

- only items with both `version` and `meta_url` are update-checkable
- Local Subset items are scanned and displayed but skipped for update check

Grouping:

- group update-checkable items by `Meta-URL`
- one network source context is shared within a group
- Scripta checks up to 6 Meta-URL groups concurrently

Result:

- record a resolution per item id on success
- record structured failure data and a compatibility error string per item id on failure
- one item failing must not fail the entire check
- cancellation must stop the entire task

Progress events:

- event id is the Meta-URL string for network groups
- state values:
  - `active`
  - `active_error`
  - `persistent_error`
  - `finished`
- emit scan start and finish from the app/catalog layer
- emit item checking text before each script id resolution
- emit candidate URL loading text for gist/GitHub candidates
- emit retry text for retryable network failures
- emit persistent error when a group finishes with failures
- emit finished when a group finishes without failures

## Distribution Source Loading Requirements

Classify `Meta-URL` in this order:

1. gist URL
2. GitHub repository URL
3. GitHub directory URL
4. generic plain text URL

Gist URL:

- host contains `gist.github.com`
- path must contain at least `{user}/{gistID}`
- try raw candidates in order:
  - `https://gist.githubusercontent.com/{user}/{gistID}/raw/SCRIPTMETA.txt`
  - `https://gist.githubusercontent.com/{user}/{gistID}/raw/scriptmeta.txt`
  - `https://gist.github.com/{user}/{gistID}/raw/SCRIPTMETA.txt`
  - `https://gist.github.com/{user}/{gistID}/raw/scriptmeta.txt`
- accept the first candidate containing a SCRIPTMETA block
- if none work, return `gist_meta_file_not_found`

GitHub repository URL:

- host equals `github.com`
- path has exactly `{owner}/{repo}`
- try raw candidates in order:
  - `https://raw.githubusercontent.com/{owner}/{repo}/HEAD/SCRIPTMETA.txt`
  - `https://raw.githubusercontent.com/{owner}/{repo}/main/SCRIPTMETA.txt`
  - `https://raw.githubusercontent.com/{owner}/{repo}/master/SCRIPTMETA.txt`
- accept the first candidate containing a SCRIPTMETA block
- if none work, return `github_meta_file_not_found`

GitHub directory URL:

- host equals `github.com`
- path is `{owner}/{repo}/tree/{branch}/{directory...}`
- load:
  - `https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{directory...}/SCRIPTMETA.txt`
- do not crawl child directories
- do not parse GitHub HTML as canonical update metadata

Generic URL:

- fetch the URL as text
- parse SCRIPTMETA from the fetched text

HTTP behavior:

- request timeout is 15 seconds
- ignore local cache
- send `Cache-Control: no-cache`
- send `Pragma: no-cache`
- accept only HTTP 2xx responses
- non-HTTP response is invalid

Streaming behavior:

- read the source stream until the distribution metadata block is found or EOF is reached
- do not truncate the overall source at a fixed byte count
- scan for begin and end markers while streaming
- detect markers even when they are split across read chunk boundaries
- treat only `SCRIPTMETA-DIST-BEGIN` / `SCRIPTMETA-DIST-END` as distribution metadata
- return a parse error when no `SCRIPTMETA-DIST` block is found by EOF
- do not retain bytes before the begin marker
- once an end marker has been found, discard bytes after it
- decode only the collected metadata block bytes with the text decoder
- the default `max_metadata_block_bytes` 256 KiB limit applies to the retained metadata block, not to the full source stream

Retry behavior:

- retry only:
  - TLS/secure connection failure
  - network connection lost
  - timeout
- retry delays:
  - 1 second
  - 3 seconds
- after retries are exhausted, return the original network error

## Latest-URL Resolution Requirements

Input:

- start URL
- target script id

Process:

- load source from current URL
- parse distribution records
- find target `Script-ID`
- if absent, return `record_not_found`
- if record has `Latest-URL` and it is a new unvisited URL, follow it
- continue until a record without a followable `Latest-URL` is reached

Limits:

- maximum redirect count is 8
- detect same-page URL
- detect circular references
- store visited URL strings

Final resolution:

- if final record has `latest_version`, return it
- if final record has no version, return unresolved with a version-missing note
- if `Latest-URL` points to the same page, return unresolved or latest version
  from the same record with a same-page ignored note, matching Scripta's current
  resolution shape
- if `Latest-URL` is circular, return unresolved or latest version from the
  current record with a circular-reference note
- if redirect count exceeds the limit, return unresolved with too-many-redirects
  note

Source caches during one check:

- cache loaded sources by original URL string
- maximum source cache count: 4
- cache parsed distribution records by resolved source URL string
- maximum parsed cache count: 8
- evict oldest entries first

## Cache Requirements

Scripta has two cache files:

- `ScriptMetaUpdateCache.json`
- `ScriptMetaDisplayCache.json`

Rust should make cache storage app-controlled. The library can provide
serializable structures and helper functions, but the app should decide the
directory.

Snapshot cache contains:

- source revision id
- deduplicated items
- update check result
- candidate cache

Display cache contains:

- schema version
- source revision
- source cache fingerprint
- registered directories
- deduplicated items
- file items
- update check result

Cache size limits from Scripta:

- update cache maximum: 50 MiB
- display cache maximum: 8 MiB

Update-result preservation:

- preserve cached result only for rescanned item ids that still exist
- script id must match
- version must match
- meta URL must match
- drop stale resolutions and errors

Save behavior:

- write atomically where practical
- use schema versioning
- never panic on corrupt cache
- corrupt or mismatched schema cache should be ignored

## Editor Helper Requirements

This section can be implemented after scanner and resolver, but keep the
requirements because Scripta currently includes this behavior.

Editable extensions:

- `js`
- `jsx`
- `jsxinc`
- `applescript`
- `idjs`
- `jxa`
- `psjs`

Obfuscated JSX detection:

- probe `js`, `jsx`, and `jsxbin`
- read first 8 bytes
- if prefix equals `@JSXBIN`, treat as obfuscated and not editable
- `jsxbin` is not editable

Editable file limit:

- maximum editable size is 10 MiB

Editor modes:

- `local_subset`
- `update_profile`

Visible fields:

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

Required fields when rendering:

- Local Subset:
  - `Script-ID`
- Update Profile:
  - `Script-ID`
  - `Version`
  - `Meta-URL`

Additional fields:

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

Save validation:

- `Script-ID` is required
- `Script-ID` must match `^[A-Za-z0-9._-]+$`
- new or changed `Script-ID` must not conflict with another registered script
- Update Profile requires valid `Version`
- Update Profile requires valid `Meta-URL`
- Local Subset may have `Version`, but it must be valid if present
- Local Subset removes `Meta-URL` and `Min-Target-Version`
- `Edit-Password-SHA256` requires `Author`
- empty password update is invalid
- malformed stored password hash is invalid for editor password checking

Password hash:

- stored format is `<salt>:<sha256>`
- salt is 16 alphanumeric characters
- sha256 is lowercase hex and 64 characters
- digest input is `{salt}:{password}`
- hash algorithm is SHA-256
- this is only an editor guard, not author proof

Generated local Script-ID:

- format: `local.{user_component}.{file_component}_{suffix}`
- components allow letters, digits, underscore, and hyphen
- unsupported characters become `_`
- collapse repeated underscores during sanitization
- trim leading and trailing `_` and `-`
- suffix is 12 lowercase alphanumeric characters
- fall back to UUID-derived suffix if repeated attempts conflict

Release date:

- write format is `yyyy-MM-dd`
- accepted read formats:
  - `yyyy-MM-dd`
  - `yyyy/MM/dd`
  - `yyyy.MM.dd`

Block comments:

- JavaScript-like scripts use `/* ... */`
- AppleScript uses `(* ... *)`
- when writing metadata, escape closing delimiters inside metadata text:
  - `*/` becomes `* /`
  - `*)` becomes `* )`

Block detection for editing:

- find the first `SCRIPTMETA-BEGIN` to `SCRIPTMETA-END` range
- accept it if it is inside the expected block comment syntax
- accept it if it is standalone on its own lines
- otherwise continue searching

Insertion:

- if an existing metadata block is in the leading header, replace it in place
- if an existing block is outside the leading header, remove it and insert a new
  wrapped block near the top
- if there is a leading block comment, insert inside it before the closing
  delimiter
- otherwise insert after leading target directives beginning with `#`
- preserve original line ending when writing

Backups:

- backup before save
- backup before restore
- store backup index as JSON
- include file size and SHA-256 digest
- support resetting backups while keeping current file as initial generation

Backup storage is application behavior. The Rust library can expose helper
types, but the app should choose the actual directory.

## Settings and File List Integration Requirements

These are app-facing behaviors, but the Rust result shape should make them easy.

Settings update check:

- scan registered folders first
- display scan progress
- update the current item list after scan
- mark all rescanned items as checking
- run update check only for checkable items
- apply result by filtering stale item ids
- cache the refreshed snapshot

Card status:

- loading when item id is currently checking
- failed when item id has an error
- idle when no resolution exists
- failed when resolution exists but `latest_version` is missing
- idle when item has no current version
- update available when current version is lower than latest version
- up to date otherwise

Update link:

- prefer `latest_page_url`
- fall back to `final_page_url`

Section grouping:

- group regular items by trimmed `target_app`
- use app-provided "other" title when target app is absent
- put update-available items in a priority update section
- put failed items in a priority error section
- sort normal target sections by title, keeping "other" last
- sort items by status priority and file name

Status priority:

1. update available
2. failed
3. loading
4. up to date
5. idle

File-list metadata:

- file list must not parse script files on hover
- load metadata from display cache / catalog output
- key metadata by normalized script path
- match by resolved URL first, then display URL fallback
- include file name, script id, name, author, description, version, target app,
  release date, and edit-password presence
- search should include script id, name, author, description, version, target
  app, and release date
- Local Subset can show description and `Name` even when not update-checkable

## AppleScript Automation Metadata

Scripta also exposes metadata to AppleScript. Rust does not need AppleScript
support directly, but the public model should preserve the fields needed by the
host app:

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

## Error Categories

Expose typed errors. Let the app localize messages.

Script metadata errors:

- `block_not_found`
- `script_id_missing`
- `version_missing`
- `version_invalid`
- `meta_url_missing`
- `meta_url_invalid`

Local scan behavior:

- `block_not_found` means the file is ignored
- missing `Script-ID` means the file is ignored or reported depending on API
- missing `Version` does not reject a Local Subset
- missing `Meta-URL` does not reject a Local Subset
- invalid present `Version` rejects the parsed item
- invalid present `Meta-URL` rejects the parsed item

Distribution errors:

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

Resolution notes:

- same-page latest URL ignored
- circular latest URL
- version missing
- too many redirects

Editor errors:

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

## Test Requirements

### Parser Tests

Cover:

- Local Subset with only `Script-ID`
- Update Profile with all required fields
- missing block
- missing `Script-ID`
- missing `Version` accepted for Local Subset
- missing `Meta-URL` accepted for Local Subset
- invalid present version rejected
- invalid present Meta-URL rejected
- `META-URL` alias
- `Description` with nested key-looking text
- `Description-END` inside a sentence not closing the block
- CRLF and CR line endings
- AppleScript `--` comment preprocessing
- missing-newline repair before known keys
- unknown keys ignored
- duplicate known keys last-write-wins

### Distribution Parser Tests

Cover:

- preferred `SCRIPTMETA-DIST` block
- missing `SCRIPTMETA-DIST` block
- multiple records split by `Script-ID`
- missing target record
- `Version`
- `Latest-Version`
- `Latest-URL`
- `Latest-Page-URL`
- invalid latest URL
- invalid version
- distribution description ignored

### Version Tests

Cover:

- `1`
- `1.2`
- `1.2.0`
- `v1.2.3`
- whitespace around and inside version
- invalid empty version
- invalid non-numeric version
- component padding in comparison

### URL Tests

Cover:

- regular https URL
- quoted URL
- missing scheme
- `http:/` repair
- `https:/` repair
- no host rejected
- direct file suffix rejected by editor validation

### Scanner Tests

Cover:

- supported extensions
- unsupported extension ignored
- hidden file ignored
- hidden directory ignored
- package directory ignored where detectable
- read limit
- symlink identity deduplication
- duplicate `Script-ID` chooses higher version
- duplicate `Script-ID` tie chooses earlier path
- candidate cache reuse
- changed path relevance filter
- root-aware result with missing/unreadable root
- cancellation

### Resolver Tests

Cover with mocked HTTP:

- generic text URL
- gist candidate order
- gist not found
- GitHub repository HEAD/main/master order
- GitHub directory raw URL
- HTTP non-2xx
- response without SCRIPTMETA block
- stream truncation after end marker
- retryable errors retried
- non-retryable errors not retried
- max byte limit
- cancellation

### Latest-URL Tests

Cover:

- direct latest version
- follow Latest-URL
- Latest-URL preferred over Version
- same-page URL note
- circular URL note
- too many redirects
- final version missing
- source cache reuse
- parsed records cache reuse

### Cache Tests

Cover:

- serialize and deserialize snapshot
- schema mismatch ignored
- corrupt JSON ignored
- stale update result removed
- unchanged item preserves update result
- display cache has file items and deduplicated items

### Editor Tests

Cover:

- editable extension detection
- JSXBIN signature detection
- field visibility by mode
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

## Suggested Implementation Phases

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
- tests with temporary directories

### Phase 3: Resolver

- HTTP abstraction
- source loader
- gist/GitHub URL handling
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
- write-back helpers if needed

### Phase 6: App Integration

- connect `scriptmetakitApp`
- show scan result
- show update status
- exercise macOS UI flow
- keep this outside the Rust core crate

### Revisit Later

- Script file replacement is out of scope for the current specification. Revisit only if the specification changes.

## Dependency Candidates

Keep dependencies small and cross-platform.

- `serde`
- `serde_json`
- `thiserror`
- `url`
- `encoding_rs`
- `walkdir`
- `reqwest`
- `tokio`
- `sha2`
- `time` or `chrono`
- `tempfile` for tests

Use feature flags if networking or editor support should be optional.

Suggested features:

- `default = ["scanner", "resolver"]`
- `scanner`
- `resolver`
- `editor`
- `cache`

## Non-Negotiable Behavior

- Do not require `Version` for Local Subset scan inclusion.
- Do not require `Meta-URL` for Local Subset scan inclusion.
- Do not auto-download scripts.
- Do not auto-install scripts.
- Do not use GitHub Releases as canonical update metadata.
- Do not parse GitHub HTML as canonical update metadata when a repository or
  directory Meta-URL is given.
- Do not invent localized keys or markers.
- Do not treat `Edit-Password-SHA256` as author verification.
- Do not parse descriptions on distribution pages for app display.
- Do not follow `Latest-URL` without loop and depth protection.
- Keep `Name` as a metadata field, separate from file name and display fallback.
