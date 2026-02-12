use std::time::{SystemTime, UNIX_EPOCH};

use swiftfind_core::core_service::CoreService;
#[cfg(not(target_os = "windows"))]
use swiftfind_core::discovery::StartMenuAppDiscoveryProvider;
use swiftfind_core::discovery::{
    AppProvider, DiscoveryProvider, FileProvider, FileSystemDiscoveryProvider,
};

#[test]
fn app_provider_fixture_is_deterministic() {
    let provider = AppProvider::deterministic_fixture();
    let items = provider.discover().unwrap();

    assert_eq!(provider.provider_name(), "app");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, "app-code");
    assert_eq!(items[1].id, "app-term");
}

#[test]
fn file_provider_fixture_is_deterministic() {
    let provider = FileProvider::deterministic_fixture();
    let items = provider.discover().unwrap();

    assert_eq!(provider.provider_name(), "file");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, "file-report");
    assert_eq!(items[1].id, "file-notes");
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_menu_provider_returns_empty_off_windows() {
    let provider = StartMenuAppDiscoveryProvider::default();
    let items = provider.discover().unwrap();
    assert!(items.is_empty());
}

#[test]
fn file_system_provider_discovers_files_in_roots() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let root = std::env::temp_dir().join(format!("swiftfind-discovery-{unique}"));
    let nested = root.join("nested");
    std::fs::create_dir_all(&nested).unwrap();

    let report = nested.join("Q4_Report.xlsx");
    let notes = root.join("Notes.txt");
    std::fs::write(&report, b"report").unwrap();
    std::fs::write(&notes, b"notes").unwrap();

    let provider = FileSystemDiscoveryProvider::new(vec![root.clone()], 4);
    let items = provider.discover().unwrap();

    let titles: Vec<String> = items.iter().map(|i| i.title.clone()).collect();
    let kinds: Vec<String> = items.iter().map(|i| i.kind.clone()).collect();
    assert!(titles.contains(&"nested".to_string()));
    assert!(titles.contains(&"Q4_Report.xlsx".to_string()));
    assert!(titles.contains(&"Notes.txt".to_string()));
    assert!(kinds.contains(&"folder".to_string()));

    std::fs::remove_file(&report).unwrap();
    std::fs::remove_file(&notes).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn rebuild_index_uses_registered_providers() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let app_path = std::env::temp_dir().join(format!("swiftfind-app-provider-{unique}.tmp"));
    let file_path = std::env::temp_dir().join(format!("swiftfind-file-provider-{unique}.tmp"));
    std::fs::write(&app_path, b"app").unwrap();
    std::fs::write(&file_path, b"file").unwrap();

    let config = swiftfind_core::config::Config::default();
    let db = swiftfind_core::index_store::open_memory().unwrap();

    let service = CoreService::with_connection(config, db)
        .unwrap()
        .with_providers(vec![
            Box::new(AppProvider::from_apps(vec![
                swiftfind_core::model::SearchItem::new(
                    "app-code",
                    "app",
                    "Visual Studio Code",
                    app_path.to_string_lossy().as_ref(),
                ),
            ])),
            Box::new(FileProvider::from_files(vec![
                swiftfind_core::model::SearchItem::new(
                    "file-report",
                    "file",
                    "Q4_Report.xlsx",
                    file_path.to_string_lossy().as_ref(),
                ),
            ])),
        ]);

    let inserted = service.rebuild_index().unwrap();
    assert_eq!(inserted, 2);

    let results = service.search("report", 10).unwrap();
    assert_eq!(results[0].id, "file-report");

    std::fs::remove_file(app_path).unwrap();
    std::fs::remove_file(file_path).unwrap();
}

#[test]
fn runtime_providers_use_configured_roots() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("swiftfind-runtime-roots-{unique}"));
    std::fs::create_dir_all(&root).unwrap();
    let file_path = root.join("RuntimeDoc.txt");
    std::fs::write(&file_path, b"runtime").unwrap();

    let mut config = swiftfind_core::config::Config::default();
    config.discovery_roots = vec![root.clone()];

    let db = swiftfind_core::index_store::open_memory().unwrap();
    let service = CoreService::with_connection(config, db)
        .unwrap()
        .with_runtime_providers();

    let inserted = service.rebuild_index().unwrap();
    assert!(inserted >= 1);

    let results = service.search("runtimedoc", 10).unwrap();
    assert!(!results.is_empty());

    std::fs::remove_file(&file_path).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}
