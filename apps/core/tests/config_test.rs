use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

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
    assert_eq!(cfg.version, swiftfind_core::config::CURRENT_CONFIG_VERSION);
    assert_eq!(cfg.hotkey, "Ctrl+Shift+Space");
    assert!(!cfg.launch_at_startup);
    assert!(!cfg.hotkey_help.trim().is_empty());
    assert!(!cfg.hotkey_recommended.is_empty());
    assert!(cfg.index_db_path.to_string_lossy().contains("swiftfind") || cfg.index_db_path.to_string_lossy().contains("SwiftFind"));
    assert!(cfg.config_path.to_string_lossy().contains("swiftfind") || cfg.config_path.to_string_lossy().contains("SwiftFind"));
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

    drop(db);
    std::fs::remove_file(&cfg.index_db_path).unwrap();
}

#[test]
fn loads_default_when_config_file_missing() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let config_path = std::env::temp_dir()
        .join("swiftfind")
        .join(format!("missing-config-{unique}.json"));

    let cfg = swiftfind_core::config::load(Some(&config_path)).unwrap();

    assert_eq!(cfg.config_path, config_path);
    assert_eq!(cfg.max_results, 20);
    assert_eq!(cfg.version, swiftfind_core::config::CURRENT_CONFIG_VERSION);
}

#[test]
fn saves_and_reloads_config_file() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let config_path = std::env::temp_dir()
        .join("swiftfind")
        .join(format!("save-reload-{unique}.json"));

    let mut cfg = swiftfind_core::config::Config::default();
    cfg.config_path = config_path.clone();
    cfg.max_results = 33;
    cfg.hotkey = "Ctrl+Space".to_string();
    cfg.launch_at_startup = true;
    cfg.discovery_roots = vec![std::env::temp_dir().join("root-a")];

    swiftfind_core::config::save(&cfg).unwrap();
    let loaded = swiftfind_core::config::load(Some(&config_path)).unwrap();

    assert_eq!(loaded.max_results, 33);
    assert_eq!(loaded.hotkey, "Ctrl+Space");
    assert!(loaded.launch_at_startup);
    assert_eq!(loaded.discovery_roots.len(), 1);
    assert_eq!(loaded.version, swiftfind_core::config::CURRENT_CONFIG_VERSION);

    std::fs::remove_file(&config_path).unwrap();
}

#[test]
fn loads_partial_config_with_migration_safe_defaults() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let config_path = std::env::temp_dir()
        .join("swiftfind")
        .join(format!("partial-{unique}.json"));

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    std::fs::write(
        &config_path,
        r#"{ "max_results": 25, "config_path": "placeholder" }"#,
    )
    .unwrap();

    let loaded = swiftfind_core::config::load(Some(&config_path)).unwrap();

    assert_eq!(loaded.max_results, 25);
    assert_eq!(loaded.version, swiftfind_core::config::CURRENT_CONFIG_VERSION);
    assert_eq!(loaded.hotkey, "Ctrl+Shift+Space");
    assert!(!loaded.launch_at_startup);
    assert_eq!(loaded.config_path, config_path);
    assert!(!loaded.index_db_path.as_os_str().is_empty());
    assert!(!loaded.hotkey_help.trim().is_empty());
    assert!(!loaded.hotkey_recommended.is_empty());

    if Path::new(&loaded.config_path).exists() {
        std::fs::remove_file(&loaded.config_path).unwrap();
    }
}

#[test]
fn writes_user_template_with_comments_and_loads_it() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let config_path = std::env::temp_dir()
        .join("swiftfind")
        .join(format!("template-{unique}.json"));

    let mut cfg = swiftfind_core::config::Config::default();
    cfg.config_path = config_path.clone();

    swiftfind_core::config::write_user_template(&cfg, &config_path).unwrap();
    let raw = std::fs::read_to_string(&config_path).unwrap();
    assert!(raw.contains("// SwiftFind config (comments are allowed)."));
    assert!(raw.contains("\"hotkey\": \"Ctrl+Shift+Space\""));
    assert!(raw.contains("// \"hotkey\": \"Ctrl+Alt+Space\""));
    assert!(!raw.contains("\"index_db_path\""));

    let loaded = swiftfind_core::config::load(Some(&config_path)).unwrap();
    assert_eq!(loaded.hotkey, cfg.hotkey);
    assert_eq!(loaded.max_results, cfg.max_results);

    std::fs::remove_file(&config_path).unwrap();
}
