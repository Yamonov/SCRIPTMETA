# SCRIPTMETAKit

Rust library package for SCRIPTMETA parsing and scanning.

This package is kept separate from `scriptmetakitApp`. The app can later import this crate as a package for manual and UI testing.

The first development target is macOS, but the shared API should stay portable
so it can also support Windows later. Platform-specific behavior is expected for
script metadata extraction, file watching, alias/shortcut handling, and host
script formats. Keep those parts behind target-specific adapters or feature
gates, while keeping the public root, file, metadata, cache, and update models
portable.

## Design Documents

- `IMPLEMENTATION_REQUIREMENTS.md`
- `IMPLEMENTATION_REQUIREMENTS_JA.md`
- `PACKAGE_INTEGRATION_IO_JA.md`
- `PHASE1_PUBLIC_API_AND_MODULES_JA.md`
- `SCRIPT_FORMAT_SUPPORT_JA.md`

## Development Commands

```sh
cargo fmt --check
cargo check
cargo check --target x86_64-pc-windows-msvc
cargo check --features native-watch
cargo check-all
cargo test-all
```

`native-watch` enables the optional `notify` based watcher. Keep watcher code
behind this feature or target-specific modules so the core library remains
portable.

The storage module also provides JSON cache helpers:

- `save_cache_payload(path, payload)`
- `load_cache_payload(path)`

## Manual Folder Scan

Use the example CLI to scan a real script folder before connecting this library
to Scripta or ACEMenu.

```sh
cargo run --example scan_folder -- /path/to/scripts
cargo run --example scan_folder -- /path/to/scripts-a /path/to/scripts-b
```

Useful options:

```sh
cargo run --example scan_folder -- --mode metadata /path/to/scripts
cargo run --example scan_folder -- --json /path/to/scripts
cargo run --example scan_folder -- --check-updates /path/to/scripts
cargo run --features blocking-http --example scan_folder -- --http /path/to/scripts
```

When multiple folders are passed, each folder is registered as an independent
root. The JSON result keeps root-aware data in `roots` and
`file_list_snapshots`, while `catalog_snapshot.all_items` provides the merged
metadata list for app display.

HTTP/HTTPS update checks require the `blocking-http` feature. `Meta-URL` values
that point to `gist.github.com/<owner>/<id>` use Scripta-compatible raw
candidate lookup. GitHub repository URLs use `HEAD`, `main`, then `master`
`SCRIPTMETA.txt` candidates, and GitHub directory URLs such as
`github.com/<owner>/<repo>/tree/<branch>/<directory>` load that directory's raw
`SCRIPTMETA.txt`. Update resolutions also include `latest_url_history` for
diagnosing `Latest-URL` chains.

## Test App

`scriptmetakitApp` is a macOS test harness. It calls the Rust FFI layer, shows
the scanned file list and SCRIPTMETA items, can run update checks, stores the
last scan in Application Support, and can monitor the selected folder for
changes.

```sh
script/build_and_run.sh
```
