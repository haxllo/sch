use std::time::{SystemTime, UNIX_EPOCH};

use swiftfind_core::core_service::{CoreService, LaunchTarget, ServiceError};
use swiftfind_core::discovery::{DiscoveryProvider, ProviderError};
use swiftfind_core::model::SearchItem;
use std::sync::{Arc, Mutex};

fn test_config() -> swiftfind_core::config::Config {
    swiftfind_core::config::Config::default()
}

#[test]
fn service_search_returns_ranked_results() {
    let config = test_config();
    let db = swiftfind_core::index_store::open_memory().unwrap();
    let service = CoreService::with_connection(config, db).unwrap();

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p1 = std::env::temp_dir().join(format!("swiftfind-search-ranked-{unique}-1.tmp"));
    let p2 = std::env::temp_dir().join(format!("swiftfind-search-ranked-{unique}-2.tmp"));
    std::fs::write(&p1, b"1").unwrap();
    std::fs::write(&p2, b"2").unwrap();

    service
        .upsert_item(&swiftfind_core::model::SearchItem::new(
            "1",
            "app",
            "Code",
            p1.to_string_lossy().as_ref(),
        ))
        .unwrap();
    service
        .upsert_item(&swiftfind_core::model::SearchItem::new(
            "2",
            "app",
            "Codeium",
            p2.to_string_lossy().as_ref(),
        ))
        .unwrap();

    let results = service.search("code", 10).unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, "1");

    std::fs::remove_file(p1).unwrap();
    std::fs::remove_file(p2).unwrap();
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
        .upsert_item(&swiftfind_core::model::SearchItem::new(
            "1",
            "file",
            "A",
            "C:\\A.txt",
        ))
        .unwrap();
    service
        .upsert_item(&swiftfind_core::model::SearchItem::new(
            "2",
            "file",
            "B",
            "C:\\B.txt",
        ))
        .unwrap();

    let rebuilt_count = service.rebuild_index().unwrap();
    assert_eq!(rebuilt_count, 2);
}

#[test]
fn service_search_prunes_stale_items() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let present_path = std::env::temp_dir().join(format!("swiftfind-present-{unique}.tmp"));
    let missing_path = std::env::temp_dir().join(format!("swiftfind-missing-{unique}.tmp"));
    std::fs::write(&present_path, b"ok").unwrap();

    let config = test_config();
    let db = swiftfind_core::index_store::open_memory().unwrap();
    let service = CoreService::with_connection(config, db).unwrap();

    service
        .upsert_item(&swiftfind_core::model::SearchItem::new(
            "present",
            "file",
            "Present",
            present_path.to_string_lossy().as_ref(),
        ))
        .unwrap();
    service
        .upsert_item(&swiftfind_core::model::SearchItem::new(
            "stale",
            "file",
            "Stale",
            missing_path.to_string_lossy().as_ref(),
        ))
        .unwrap();

    let _ = service.search("present", 10).unwrap();
    let stale_launch = service.launch(LaunchTarget::Id("stale"));
    match stale_launch {
        Err(ServiceError::ItemNotFound(id)) => assert_eq!(id, "stale"),
        other => panic!("unexpected result: {other:?}"),
    }

    std::fs::remove_file(present_path).unwrap();
}

#[test]
fn service_launch_missing_path_prunes_item() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let missing_path = std::env::temp_dir().join(format!("swiftfind-launch-missing-{unique}.tmp"));

    let config = test_config();
    let db = swiftfind_core::index_store::open_memory().unwrap();
    let service = CoreService::with_connection(config, db).unwrap();

    service
        .upsert_item(&swiftfind_core::model::SearchItem::new(
            "stale-launch",
            "file",
            "Stale Launch",
            missing_path.to_string_lossy().as_ref(),
        ))
        .unwrap();

    let first = service.launch(LaunchTarget::Id("stale-launch"));
    match first {
        Err(ServiceError::Launch(swiftfind_core::action_executor::LaunchError::MissingPath(_))) => {
        }
        other => panic!("unexpected first launch result: {other:?}"),
    }

    let second = service.launch(LaunchTarget::Id("stale-launch"));
    match second {
        Err(ServiceError::ItemNotFound(id)) => assert_eq!(id, "stale-launch"),
        other => panic!("unexpected second launch result: {other:?}"),
    }
}

