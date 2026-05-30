#[test]
fn imports_scriptmetakit_crate() {
    assert_eq!(scriptmetakit::package_name(), "scriptmetakit");
}

#[test]
fn parses_script_metadata_without_replacing_name() {
    let metadata = scriptmetakit::parse_script_metadata(
        r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.resize
// Version: 1.2.3
// Name: Human Title
// Author: Yamo
// Meta-URL: https://example.com/SCRIPTMETA.txt
// Description-BEGIN
// Resize selected object.
// Description-END
// SCRIPTMETA-END
"#,
    )
    .expect("metadata should parse");

    assert_eq!(metadata.script_id, "com.example.resize");
    assert_eq!(metadata.version.as_deref(), Some("1.2.3"));
    assert_eq!(metadata.name.as_deref(), Some("Human Title"));
    assert_eq!(
        metadata.description.as_deref(),
        Some("Resize selected object.")
    );
}

#[test]
fn parses_equals_style_script_metadata() {
    let metadata = scriptmetakit::parse_script_metadata(
        r#"
SCRIPTMETA-BEGIN
Script-ID=org.iwashi.Halftone_Generator
Version=1.1
Meta-URL=https://gist.github.com/Yamonov/6f00bd65e486513d82f773f858ac76cb
Name=Photoshopで疑似AMスクリーン生成
Description-BEGIN
Photoshop用のテストメタデータです。
Description-END
SCRIPTMETA-END
"#,
    )
    .expect("equals-style metadata should parse");

    assert_eq!(metadata.script_id, "org.iwashi.Halftone_Generator");
    assert_eq!(metadata.version.as_deref(), Some("1.1"));
    assert_eq!(
        metadata.meta_url.as_ref().map(|url| url.as_str()),
        Some("https://gist.github.com/Yamonov/6f00bd65e486513d82f773f858ac76cb")
    );
    assert_eq!(
        metadata.name.as_deref(),
        Some("Photoshopで疑似AMスクリーン生成")
    );
    assert_eq!(
        metadata.description.as_deref(),
        Some("Photoshop用のテストメタデータです。")
    );
}

#[test]
fn parses_distribution_metadata_for_matching_script_id() {
    let metadata = scriptmetakit::parse_distribution_metadata_for_script(
        r#"
SCRIPTMETA-DIST-BEGIN
Script-ID=org.example.first
Version=1.0.0
Script-ID=org.example.second
Version=2.5.0
SCRIPTMETA-DIST-END
"#,
        "org.example.second",
    )
    .expect("matching distribution metadata should parse");

    assert_eq!(metadata.script_id.as_deref(), Some("org.example.second"));
    assert_eq!(metadata.latest_version.as_deref(), Some("2.5.0"));
}

#[test]
fn parses_inline_distribution_metadata_from_note_description() {
    let metadata = scriptmetakit::parse_distribution_metadata_for_script(
        r#"
SCRIPTMETA-DIST-BEGIN Script-ID=com.kojirasetakuma.ai.datamergekit Version=2.5.0 Latest-URL=https://note.com/nice_lotus120/n/n1a3ab689bed6 Latest-Page-URL=https://note.com/nice_lotus120/n/n1a3ab689bed6 Release-Date=2026-05-21 SCRIPTMETA-DIST-END
"#,
        "com.kojirasetakuma.ai.datamergekit",
    )
    .expect("inline distribution metadata should parse");

    assert_eq!(
        metadata.script_id.as_deref(),
        Some("com.kojirasetakuma.ai.datamergekit")
    );
    assert_eq!(metadata.latest_version.as_deref(), Some("2.5.0"));
    assert_eq!(
        metadata.latest_url.as_ref().map(url::Url::as_str),
        Some("https://note.com/nice_lotus120/n/n1a3ab689bed6")
    );
    assert_eq!(
        metadata.latest_page_url.as_ref().map(url::Url::as_str),
        Some("https://note.com/nice_lotus120/n/n1a3ab689bed6")
    );
}

#[test]
fn parses_html_distribution_metadata_from_note_body() {
    let metadata = scriptmetakit::parse_distribution_metadata_for_script(
        r#"
SCRIPTMETA-DIST-BEGIN<br>Script-ID=com.kojirasetakuma.ai.datamergekit<br>Version=2.5.0<br>Latest-URL=<a href="https://note.com/nice_lotus120/n/n1a3ab689bed6" target="_blank" rel="noopener nofollow">https://note.com/nice_lotus120/n/n1a3ab689bed6</a><br>Latest-Page-URL=<a href="https://note.com/nice_lotus120/n/n1a3ab689bed6" target="_blank" rel="noopener nofollow">https://note.com/nice_lotus120/n/n1a3ab689bed6</a><br>Release-Date=2026-05-21<br>SCRIPTMETA-DIST-END
"#,
        "com.kojirasetakuma.ai.datamergekit",
    )
    .expect("HTML distribution metadata should parse");

    assert_eq!(
        metadata.script_id.as_deref(),
        Some("com.kojirasetakuma.ai.datamergekit")
    );
    assert_eq!(metadata.latest_version.as_deref(), Some("2.5.0"));
    assert_eq!(
        metadata.latest_url.as_ref().map(url::Url::as_str),
        Some("https://note.com/nice_lotus120/n/n1a3ab689bed6")
    );
    assert_eq!(
        metadata.latest_page_url.as_ref().map(url::Url::as_str),
        Some("https://note.com/nice_lotus120/n/n1a3ab689bed6")
    );
}

#[test]
fn treats_html_closing_blocks_as_distribution_line_breaks() {
    let metadata = scriptmetakit::parse_distribution_metadata_for_script(
        r#"
SCRIPTMETA-DIST-BEGIN</br>Script-ID=com.example.html</p><p>Version=3.1.0</p><p>Latest-Page-URL=https://example.com/script</p>SCRIPTMETA-DIST-END
"#,
        "com.example.html",
    )
    .expect("HTML block separators should parse");

    assert_eq!(metadata.script_id.as_deref(), Some("com.example.html"));
    assert_eq!(metadata.latest_version.as_deref(), Some("3.1.0"));
    assert_eq!(
        metadata.latest_page_url.as_ref().map(url::Url::as_str),
        Some("https://example.com/script")
    );
}

#[test]
fn rejects_legacy_script_metadata_for_distribution_metadata() {
    let error = scriptmetakit::parse_distribution_metadata_for_script(
        r#"
SCRIPTMETA-BEGIN
Script-ID=org.example.legacy
Version=1.0.0
SCRIPTMETA-END
"#,
        "org.example.legacy",
    )
    .expect_err("legacy script metadata is not distribution metadata");

    assert!(
        error
            .to_string()
            .contains("missing SCRIPTMETA distribution block")
    );
}

