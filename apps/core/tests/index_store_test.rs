use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn inserts_and_reads_search_item() {
    let db = swiftfind_core::index_store::open_memory().unwrap();
    let item = swiftfind_core::model::SearchItem::new("1", "app", "Code", "C:\\Code.exe")
        .with_usage(3, 1_700_000_000);

    swiftfind_core::index_store::upsert_item(&db, &item).unwrap();
    let got = swiftfind_core::index_store::get_item(&db, "1").unwrap().unwrap();

    assert_eq!(got.title, "Code");
    assert_eq!(got.use_count, 3);
    assert_eq!(got.last_accessed_epoch_secs, 1_700_000_000);
}

#[test]
fn persists_items_across_reopen() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = std::env::temp_dir()
        .join("swiftfind")
        .join(format!("persist-test-{unique}.sqlite3"));

    {
        let db = swiftfind_core::index_store::open_file(&db_path).unwrap();
        let item = swiftfind_core::model::SearchItem::new("persist-1", "file", "Report", "C:\\Report.xlsx")
            .with_usage(7, 1_800_000_000);
        swiftfind_core::index_store::upsert_item(&db, &item).unwrap();
    }

    let reopened = swiftfind_core::index_store::open_file(&db_path).unwrap();
    let got = swiftfind_core::index_store::get_item(&reopened, "persist-1")
        .unwrap()
        .unwrap();

    assert_eq!(got.title, "Report");
    assert_eq!(got.use_count, 7);
    assert_eq!(got.last_accessed_epoch_secs, 1_800_000_000);

    std::fs::remove_file(&db_path).unwrap();
}