struct MutableProvider {
    name: &'static str,
    items: Arc<Mutex<Vec<SearchItem>>>,
}

impl MutableProvider {
    fn new(name: &'static str, items: Arc<Mutex<Vec<SearchItem>>>) -> Self {
        Self { name, items }
    }
}

impl DiscoveryProvider for MutableProvider {
    fn provider_name(&self) -> &'static str {
        self.name
    }

    fn discover(&self) -> Result<Vec<SearchItem>, ProviderError> {
        Ok(self.items.lock().expect("provider lock poisoned").clone())
    }
}

#[test]
fn incremental_rebuild_prunes_missing_provider_items() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let initial_path = std::env::temp_dir().join(format!("swiftfind-inc-initial-{unique}.tmp"));
    let stable_path = std::env::temp_dir().join(format!("swiftfind-inc-stable-{unique}.tmp"));
    let replacement_path = std::env::temp_dir().join(format!("swiftfind-inc-next-{unique}.tmp"));
    std::fs::write(&initial_path, b"a").unwrap();
    std::fs::write(&stable_path, b"b").unwrap();
    std::fs::write(&replacement_path, b"c").unwrap();

    let provider_items = Arc::new(Mutex::new(vec![
        SearchItem::new(
            "file:initial",
            "file",
            "Initial",
            initial_path.to_string_lossy().as_ref(),
        ),
        SearchItem::new(
            "file:stable",
            "file",
            "Stable",
            stable_path.to_string_lossy().as_ref(),
        ),
    ]));

    let config = test_config();
    let db = swiftfind_core::index_store::open_memory().unwrap();
    let service = CoreService::with_connection(config, db)
        .unwrap()
        .with_providers(vec![Box::new(MutableProvider::new(
            "filesystem",
            provider_items.clone(),
        ))]);

    let first = service.rebuild_index_with_report().unwrap();
    assert_eq!(first.indexed_total, 2);
    assert_eq!(first.removed_total, 0);

    *provider_items.lock().unwrap() = vec![
        SearchItem::new(
            "file:stable",
            "file",
            "Stable",
            stable_path.to_string_lossy().as_ref(),
        ),
        SearchItem::new(
            "file:next",
            "file",
            "Next",
            replacement_path.to_string_lossy().as_ref(),
        ),
    ];

    let second = service.rebuild_index_with_report().unwrap();
    assert_eq!(second.indexed_total, 2);
    assert_eq!(second.removed_total, 1);

    let stale_launch = service.launch(LaunchTarget::Id("file:initial"));
    match stale_launch {
        Err(ServiceError::ItemNotFound(id)) => assert_eq!(id, "file:initial"),
        other => panic!("expected pruned item to be removed, got: {other:?}"),
    }

    std::fs::remove_file(initial_path).unwrap();
    std::fs::remove_file(stable_path).unwrap();
    std::fs::remove_file(replacement_path).unwrap();
}

#[test]
fn service_search_order_is_deterministic_for_mixed_ties() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let app_path = std::env::temp_dir().join(format!("swiftfind-order-app-{unique}.exe"));
    let file_path = std::env::temp_dir().join(format!("swiftfind-order-file-{unique}.txt"));
    std::fs::write(&app_path, b"app").unwrap();
    std::fs::write(&file_path, b"file").unwrap();

    let config = test_config();
    let db = swiftfind_core::index_store::open_memory().unwrap();
    let service = CoreService::with_connection(config, db).unwrap();

    service
        .upsert_item(&SearchItem::new(
            "other",
            "doc",
            "Code Manual",
            "https://example.com/manual",
        ))
        .unwrap();
    service
        .upsert_item(&SearchItem::new("file", "file", "Code Notes", file_path.to_string_lossy().as_ref()))
        .unwrap();
    service
        .upsert_item(&SearchItem::new("app", "app", "Code", app_path.to_string_lossy().as_ref()))
        .unwrap();

    let first = service.search("code", 10).unwrap();
    let second = service.search("code", 10).unwrap();

    let first_ids: Vec<&str> = first.iter().map(|item| item.id.as_str()).collect();
    let second_ids: Vec<&str> = second.iter().map(|item| item.id.as_str()).collect();

    assert_eq!(first_ids, vec!["app", "file", "other"]);
    assert_eq!(first_ids, second_ids);

    std::fs::remove_file(app_path).unwrap();
    std::fs::remove_file(file_path).unwrap();
}