#[test]
fn builds_watch_plan_for_visible_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root_a = temp.path().join("A");
    let root_b = temp.path().join("B");
    std::fs::create_dir_all(&root_a).expect("root a");
    std::fs::create_dir_all(&root_b).expect("root b");

    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");
    engine
        .set_roots(vec![
            scriptmetakit::RootRegistration {
                root_id: "a".to_string(),
                path: root_a,
                display_name: Some("A".to_string()),
                purpose: scriptmetakit::RootPurpose::FileListAndMetadata,
                watch_policy: scriptmetakit::WatchPolicy::VisibleOnly,
                cache_policy: scriptmetakit::CachePolicy::MemoryAndPersistent,
                refresh_policy: scriptmetakit::RefreshPolicy::OnVisible,
                priority: scriptmetakit::RootPriority::VisibleWhenSelected,
            },
            scriptmetakit::RootRegistration {
                root_id: "b".to_string(),
                path: root_b,
                display_name: Some("B".to_string()),
                purpose: scriptmetakit::RootPurpose::FileListAndMetadata,
                watch_policy: scriptmetakit::WatchPolicy::VisibleOnly,
                cache_policy: scriptmetakit::CachePolicy::MemoryAndPersistent,
                refresh_policy: scriptmetakit::RefreshPolicy::OnVisible,
                priority: scriptmetakit::RootPriority::VisibleWhenSelected,
            },
        ])
        .expect("roots");
    engine.set_visible_root(Some("b".to_string()));

    let plan = engine.watch_plan();
    assert_eq!(plan.physical_roots.len(), 1);
    assert_eq!(plan.physical_roots[0].covers_root_ids, vec!["b"]);
}

#[test]
fn scans_metadata_and_file_list() {
    let temp = tempfile::tempdir().expect("tempdir");
    let script_path = temp.path().join("Example.jsx");
    std::fs::write(
        &script_path,
        r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.script
// Version: 2.0.0
// Name: Example Script
// SCRIPTMETA-END
"#,
    )
    .expect("script");
    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");
    engine
        .set_roots(vec![scriptmetakit::RootRegistration {
            root_id: "scripts".to_string(),
            path: temp.path().to_path_buf(),
            display_name: Some("Scripts".to_string()),
            purpose: scriptmetakit::RootPurpose::FileListAndMetadata,
            watch_policy: scriptmetakit::WatchPolicy::AllRegistered,
            cache_policy: scriptmetakit::CachePolicy::MemoryAndPersistent,
            refresh_policy: scriptmetakit::RefreshPolicy::OnFileEvent,
            priority: scriptmetakit::RootPriority::UserInitiated,
        }])
        .expect("roots");

    let result = engine
        .scan_roots(scriptmetakit::ScanRequest::all(
            scriptmetakit::ScanMode::FileListAndMetadata,
        ))
        .expect("scan");

    assert_eq!(result.file_list_snapshots.len(), 1);
    assert_eq!(
        result
            .catalog_snapshot
            .as_ref()
            .expect("catalog")
            .all_items
            .len(),
        1
    );
    assert_eq!(
        result.catalog_snapshot.unwrap().all_items[0]
            .name
            .as_deref(),
        Some("Example Script")
    );
}

#[test]
fn scans_multiple_root_paths_with_root_aware_result() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root_a = temp.path().join("A");
    let root_b = temp.path().join("B");
    std::fs::create_dir_all(&root_a).expect("root a");
    std::fs::create_dir_all(&root_b).expect("root b");
    std::fs::write(
        root_a.join("A.jsx"),
        r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.a
// Version: 1.0.0
// Name: Script A
// SCRIPTMETA-END
"#,
    )
    .expect("script a");
    std::fs::write(
        root_b.join("B.jsx"),
        r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.b
// Version: 1.0.0
// Name: Script B
// SCRIPTMETA-END
"#,
    )
    .expect("script b");

    let root_a_id = scriptmetakit::path_based_root_id(&root_a);
    let root_b_id = scriptmetakit::path_based_root_id(&root_b);
    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");

    let result = engine
        .scan_root_paths(
            vec![root_a.clone(), root_b.clone()],
            scriptmetakit::ScanMode::FileListAndMetadata,
        )
        .expect("scan");

    assert_eq!(engine.roots().len(), 2);
    assert_eq!(result.roots.len(), 2);
    assert_eq!(result.file_list_snapshots.len(), 2);
    assert!(
        result
            .file_list_snapshots
            .iter()
            .any(|snapshot| snapshot.root.root_id == root_a_id)
    );
    assert!(
        result
            .file_list_snapshots
            .iter()
            .any(|snapshot| snapshot.root.root_id == root_b_id)
    );

    let catalog = result.catalog_snapshot.as_ref().expect("catalog");
    assert_eq!(catalog.roots.len(), 2);
    assert_eq!(catalog.all_items.len(), 2);
    let mut item_ids: Vec<_> = catalog
        .all_items
        .iter()
        .map(|item| (item.root_id.as_str(), item.script_id.as_str()))
        .collect();
    item_ids.sort();
    assert_eq!(
        item_ids,
        vec![
            (root_a_id.as_str(), "com.example.a"),
            (root_b_id.as_str(), "com.example.b")
        ]
    );
}

#[test]
fn keeps_file_items_for_overlapping_registered_roots() {
    let temp = tempfile::tempdir().expect("tempdir");
    let parent = temp.path().join("Parent");
    let child = parent.join("JSX");
    std::fs::create_dir_all(&child).expect("child dir");
    std::fs::write(
        child.join("Shared.jsx"),
        r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.overlap
// Version: 1.0.0
// Name: Overlap
// SCRIPTMETA-END
"#,
    )
    .expect("script");

    let parent_id = scriptmetakit::path_based_root_id(&parent);
    let child_id = scriptmetakit::path_based_root_id(&child);
    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("HostApp", "HostApp.ScriptMetaKit"),
    )
    .expect("engine");

    let result = engine
        .scan_root_paths(
            vec![parent.clone(), child.clone()],
            scriptmetakit::ScanMode::FileListAndMetadata,
        )
        .expect("scan");
    let catalog = result.catalog_snapshot.as_ref().expect("catalog");

    assert_eq!(catalog.all_items.len(), 1);
    assert_eq!(catalog.file_items.len(), 2);
    assert_eq!(
        catalog
            .file_items
            .iter()
            .filter(|item| item.root_id == parent_id)
            .count(),
        1
    );
    assert_eq!(
        catalog
            .file_items
            .iter()
            .filter(|item| item.root_id == child_id)
            .count(),
        1
    );
}

#[test]
fn scans_shared_script_extensions_for_any_host_app() {
    let temp = tempfile::tempdir().expect("tempdir");
    let extensions = [
        "js",
        "jsx",
        "jsxbin",
        "jsxinc",
        "scpt",
        "applescript",
        "jxa",
        "idjs",
        "psjs",
    ];
    for extension in extensions {
        std::fs::write(
            temp.path().join(format!("Example.{extension}")),
            format!(
                r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.{extension}
// Version: 1.0.0
// Name: {extension}
// SCRIPTMETA-END
"#
            ),
        )
        .expect("script");
    }

    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("HostApp", "HostApp.ScriptMetaKit"),
    )
    .expect("engine");

    let result = engine
        .scan_root_paths(
            vec![temp.path().to_path_buf()],
            scriptmetakit::ScanMode::FileListAndMetadata,
        )
        .expect("scan");

    let snapshot = result
        .file_list_snapshots
        .first()
        .expect("file list snapshot");
    assert_eq!(
        count_files(snapshot.children.as_deref().unwrap_or_default()),
        extensions.len()
    );

    let catalog = result.catalog_snapshot.as_ref().expect("catalog");
    assert_eq!(catalog.all_items.len(), extensions.len());
}

