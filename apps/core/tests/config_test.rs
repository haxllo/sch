#[test]
fn rejects_max_results_out_of_range() {
    let cfg = swiftfind_core::config::Config {
        max_results: 200,
        ..Default::default()
    };
    assert!(swiftfind_core::config::validate(&cfg).is_err());
}

#[test]
fn accepts_default_config() {
    let cfg = swiftfind_core::config::Config::default();
    assert_eq!(cfg.max_results, 20);
    assert!(cfg.index_db_path.to_string_lossy().contains("swiftfind"));
    assert!(cfg.config_path.to_string_lossy().contains("swiftfind"));
    assert!(swiftfind_core::config::validate(&cfg).is_ok());
}

#[test]
fn opens_index_store_from_config_path() {
    let mut cfg = swiftfind_core::config::Config::default();
    cfg.index_db_path = std::env::temp_dir().join("swiftfind").join("cfg-open.sqlite3");

    let db = swiftfind_core::index_store::open_from_config(&cfg).unwrap();
    let item = swiftfind_core::model::SearchItem::new("cfg-1", "app", "Terminal", "C:\\Terminal.exe");
    swiftfind_core::index_store::upsert_item(&db, &item).unwrap();

    let got = swiftfind_core::index_store::get_item(&db, "cfg-1").unwrap();
    assert!(got.is_some());

    std::fs::remove_file(&cfg.index_db_path).unwrap();
}
