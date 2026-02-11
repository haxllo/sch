use swiftfind_core::core_service::CoreService;
use swiftfind_core::discovery::{AppProvider, DiscoveryProvider, FileProvider};

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

#[test]
fn rebuild_index_uses_registered_providers() {
    let config = swiftfind_core::config::Config::default();
    let db = swiftfind_core::index_store::open_memory().unwrap();

    let service = CoreService::with_connection(config, db)
        .unwrap()
        .with_providers(vec![
            Box::new(AppProvider::deterministic_fixture()),
            Box::new(FileProvider::deterministic_fixture()),
        ]);

    let inserted = service.rebuild_index().unwrap();
    assert_eq!(inserted, 4);

    let results = service.search("report", 10).unwrap();
    assert_eq!(results[0].id, "file-report");
}