#[test]
fn excludes_shell_command_and_txt_files_by_default() {
    let temp = tempfile::tempdir().expect("tempdir");
    for extension in ["sh", "command", "txt"] {
        std::fs::write(
            temp.path().join(format!("Example.{extension}")),
            format!(
                r#"
# SCRIPTMETA-BEGIN
# Script-ID: com.example.{extension}
# Version: 1.0.0
# SCRIPTMETA-END
"#
            ),
        )
        .expect("script");
    }

    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("HostApp", "HostApp.ScriptMetaKit"),
    )
    .expect("engine");

    let result = engine
        .scan_root_paths(
            vec![temp.path().to_path_buf()],
            scriptmetakit::ScanMode::FileListAndMetadata,
        )
        .expect("scan");

    let snapshot = result
        .file_list_snapshots
        .first()
        .expect("file list snapshot");
    assert_eq!(
        count_files(snapshot.children.as_deref().unwrap_or_default()),
        0
    );
    assert_eq!(
        result
            .catalog_snapshot
            .as_ref()
            .expect("catalog")
            .all_items
            .len(),
        0
    );
}

#[test]
fn detects_jxa_shebang_for_js_and_applescript_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("Plain.js"),
        r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.plain-js
// Version: 1.0.0
// SCRIPTMETA-END
"#,
    )
    .expect("plain js");
    std::fs::write(
        temp.path().join("Automation.js"),
        r#"#!/usr/bin/osascript -l JavaScript
// SCRIPTMETA-BEGIN
// Script-ID: com.example.jxa-js
// Version: 1.0.0
// SCRIPTMETA-END
"#,
    )
    .expect("jxa js");
    std::fs::write(
        temp.path().join("Automation.applescript"),
        r#"#!/usr/bin/osascript -l JavaScript
// SCRIPTMETA-BEGIN
// Script-ID: com.example.jxa-applescript
// Version: 1.0.0
// SCRIPTMETA-END
"#,
    )
    .expect("jxa applescript");
    std::fs::write(
        temp.path().join("EnvAutomation.js"),
        r#"#!/usr/bin/env osascript -l JavaScript
// SCRIPTMETA-BEGIN
// Script-ID: com.example.env-js
// Version: 1.0.0
// SCRIPTMETA-END
"#,
    )
    .expect("env js");

    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("HostApp", "HostApp.ScriptMetaKit"),
    )
    .expect("engine");

    let result = engine
        .scan_root_paths(
            vec![temp.path().to_path_buf()],
            scriptmetakit::ScanMode::FileListAndMetadata,
        )
        .expect("scan");
    let catalog = result.catalog_snapshot.as_ref().expect("catalog");
    let runtime_by_script_id: std::collections::BTreeMap<_, _> = catalog
        .all_items
        .iter()
        .map(|item| (item.script_id.as_str(), item.runtime_kind))
        .collect();

    assert_eq!(
        runtime_by_script_id.get("com.example.plain-js").copied(),
        Some(Some(scriptmetakit::ScriptRuntimeKind::AdobeJavaScript))
    );
    assert_eq!(
        runtime_by_script_id.get("com.example.jxa-js").copied(),
        Some(Some(
            scriptmetakit::ScriptRuntimeKind::JavaScriptForAutomation
        ))
    );
    assert_eq!(
        runtime_by_script_id
            .get("com.example.jxa-applescript")
            .copied(),
        Some(Some(
            scriptmetakit::ScriptRuntimeKind::JavaScriptForAutomation
        ))
    );
    assert_eq!(
        runtime_by_script_id.get("com.example.env-js").copied(),
        Some(Some(scriptmetakit::ScriptRuntimeKind::AdobeJavaScript)),
        "kit should match Scripta's exact shebang prefix rule"
    );

    let snapshot = result
        .file_list_snapshots
        .first()
        .expect("file list snapshot");
    let entries = snapshot.children.as_deref().unwrap_or_default();
    assert!(
        contains_runtime_kind(
            entries,
            "Automation.js",
            scriptmetakit::ScriptRuntimeKind::JavaScriptForAutomation
        ),
        "file list should expose JXA runtime for shebang-based .js"
    );
}

#[test]
fn reports_scriptmeta_edit_capabilities_for_file_list_and_items() {
    let temp = tempfile::tempdir().expect("tempdir");
    let editable_path = temp.path().join("Editable.jsx");
    let appendable_path = temp.path().join("Appendable.jsx");
    let obfuscated_path = temp.path().join("Obfuscated.jsx");
    let readonly_path = temp.path().join("ReadOnly.jsx");

    std::fs::write(
        &editable_path,
        r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.editable
// Version: 1.0.0
// Edit-Password-SHA256: salt:abcdef
// SCRIPTMETA-END
"#,
    )
    .expect("editable script");
    std::fs::write(&appendable_path, "alert('append');").expect("appendable script");
    std::fs::write(&obfuscated_path, "@JSXBIN@ES@2.0@script").expect("jsxbin script");
    std::fs::write(
        &readonly_path,
        r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.readonly
// Version: 1.0.0
// SCRIPTMETA-END
"#,
    )
    .expect("readonly script");
    let original_permissions = std::fs::metadata(&readonly_path)
        .expect("readonly metadata")
        .permissions();
    let mut permissions = original_permissions.clone();
    permissions.set_readonly(true);
    std::fs::set_permissions(&readonly_path, permissions).expect("set readonly");

    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("HostApp", "HostApp.ScriptMetaKit"),
    )
    .expect("engine");

    let result = engine
        .scan_root_paths(
            vec![temp.path().to_path_buf()],
            scriptmetakit::ScanMode::FileListAndMetadata,
        )
        .expect("scan");

    std::fs::set_permissions(&readonly_path, original_permissions).expect("restore permissions");

    let entries = result.file_list_snapshots[0]
        .children
        .as_deref()
        .unwrap_or_default();
    let editable_entry = find_file_entry(entries, &editable_path).expect("editable entry");
    assert!(editable_entry.has_scriptmeta);
    assert!(editable_entry.has_scriptmeta_edit_password);
    assert!(editable_entry.can_edit_scriptmeta);
    assert!(!editable_entry.can_append_scriptmeta);
    assert_eq!(
        editable_entry.scriptmeta_edit_state,
        scriptmetakit::ScriptMetaEditState::Editable
    );

    let appendable_entry = find_file_entry(entries, &appendable_path).expect("appendable entry");
    assert!(!appendable_entry.has_scriptmeta);
    assert!(!appendable_entry.can_edit_scriptmeta);
    assert!(appendable_entry.can_append_scriptmeta);
    assert_eq!(
        appendable_entry.scriptmeta_edit_state,
        scriptmetakit::ScriptMetaEditState::Appendable
    );

    let obfuscated_entry = find_file_entry(entries, &obfuscated_path).expect("obfuscated entry");
    assert!(!obfuscated_entry.can_edit_scriptmeta);
    assert!(!obfuscated_entry.can_append_scriptmeta);
    assert_eq!(
        obfuscated_entry.scriptmeta_edit_state,
        scriptmetakit::ScriptMetaEditState::Obfuscated
    );

    let readonly_entry = find_file_entry(entries, &readonly_path).expect("readonly entry");
    assert!(readonly_entry.has_scriptmeta);
    assert!(readonly_entry.is_read_only);
    assert!(!readonly_entry.can_edit_scriptmeta);
    assert_eq!(
        readonly_entry.scriptmeta_edit_state,
        scriptmetakit::ScriptMetaEditState::ReadOnly
    );

    let catalog = result.catalog_snapshot.as_ref().expect("catalog");
    let editable_item = catalog
        .file_items
        .iter()
        .find(|item| item.script_id == "com.example.editable")
        .expect("editable item");
    assert!(editable_item.can_edit_scriptmeta);
    assert!(editable_item.has_scriptmeta_edit_password);
}

