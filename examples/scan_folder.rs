use std::{
    env,
    path::{Path, PathBuf},
    process,
};

use scriptmetakit::{
    ScanMode, ScriptMetaCatalogSnapshot, ScriptMetaKitConfig, ScriptMetaKitEngine,
    UpdateCheckRequest,
};

fn main() {
    let args = match Args::parse(env::args().skip(1)) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            print_usage();
            process::exit(2);
        }
    };

    if args.help {
        print_usage();
        return;
    }

    if args.folders.is_empty() {
        eprintln!("missing folder path");
        print_usage();
        process::exit(2);
    }

    for folder in &args.folders {
        if !folder.is_dir() {
            eprintln!("not a directory: {}", folder.display());
            process::exit(2);
        }
    }

    if let Err(error) = run(args) {
        eprintln!("error: {error}");
        process::exit(1);
    }
}

fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = ScriptMetaKitConfig::new("scan_folder", "scan_folder");

    if args.http {
        config.update_check.request_timeout_millis = Some(15_000);
    }

    let mut engine = ScriptMetaKitEngine::new(config)?;
    let mut result = engine.scan_root_paths(args.folders, args.mode)?;

    if args.check_updates {
        let items = result
            .catalog_snapshot
            .as_ref()
            .map(|snapshot| snapshot.all_items.clone())
            .unwrap_or_default();
        let update_result = pollster::block_on(engine.check_updates(UpdateCheckRequest { items }))?;
        if args.json {
            if let Some(catalog) = &mut result.catalog_snapshot {
                catalog.update_check_result = Some(update_result.clone());
            }
            println!("{}", serde_json::to_string_pretty(&result)?);
            return Ok(());
        }

        print_scan_summary(&result);
        println!();
        println!("Update check");
        println!("------------");
        if update_result.statuses_by_item_id.is_empty() {
            println!("No update-checkable items.");
        }
        for (item_id, status) in update_result.statuses_by_item_id {
            let latest_version = update_result
                .resolutions_by_item_id
                .get(&item_id)
                .and_then(|resolution| resolution.latest_version.as_deref())
                .unwrap_or("-");
            let failure = update_result.failures_by_item_id.get(&item_id);
            if let Some(failure) = failure {
                println!(
                    "- {item_id}: {status:?}, latest={latest_version}, error={} ({})",
                    failure.message, failure.code
                );
            } else {
                println!("- {item_id}: {status:?}, latest={latest_version}");
            }
        }
    } else {
        if args.json {
            println!("{}", serde_json::to_string_pretty(&result)?);
            return Ok(());
        }

        print_scan_summary(&result);
    }

    Ok(())
}

fn print_scan_summary(result: &scriptmetakit::ScanResult) {
    println!("SCRIPTMETAKit scan");
    println!("==================");
    for root in &result.roots {
        println!("Root: {}", root.path.display());
        println!("  id: {}", root.root_id);
        println!("  status: {:?}", root.status);
        println!("  dirty: {}", root.is_dirty);
        println!("  item count: {}", root.item_count);
        if let Some(error) = &root.error {
            println!("  error: {} {}", error.code, error.message);
        }
    }

    if !result.file_list_snapshots.is_empty() {
        println!();
        println!("File list");
        println!("---------");
        for snapshot in &result.file_list_snapshots {
            let child_count = snapshot.children.as_ref().map_or(0, Vec::len);
            println!(
                "- {}: top-level entries={}, directories tracked={}, truncated={}",
                snapshot.root.path.display(),
                child_count,
                snapshot.directory_states.len(),
                snapshot.truncated
            );
        }
    }

    if let Some(catalog) = &result.catalog_snapshot {
        print_catalog(catalog);
    }
}

fn print_catalog(catalog: &ScriptMetaCatalogSnapshot) {
    println!();
    println!("SCRIPTMETA catalog");
    println!("------------------");
    println!("all_items: {}", catalog.all_items.len());
    println!("file_items: {}", catalog.file_items.len());
    println!(
        "candidate_records: {}",
        catalog.candidate_cache.records.len()
    );

    for item in &catalog.all_items {
        println!();
        println!("- {}", item.file_path.display());
        println!("  Script-ID: {}", item.script_id);
        println!("  Version: {}", item.version.as_deref().unwrap_or("-"));
        println!("  Name: {}", item.name.as_deref().unwrap_or("-"));
        println!("  Author: {}", item.author.as_deref().unwrap_or("-"));
        println!(
            "  Meta-URL: {}",
            item.meta_url.as_ref().map_or("-", |url| url.as_str())
        );
        println!(
            "  Update checkable: {}",
            if item.is_update_checkable() {
                "yes"
            } else {
                "no"
            }
        );
    }
}

#[derive(Clone, Debug)]
struct Args {
    folders: Vec<PathBuf>,
    mode: ScanMode,
    json: bool,
    check_updates: bool,
    http: bool,
    help: bool,
}

impl Args {
    fn parse(values: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut args = Self {
            folders: Vec::new(),
            mode: ScanMode::FileListAndMetadata,
            json: false,
            check_updates: false,
            http: false,
            help: false,
        };

        let mut iterator = values.into_iter();
        while let Some(value) = iterator.next() {
            match value.as_str() {
                "-h" | "--help" => args.help = true,
                "--json" => args.json = true,
                "--check-updates" => args.check_updates = true,
                "--http" => args.http = true,
                "--mode" => {
                    let Some(mode) = iterator.next() else {
                        return Err("--mode requires a value".to_string());
                    };
                    args.mode = parse_mode(&mode)?;
                }
                _ if value.starts_with('-') => return Err(format!("unknown option: {value}")),
                _ => {
                    args.folders.push(Path::new(&value).to_path_buf());
                }
            }
        }

        if args.http && !args.check_updates {
            args.check_updates = true;
        }

        Ok(args)
    }
}

fn parse_mode(value: &str) -> Result<ScanMode, String> {
    match value {
        "both" | "file-list-and-metadata" => Ok(ScanMode::FileListAndMetadata),
        "metadata" | "metadata-only" => Ok(ScanMode::MetadataOnly),
        "file-list" | "file-list-only" => Ok(ScanMode::FileListOnly),
        _ => Err(format!("unknown mode: {value}")),
    }
}

fn print_usage() {
    eprintln!(
        "usage: cargo run --example scan_folder -- [options] <folder>...\n\
\n\
options:\n\
  --mode <both|metadata|file-list>  Scan mode. Default: both\n\
  --json                            Print full JSON result\n\
  --check-updates                   Run update check for items with Version and Meta-URL\n\
  --http                            Enable HTTP update checks. Requires --features blocking-http\n\
  -h, --help                        Show this help"
    );
}
