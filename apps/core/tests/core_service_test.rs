use std::time::{SystemTime, UNIX_EPOCH};

use swiftfind_core::core_service::{CoreService, LaunchTarget, ServiceError};

fn test_config() -> swiftfind_core::config::Config {
    swiftfind_core::config::Config::default()
}

#[test]
fn service_search_returns_ranked_results() {
    let config = test_config();
    let db = swiftfind_core::index_store::open_memory().unwrap();
    let service = CoreService::with_connection(config, db).unwrap();

    service
        .upsert_item(&swiftfind_core::model::SearchItem::new(
            "1",
            "app",
            "Code",
            "C:\\Code.exe",
        ))
        .unwrap();
    service
        .upsert_item(&swiftfind_core::model::SearchItem::new(
            "2",
            "app",
            "Codeium",
            "C:\\Codeium.exe",
        ))
        .unwrap();

    let results = service.search("code", 10).unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, "1");
}

#[test]
fn service_launch_by_id_uses_indexed_path() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let launch_path = std::env::temp_dir().join(format!("swiftfind-service-launch-{unique}.tmp"));
    std::fs::write(&launch_path, b"ok").unwrap();

    let config = test_config();
    let db = swiftfind_core::index_store::open_memory().unwrap();
    let service = CoreService::with_connection(config, db).unwrap();

    service
        .upsert_item(&swiftfind_core::model::SearchItem::new(
            "launch-id",
            "file",
            "Launch Target",
            launch_path.to_str().unwrap(),
        ))
        .unwrap();

    let launched = service.launch(LaunchTarget::Id("launch-id"));
    std::fs::remove_file(&launch_path).unwrap();

    assert!(launched.is_ok());
}

#[test]
fn service_launch_by_missing_id_returns_typed_error() {
    let config = test_config();
    let db = swiftfind_core::index_store::open_memory().unwrap();
    let service = CoreService::with_connection(config, db).unwrap();

    let result = service.launch(LaunchTarget::Id("missing"));

    match result {
        Err(ServiceError::ItemNotFound(id)) => assert_eq!(id, "missing"),
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn service_rebuild_index_reports_item_count() {
    let config = test_config();
    let db = swiftfind_core::index_store::open_memory().unwrap();
    let service = CoreService::with_connection(config, db).unwrap();

    service
        .upsert_item(&swiftfind_core::model::SearchItem::new("1", "file", "A", "C:\\A.txt"))
        .unwrap();
    service
        .upsert_item(&swiftfind_core::model::SearchItem::new("2", "file", "B", "C:\\B.txt"))
        .unwrap();

    let rebuilt_count = service.rebuild_index().unwrap();
    assert_eq!(rebuilt_count, 2);
}