#[test]
fn omits_directories_without_displayable_script_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let empty_dir = temp.path().join("Empty");
    let asset_dir = temp.path().join("Assets");
    let script_dir = temp.path().join("Scripts");
    std::fs::create_dir_all(&empty_dir).expect("empty dir");
    std::fs::create_dir_all(&asset_dir).expect("asset dir");
    std::fs::create_dir_all(&script_dir).expect("script dir");
    std::fs::write(asset_dir.join("note.md"), "# note").expect("asset");
    std::fs::write(script_dir.join("WithoutMeta.jsx"), "alert('ok');").expect("script");

    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("HostApp", "HostApp.ScriptMetaKit"),
    )
    .expect("engine");

    let result = engine
        .scan_root_paths(
            vec![temp.path().to_path_buf()],
            scriptmetakit::ScanMode::FileListAndMetadata,
        )
        .expect("scan");

    let children = result.file_list_snapshots[0]
        .children
        .as_deref()
        .unwrap_or_default();
    assert!(!contains_directory(children, &empty_dir));
    assert!(!contains_directory(children, &asset_dir));
    assert!(contains_directory(children, &script_dir));
}

#[test]
fn scans_scripts_in_third_level_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let nested_dir = temp.path().join("Brush").join("OpacityFlowExposure");
    std::fs::create_dir_all(&nested_dir).expect("nested dir");
    let script_path = nested_dir.join("OFE_O3.jsx");
    std::fs::write(
        &script_path,
        r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.deep
// Version: 1.0.0
// Name: Deep Script
// SCRIPTMETA-END
"#,
    )
    .expect("script");
    let expected_script_path = script_path.canonicalize().expect("canonical script path");

    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");
    engine
        .set_roots(vec![scriptmetakit::RootRegistration {
            root_id: "scripts".to_string(),
            path: temp.path().to_path_buf(),
            display_name: Some("Scripts".to_string()),
            purpose: scriptmetakit::RootPurpose::FileListAndMetadata,
            watch_policy: scriptmetakit::WatchPolicy::AllRegistered,
            cache_policy: scriptmetakit::CachePolicy::MemoryAndPersistent,
            refresh_policy: scriptmetakit::RefreshPolicy::OnFileEvent,
            priority: scriptmetakit::RootPriority::UserInitiated,
        }])
        .expect("roots");

    let result = engine
        .scan_roots(scriptmetakit::ScanRequest::all(
            scriptmetakit::ScanMode::FileListAndMetadata,
        ))
        .expect("scan");

    let snapshot = result
        .file_list_snapshots
        .first()
        .expect("file list snapshot");
    assert!(
        contains_file(
            snapshot.children.as_deref().unwrap_or_default(),
            &expected_script_path
        ),
        "third-level script should be present in file list"
    );
    assert_eq!(snapshot.root.status, scriptmetakit::RootStatus::Ready);
    assert!(!snapshot.truncated);

    let catalog = result.catalog_snapshot.as_ref().expect("catalog");
    assert_eq!(catalog.all_items.len(), 1);
    assert_eq!(catalog.all_items[0].script_id, "com.example.deep");
}

#[test]
fn scans_equals_style_script_metadata_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let script_path = temp.path().join("HalftoneGenerator.jsx");
    std::fs::write(
        &script_path,
        r#"
SCRIPTMETA-BEGIN
Script-ID=org.iwashi.Halftone_Generator
Version=1.1
Name=Photoshopで疑似AMスクリーン生成
SCRIPTMETA-END
"#,
    )
    .expect("script");

    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");
    engine
        .set_roots(vec![scriptmetakit::RootRegistration {
            root_id: "scripts".to_string(),
            path: temp.path().to_path_buf(),
            display_name: Some("Photoshop".to_string()),
            purpose: scriptmetakit::RootPurpose::FileListAndMetadata,
            watch_policy: scriptmetakit::WatchPolicy::AllRegistered,
            cache_policy: scriptmetakit::CachePolicy::MemoryAndPersistent,
            refresh_policy: scriptmetakit::RefreshPolicy::OnFileEvent,
            priority: scriptmetakit::RootPriority::UserInitiated,
        }])
        .expect("roots");

    let result = engine
        .scan_roots(scriptmetakit::ScanRequest::all(
            scriptmetakit::ScanMode::FileListAndMetadata,
        ))
        .expect("scan");

    let catalog = result.catalog_snapshot.as_ref().expect("catalog");
    assert_eq!(catalog.all_items.len(), 1);
    assert_eq!(
        catalog.all_items[0].script_id,
        "org.iwashi.Halftone_Generator"
    );
    assert_eq!(
        catalog.all_items[0].name.as_deref(),
        Some("Photoshopで疑似AMスクリーン生成")
    );
}

#[test]
fn reports_file_list_changes_after_rescan() {
    let temp = tempfile::tempdir().expect("tempdir");
    let removed_path = temp.path().join("Removed.jsx");
    let modified_path = temp.path().join("Modified.jsx");
    let added_path = temp.path().join("Added.jsx");
    std::fs::write(&removed_path, "alert('removed');").expect("removed");
    std::fs::write(&modified_path, "alert('before');").expect("modified before");

    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");
    engine
        .scan_root_paths(
            vec![temp.path().to_path_buf()],
            scriptmetakit::ScanMode::FileListAndMetadata,
        )
        .expect("initial scan");

    std::fs::remove_file(&removed_path).expect("remove");
    std::fs::write(&modified_path, "alert('after after');").expect("modified after");
    std::fs::write(&added_path, "alert('added');").expect("added");

    let result = engine
        .scan_roots(scriptmetakit::ScanRequest::all(
            scriptmetakit::ScanMode::FileListAndMetadata,
        ))
        .expect("rescan");
    let summary = result.change_summary.expect("change summary");

    assert_eq!(summary.added_count, 1);
    assert_eq!(summary.removed_count, 1);
    assert_eq!(summary.modified_count, 1);

    let changes: std::collections::BTreeMap<_, _> = summary
        .changes
        .iter()
        .map(|change| {
            (
                change
                    .resolved_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or(""),
                change.kind,
            )
        })
        .collect();
    assert_eq!(
        changes.get("Added.jsx").copied(),
        Some(scriptmetakit::FileEntryChangeKind::Added)
    );
    assert_eq!(
        changes.get("Removed.jsx").copied(),
        Some(scriptmetakit::FileEntryChangeKind::Removed)
    );
    assert_eq!(
        changes.get("Modified.jsx").copied(),
        Some(scriptmetakit::FileEntryChangeKind::Modified)
    );
}

#[test]
fn ignores_hidden_and_unsupported_file_events_before_dirtying_roots() {
    let temp = tempfile::tempdir().expect("tempdir");
    let script_path = temp.path().join("Example.jsx");
    std::fs::write(&script_path, "alert('script');").expect("script");

    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");
    engine
        .scan_root_paths(
            vec![temp.path().to_path_buf()],
            scriptmetakit::ScanMode::FileListAndMetadata,
        )
        .expect("initial scan");

    let root_id = scriptmetakit::path_based_root_id(temp.path());
    let hidden_path = temp.path().join(".DS_Store");
    let unsupported_path = temp.path().join("notes.txt");
    std::fs::write(&hidden_path, "hidden").expect("hidden file");
    std::fs::write(&unsupported_path, "notes").expect("unsupported file");

    let events = engine
        .mark_changed_paths(scriptmetakit::RawChangeBatch {
            paths: vec![hidden_path, unsupported_path],
            overflowed: false,
        })
        .expect("mark changed");

    assert!(events.is_empty());
    assert!(!engine.snapshot(&root_id).expect("snapshot").root.is_dirty);

    let refreshed = engine
        .refresh_dirty_roots(scriptmetakit::RefreshRequest {
            mode: scriptmetakit::ScanMode::FileListAndMetadata,
        })
        .expect("refresh dirty roots");
    assert!(refreshed.change_summary.is_none());
}

#[test]
fn removes_empty_directories_after_script_file_disappears() {
    let temp = tempfile::tempdir().expect("tempdir");
    let directory = temp.path().join("Tools");
    std::fs::create_dir_all(&directory).expect("script directory");
    let script_path = directory.join("Tool.jsx");
    std::fs::write(&script_path, "alert('tool');").expect("script");

    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");
    let initial = engine
        .scan_root_paths(
            vec![temp.path().to_path_buf()],
            scriptmetakit::ScanMode::FileListAndMetadata,
        )
        .expect("initial scan");
    let initial_snapshot = initial
        .file_list_snapshots
        .first()
        .expect("initial file list");
    assert!(contains_directory(
        initial_snapshot.children.as_deref().unwrap_or_default(),
        &directory
    ));

    std::fs::remove_file(&script_path).expect("remove script");
    engine
        .mark_changed_paths(scriptmetakit::RawChangeBatch {
            paths: vec![script_path],
            overflowed: false,
        })
        .expect("mark changed");

    let refreshed = engine
        .refresh_dirty_roots(scriptmetakit::RefreshRequest {
            mode: scriptmetakit::ScanMode::FileListAndMetadata,
        })
        .expect("refresh dirty roots");
    let snapshot = refreshed
        .file_list_snapshots
        .first()
        .expect("refreshed file list");
    assert!(!contains_directory(
        snapshot.children.as_deref().unwrap_or_default(),
        &directory
    ));
}

#[test]
fn directory_rename_updates_visible_file_list() {
    let temp = tempfile::tempdir().expect("tempdir");
    let old_directory = temp.path().join("Old");
    let new_directory = temp.path().join("New");
    std::fs::create_dir_all(&old_directory).expect("old directory");
    let old_script_path = old_directory.join("Tool.jsx");
    std::fs::write(&old_script_path, "alert('tool');").expect("script");

    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");
    engine
        .scan_root_paths(
            vec![temp.path().to_path_buf()],
            scriptmetakit::ScanMode::FileListAndMetadata,
        )
        .expect("initial scan");

    std::fs::rename(&old_directory, &new_directory).expect("rename directory");
    let new_script_path = new_directory
        .join("Tool.jsx")
        .canonicalize()
        .expect("canonical new script");
    engine
        .mark_changed_paths(scriptmetakit::RawChangeBatch {
            paths: vec![old_directory, new_directory.clone()],
            overflowed: false,
        })
        .expect("mark changed");

    let refreshed = engine
        .refresh_dirty_roots(scriptmetakit::RefreshRequest {
            mode: scriptmetakit::ScanMode::FileListAndMetadata,
        })
        .expect("refresh dirty roots");
    let snapshot = refreshed
        .file_list_snapshots
        .first()
        .expect("refreshed file list");
    let children = snapshot.children.as_deref().unwrap_or_default();

    assert!(contains_directory(children, &new_directory));
    assert!(contains_file(children, &new_script_path));
    assert!(!children.iter().any(|entry| {
        entry
            .display_path
            .file_name()
            .and_then(|name| name.to_str())
            == Some("Old")
    }));
}

#[cfg(target_os = "macos")]
#[test]
fn scans_macos_alias_directory_with_display_path_preserved() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = tempfile::tempdir().expect("target tempdir");
    let target_script = target.path().join("AliasTarget.jsx");
    std::fs::write(&target_script, "alert('alias target');").expect("target script");
    let alias_path = temp.path().join("LinkedScripts");
    create_macos_alias(target.path(), &alias_path, true);

    let result = scriptmetakit::scanner::scan_file_list_root(
        &"root".to_string(),
        temp.path(),
        &scriptmetakit::ScannerOptions::default(),
        &scriptmetakit::ExtensionPolicy::default(),
    );
    let children = result.children;
    let alias_entry = children
        .iter()
        .find(|entry| entry.display_path == alias_path)
        .expect("alias directory entry");
    assert_eq!(alias_entry.path_kind, scriptmetakit::PathKind::MacosAlias);
    assert_eq!(
        alias_entry.resolution_status,
        scriptmetakit::PathResolutionStatus::Resolved
    );
    assert_eq!(
        alias_entry.resolved_path,
        target.path().canonicalize().expect("target")
    );
    assert!(contains_file(
        &alias_entry.children,
        &target_script.canonicalize().expect("target script")
    ));
    assert!(alias_entry.children.iter().any(|entry| {
        entry.display_path == alias_path.join("AliasTarget.jsx")
            && entry.resolved_path == target_script.canonicalize().expect("target script")
    }));
}

#[cfg(target_os = "macos")]
#[test]
fn scans_macos_alias_script_without_alias_extension() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target_script = temp.path().join("Actual.jsx");
    std::fs::write(&target_script, "alert('actual');").expect("target script");
    let alias_script = temp.path().join("ScriptAlias");
    create_macos_alias(&target_script, &alias_script, false);

    let result = scriptmetakit::scanner::scan_file_list_root(
        &"root".to_string(),
        temp.path(),
        &scriptmetakit::ScannerOptions::default(),
        &scriptmetakit::ExtensionPolicy::default(),
    );
    let alias_entry = result
        .children
        .iter()
        .find(|entry| entry.display_path == alias_script)
        .expect("alias script entry");
    assert!(!alias_entry.is_directory);
    assert_eq!(alias_entry.path_kind, scriptmetakit::PathKind::MacosAlias);
    assert_eq!(
        alias_entry.resolution_status,
        scriptmetakit::PathResolutionStatus::Resolved
    );
    assert_eq!(
        alias_entry.resolved_path,
        target_script.canonicalize().expect("target script")
    );
}

#[cfg(target_os = "macos")]
#[test]
fn skips_macos_alias_script_when_resolution_is_disabled() {
    let root = tempfile::tempdir().expect("root tempdir");
    let target = tempfile::tempdir().expect("target tempdir");
    let target_script = target.path().join("Actual.jsx");
    std::fs::write(
        &target_script,
        r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.alias-disabled
// Version: 1.0.0
// SCRIPTMETA-END
"#,
    )
    .expect("target script");
    let alias_script = root.path().join("ScriptAlias");
    create_macos_alias(&target_script, &alias_script, false);

    let scanner_options = scriptmetakit::ScannerOptions {
        resolve_macos_alias: false,
        ..Default::default()
    };
    let file_list = scriptmetakit::scanner::scan_file_list_root(
        &"root".to_string(),
        root.path(),
        &scanner_options,
        &scriptmetakit::ExtensionPolicy::default(),
    );
    assert!(
        file_list
            .children
            .iter()
            .all(|entry| entry.display_path != alias_script)
    );

    let mut config = scriptmetakit::ScriptMetaKitConfig::new("Test", "Test");
    config.scanner.resolve_macos_alias = false;
    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(config).expect("engine");
    let result = engine
        .scan_root_paths(
            vec![root.path().to_path_buf()],
            scriptmetakit::ScanMode::FileListAndMetadata,
        )
        .expect("scan");
    assert_eq!(
        result
            .catalog_snapshot
            .as_ref()
            .expect("catalog")
            .all_items
            .len(),
        0
    );
}

#[cfg(target_os = "macos")]
#[test]
fn reports_macos_alias_directory_cycle_without_descending() {
    let temp = tempfile::tempdir().expect("tempdir");
    let alias_path = temp.path().join("Loop");
    create_macos_alias(temp.path(), &alias_path, true);

    let result = scriptmetakit::scanner::scan_file_list_root(
        &"root".to_string(),
        temp.path(),
        &scriptmetakit::ScannerOptions::default(),
        &scriptmetakit::ExtensionPolicy::default(),
    );
    let alias_entry = result
        .children
        .iter()
        .find(|entry| entry.display_path == alias_path)
        .expect("cycle alias entry");
    assert!(alias_entry.is_directory);
    assert_eq!(alias_entry.path_kind, scriptmetakit::PathKind::MacosAlias);
    assert_eq!(
        alias_entry.resolution_status,
        scriptmetakit::PathResolutionStatus::Cycle
    );
    assert!(alias_entry.children.is_empty());
}

#[test]
fn dirty_refresh_returns_complete_registered_state() {
    let first = tempfile::tempdir().expect("first tempdir");
    let second = tempfile::tempdir().expect("second tempdir");
    let first_script = first.path().join("First.jsx");
    let second_script = second.path().join("Second.jsx");
    std::fs::write(
        &first_script,
        r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.first
// Version: 1.0.0
// SCRIPTMETA-END
alert('first');
"#,
    )
    .expect("first script");
    std::fs::write(
        &second_script,
        r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.second
// Version: 1.0.0
// SCRIPTMETA-END
alert('second');
"#,
    )
    .expect("second script");

    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");
    engine
        .scan_root_paths(
            vec![first.path().to_path_buf(), second.path().to_path_buf()],
            scriptmetakit::ScanMode::FileListAndMetadata,
        )
        .expect("initial scan");

    std::fs::write(
        &first_script,
        r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.first
// Version: 1.0.1
// SCRIPTMETA-END
alert('first updated');
"#,
    )
    .expect("first script updated");

    engine
        .mark_changed_paths(scriptmetakit::RawChangeBatch {
            paths: vec![first_script.clone()],
            overflowed: false,
        })
        .expect("mark changed");
    let result = engine
        .refresh_dirty_roots(scriptmetakit::RefreshRequest {
            mode: scriptmetakit::ScanMode::FileListAndMetadata,
        })
        .expect("refresh dirty roots");

    assert_eq!(result.roots.len(), 2);
    assert_eq!(result.file_list_snapshots.len(), 2);
    let catalog = result.catalog_snapshot.as_ref().expect("catalog");
    let script_ids: std::collections::BTreeSet<_> = catalog
        .all_items
        .iter()
        .map(|item| item.script_id.as_str())
        .collect();
    assert!(script_ids.contains("com.example.first"));
    assert!(script_ids.contains("com.example.second"));
    assert_eq!(
        catalog
            .all_items
            .iter()
            .find(|item| item.script_id == "com.example.first")
            .and_then(|item| item.version.as_deref()),
        Some("1.0.1")
    );
}

#[test]
fn dirty_refresh_preserves_update_results_for_unchanged_items_only() {
    let first = tempfile::tempdir().expect("first tempdir");
    let second = tempfile::tempdir().expect("second tempdir");
    let dist = tempfile::tempdir().expect("dist tempdir");
    let first_dist = dist.path().join("first.txt");
    let second_dist = dist.path().join("second.txt");
    let first_dist_url = url::Url::from_file_path(&first_dist).expect("first dist url");
    let second_dist_url = url::Url::from_file_path(&second_dist).expect("second dist url");
    std::fs::write(
        &first_dist,
        r#"
SCRIPTMETA-DIST-BEGIN
Script-ID: com.example.first
Latest-Version: 2.0.0
SCRIPTMETA-DIST-END
"#,
    )
    .expect("first dist");
    std::fs::write(
        &second_dist,
        r#"
SCRIPTMETA-DIST-BEGIN
Script-ID: com.example.second
Latest-Version: 2.0.0
SCRIPTMETA-DIST-END
"#,
    )
    .expect("second dist");

    let first_script = first.path().join("First.jsx");
    let second_script = second.path().join("Second.jsx");
    std::fs::write(
        &first_script,
        format!(
            r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.first
// Version: 1.0.0
// Meta-URL: {first_dist_url}
// SCRIPTMETA-END
alert('first');
"#
        ),
    )
    .expect("first script");
    std::fs::write(
        &second_script,
        format!(
            r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.second
// Version: 1.0.0
// Meta-URL: {second_dist_url}
// SCRIPTMETA-END
alert('second');
"#
        ),
    )
    .expect("second script");

    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");
    let scan_result = engine
        .scan_root_paths(
            vec![first.path().to_path_buf(), second.path().to_path_buf()],
            scriptmetakit::ScanMode::FileListAndMetadata,
        )
        .expect("initial scan");
    let items = scan_result
        .catalog_snapshot
        .as_ref()
        .expect("catalog")
        .all_items
        .clone();
    let first_item_id = first_script.to_string_lossy().into_owned();
    let second_item_id = second_script.to_string_lossy().into_owned();
    pollster::block_on(engine.check_updates(scriptmetakit::UpdateCheckRequest { items }))
        .expect("update check");

    std::fs::write(
        &first_script,
        format!(
            r#"
// SCRIPTMETA-BEGIN
// Script-ID: com.example.first
// Version: 1.1.0
// Meta-URL: {first_dist_url}
// SCRIPTMETA-END
alert('first updated');
"#
        ),
    )
    .expect("first script updated");
    engine
        .mark_changed_paths(scriptmetakit::RawChangeBatch {
            paths: vec![first_script],
            overflowed: false,
        })
        .expect("mark changed");
    let result = engine
        .refresh_dirty_roots(scriptmetakit::RefreshRequest {
            mode: scriptmetakit::ScanMode::FileListAndMetadata,
        })
        .expect("refresh dirty roots");
    let update_result = result
        .catalog_snapshot
        .as_ref()
        .and_then(|catalog| catalog.update_check_result.as_ref())
        .expect("preserved update result");

    assert!(
        !update_result
            .statuses_by_item_id
            .contains_key(&first_item_id)
    );
    assert_eq!(
        update_result
            .statuses_by_item_id
            .get(&second_item_id)
            .copied(),
        Some(scriptmetakit::UpdateStatus::UpdateAvailable)
    );
}

fn contains_file(entries: &[scriptmetakit::FileSystemEntry], path: &std::path::Path) -> bool {
    entries.iter().any(|entry| {
        (!entry.is_directory && entry.resolved_path == path) || contains_file(&entry.children, path)
    })
}

fn find_file_entry<'a>(
    entries: &'a [scriptmetakit::FileSystemEntry],
    path: &std::path::Path,
) -> Option<&'a scriptmetakit::FileSystemEntry> {
    let resolved_path = path.canonicalize().expect("canonical file path");
    entries.iter().find_map(|entry| {
        if !entry.is_directory && entry.resolved_path == resolved_path {
            Some(entry)
        } else {
            find_file_entry(&entry.children, path)
        }
    })
}

fn contains_directory(entries: &[scriptmetakit::FileSystemEntry], path: &std::path::Path) -> bool {
    let resolved_path = path.canonicalize().expect("canonical directory path");
    entries.iter().any(|entry| {
        (entry.is_directory && entry.resolved_path == resolved_path)
            || contains_directory(&entry.children, path)
    })
}

fn contains_runtime_kind(
    entries: &[scriptmetakit::FileSystemEntry],
    file_name: &str,
    runtime_kind: scriptmetakit::ScriptRuntimeKind,
) -> bool {
    entries.iter().any(|entry| {
        (!entry.is_directory
            && entry
                .display_path
                .file_name()
                .and_then(|name| name.to_str())
                == Some(file_name)
            && entry.runtime_kind == Some(runtime_kind))
            || contains_runtime_kind(&entry.children, file_name, runtime_kind)
    })
}

fn count_files(entries: &[scriptmetakit::FileSystemEntry]) -> usize {
    entries
        .iter()
        .map(|entry| {
            if entry.is_directory {
                count_files(&entry.children)
            } else {
                1
            }
        })
        .sum()
}

#[cfg(target_os = "macos")]
fn create_macos_alias(target: &std::path::Path, alias_path: &std::path::Path, is_directory: bool) {
    use objc2_foundation::{NSURL, NSURLBookmarkCreationOptions};

    let target_url = if is_directory {
        NSURL::from_directory_path(target)
    } else {
        NSURL::from_file_path(target)
    }
    .expect("target NSURL");
    let alias_url = NSURL::from_file_path(alias_path).expect("alias NSURL");
    let bookmark_data = target_url
        .bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(
            NSURLBookmarkCreationOptions::SuitableForBookmarkFile,
            None,
            None,
        )
        .expect("bookmark data");
    NSURL::writeBookmarkData_toURL_options_error(&bookmark_data, &alias_url, 0)
        .expect("write alias");
}

#[test]
fn saves_and_loads_cache_payload() {
    let temp = tempfile::tempdir().expect("tempdir");
    let cache_path = temp.path().join("cache").join("catalog.json");
    let payload = scriptmetakit::CachePayload::new(
        scriptmetakit::CacheScope::Catalog,
        serde_json::json!({
            "roots": [],
            "all_items": []
        }),
    );

    scriptmetakit::save_cache_payload(&cache_path, &payload).expect("save cache");
    let loaded = scriptmetakit::load_cache_payload(&cache_path).expect("load cache");

    assert_eq!(loaded.scope, scriptmetakit::CacheScope::Catalog);
    assert_eq!(loaded.schema.schema_version, payload.schema.schema_version);
    assert_eq!(loaded.data, payload.data);
}

#[test]
fn checks_file_url_update_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");
    let dist_path = temp.path().join("SCRIPTMETA.txt");
    let dist_url = url::Url::from_file_path(&dist_path).expect("file url");
    std::fs::write(
        &dist_path,
        r#"
SCRIPTMETA-DIST-BEGIN
Script-ID: com.example.script
Latest-Version: 3.0.0
SCRIPTMETA-DIST-END
"#,
    )
    .expect("dist");

    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");
    let result = pollster::block_on(engine.check_updates(scriptmetakit::UpdateCheckRequest {
        items: vec![scriptmetakit::ScriptMetaItem {
            root_id: "scripts".to_string(),
            file_path: temp.path().join("Example.jsx"),
            identity_path: temp.path().join("Example.jsx"),
            runtime_kind: None,
            shebang: None,
            script_id: "com.example.script".to_string(),
            version: Some("2.0.0".to_string()),
            description: None,
            target_app: None,
            meta_url: Some(dist_url),
            name: Some("Example".to_string()),
            author: None,
            release_date: None,
            edit_password_sha256: None,
            has_scriptmeta: true,
            has_scriptmeta_edit_password: false,
            is_file_locked: false,
            is_read_only: false,
            can_edit_scriptmeta: false,
            can_append_scriptmeta: false,
            scriptmeta_edit_state: scriptmetakit::ScriptMetaEditState::Unknown,
        }],
    }))
    .expect("update check");

    assert_eq!(
        result.statuses_by_item_id.values().next().copied(),
        Some(scriptmetakit::UpdateStatus::UpdateAvailable)
    );
}

#[test]
fn checks_file_url_update_metadata_after_large_prefix() {
    let temp = tempfile::tempdir().expect("tempdir");
    let dist_path = temp.path().join("large-page.txt");
    let dist_url = url::Url::from_file_path(&dist_path).expect("file url");
    let mut source = String::with_capacity(300_000);
    source.extend(std::iter::repeat_n('x', 280_000));
    source.push_str(
        r#"
SCRIPTMETA-DIST-BEGIN
Script-ID: com.example.large.page
Latest-Version: 7.0.0
SCRIPTMETA-DIST-END
"#,
    );
    std::fs::write(&dist_path, source).expect("dist");

    let script_path = temp.path().join("Large.jsx");
    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");
    let result = pollster::block_on(engine.check_updates(scriptmetakit::UpdateCheckRequest {
        items: vec![scriptmetakit::ScriptMetaItem {
            root_id: "scripts".to_string(),
            file_path: script_path.clone(),
            identity_path: script_path,
            runtime_kind: None,
            shebang: None,
            script_id: "com.example.large.page".to_string(),
            version: Some("6.0.0".to_string()),
            description: None,
            target_app: None,
            meta_url: Some(dist_url),
            name: Some("Large".to_string()),
            author: None,
            release_date: None,
            edit_password_sha256: None,
            has_scriptmeta: true,
            has_scriptmeta_edit_password: false,
            is_file_locked: false,
            is_read_only: false,
            can_edit_scriptmeta: false,
            can_append_scriptmeta: false,
            scriptmeta_edit_state: scriptmetakit::ScriptMetaEditState::Unknown,
        }],
    }))
    .expect("update check");

    assert_eq!(
        result.statuses_by_item_id.values().next().copied(),
        Some(scriptmetakit::UpdateStatus::UpdateAvailable)
    );
}

#[test]
fn returns_structured_update_failure_data() {
    let temp = tempfile::tempdir().expect("tempdir");
    let dist_path = temp.path().join("SCRIPTMETA.txt");
    let dist_url = url::Url::from_file_path(&dist_path).expect("file url");
    std::fs::write(
        &dist_path,
        r#"
SCRIPTMETA-DIST-BEGIN
Script-ID: com.example.script
SCRIPTMETA-DIST-END
"#,
    )
    .expect("dist");

    let script_path = temp.path().join("Example.jsx");
    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");
    let result = pollster::block_on(engine.check_updates(scriptmetakit::UpdateCheckRequest {
        items: vec![scriptmetakit::ScriptMetaItem {
            root_id: "scripts".to_string(),
            file_path: script_path.clone(),
            identity_path: script_path.clone(),
            runtime_kind: None,
            shebang: None,
            script_id: "com.example.script".to_string(),
            version: Some("2.0.0".to_string()),
            description: None,
            target_app: None,
            meta_url: Some(dist_url.clone()),
            name: Some("Example".to_string()),
            author: None,
            release_date: None,
            edit_password_sha256: None,
            has_scriptmeta: true,
            has_scriptmeta_edit_password: false,
            is_file_locked: false,
            is_read_only: false,
            can_edit_scriptmeta: false,
            can_append_scriptmeta: false,
            scriptmeta_edit_state: scriptmetakit::ScriptMetaEditState::Unknown,
        }],
    }))
    .expect("update check");

    let item_id = script_path.to_string_lossy().into_owned();
    let failure = result
        .failures_by_item_id
        .get(&item_id)
        .expect("failure data");
    assert_eq!(failure.code, "unresolved_distribution");
    assert_eq!(failure.message, "Latest-Version was not found");
    assert_eq!(failure.script_id, "com.example.script");
    assert_eq!(failure.current_version.as_deref(), Some("2.0.0"));
    assert_eq!(failure.meta_url.as_ref(), Some(&dist_url));
    assert_eq!(failure.source_url.as_ref(), Some(&dist_url));
    assert_eq!(
        result.statuses_by_item_id.get(&item_id).copied(),
        Some(scriptmetakit::UpdateStatus::Failed)
    );
    assert_eq!(
        result.errors_by_item_id.get(&item_id).map(String::as_str),
        Some("Latest-Version was not found")
    );
}

#[test]
fn checks_matching_entry_in_multi_script_distribution_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");
    let dist_path = temp.path().join("SCRIPTMETA.txt");
    let dist_url = url::Url::from_file_path(&dist_path).expect("file url");
    std::fs::write(
        &dist_path,
        r#"
SCRIPTMETA-DIST-BEGIN
Script-ID=org.example.first
Version=1.0.0
Script-ID=org.example.second
Version=2.5.0
SCRIPTMETA-DIST-END
"#,
    )
    .expect("dist");

    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");
    let result = pollster::block_on(engine.check_updates(scriptmetakit::UpdateCheckRequest {
        items: vec![scriptmetakit::ScriptMetaItem {
            root_id: "scripts".to_string(),
            file_path: temp.path().join("Second.jsx"),
            identity_path: temp.path().join("Second.jsx"),
            runtime_kind: None,
            shebang: None,
            script_id: "org.example.second".to_string(),
            version: Some("2.0.0".to_string()),
            description: None,
            target_app: None,
            meta_url: Some(dist_url),
            name: Some("Second".to_string()),
            author: None,
            release_date: None,
            edit_password_sha256: None,
            has_scriptmeta: true,
            has_scriptmeta_edit_password: false,
            is_file_locked: false,
            is_read_only: false,
            can_edit_scriptmeta: false,
            can_append_scriptmeta: false,
            scriptmeta_edit_state: scriptmetakit::ScriptMetaEditState::Unknown,
        }],
    }))
    .expect("update check");

    let resolution = result
        .resolutions_by_item_id
        .values()
        .next()
        .expect("resolution");
    assert_eq!(resolution.latest_version.as_deref(), Some("2.5.0"));
    assert_eq!(
        result.statuses_by_item_id.values().next().copied(),
        Some(scriptmetakit::UpdateStatus::UpdateAvailable)
    );
}

#[test]
fn follows_latest_url_chain_for_file_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");
    let first_path = temp.path().join("first.txt");
    let second_path = temp.path().join("second.txt");
    let second_url = url::Url::from_file_path(&second_path).expect("second file url");
    std::fs::write(
        &first_path,
        format!(
            r#"
SCRIPTMETA-DIST-BEGIN
Script-ID: com.example.script
Latest-URL: {second_url}
SCRIPTMETA-DIST-END
"#
        ),
    )
    .expect("first");
    std::fs::write(
        &second_path,
        r#"
SCRIPTMETA-DIST-BEGIN
Script-ID: com.example.script
Latest-Version: 4.0.0
SCRIPTMETA-DIST-END
"#,
    )
    .expect("second");

    let first_url = url::Url::from_file_path(&first_path).expect("first file url");
    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");
    let result = pollster::block_on(engine.check_updates(scriptmetakit::UpdateCheckRequest {
        items: vec![scriptmetakit::ScriptMetaItem {
            root_id: "scripts".to_string(),
            file_path: temp.path().join("Example.jsx"),
            identity_path: temp.path().join("Example.jsx"),
            runtime_kind: None,
            shebang: None,
            script_id: "com.example.script".to_string(),
            version: Some("3.0.0".to_string()),
            description: None,
            target_app: None,
            meta_url: Some(first_url),
            name: Some("Example".to_string()),
            author: None,
            release_date: None,
            edit_password_sha256: None,
            has_scriptmeta: true,
            has_scriptmeta_edit_password: false,
            is_file_locked: false,
            is_read_only: false,
            can_edit_scriptmeta: false,
            can_append_scriptmeta: false,
            scriptmeta_edit_state: scriptmetakit::ScriptMetaEditState::Unknown,
        }],
    }))
    .expect("update check");

    let resolution = result
        .resolutions_by_item_id
        .values()
        .next()
        .expect("resolution");
    assert_eq!(resolution.latest_version.as_deref(), Some("4.0.0"));
    assert_eq!(resolution.redirect_count, Some(1));
    assert_eq!(resolution.latest_url_history, vec![second_url]);
}

#[cfg(feature = "blocking-http")]
#[test]
fn checks_http_update_metadata() {
    let response_body = r#"
SCRIPTMETA-DIST-BEGIN
Script-ID: com.example.http
Latest-Version: 5.0.0
SCRIPTMETA-DIST-END
"#;
    let url = spawn_http_once(response_body);
    let temp = tempfile::tempdir().expect("tempdir");
    let mut engine = scriptmetakit::ScriptMetaKitEngine::new(
        scriptmetakit::ScriptMetaKitConfig::new("Test", "Test"),
    )
    .expect("engine");
    let result = pollster::block_on(engine.check_updates(scriptmetakit::UpdateCheckRequest {
        items: vec![scriptmetakit::ScriptMetaItem {
            root_id: "scripts".to_string(),
            file_path: temp.path().join("Http.jsx"),
            identity_path: temp.path().join("Http.jsx"),
            runtime_kind: None,
            shebang: None,
            script_id: "com.example.http".to_string(),
            version: Some("4.0.0".to_string()),
            description: None,
            target_app: None,
            meta_url: Some(url),
            name: Some("HTTP".to_string()),
            author: None,
            release_date: None,
            edit_password_sha256: None,
            has_scriptmeta: true,
            has_scriptmeta_edit_password: false,
            is_file_locked: false,
            is_read_only: false,
            can_edit_scriptmeta: false,
            can_append_scriptmeta: false,
            scriptmeta_edit_state: scriptmetakit::ScriptMetaEditState::Unknown,
        }],
    }))
    .expect("update check");

    assert_eq!(
        result.statuses_by_item_id.values().next().copied(),
        Some(scriptmetakit::UpdateStatus::UpdateAvailable)
    );
}

#[cfg(feature = "blocking-http")]
fn spawn_http_once(body: &'static str) -> url::Url {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let address = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });
    url::Url::parse(&format!("http://{address}/SCRIPTMETA.txt")).expect("server url")
}
