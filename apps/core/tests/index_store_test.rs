#[test]
fn inserts_and_reads_search_item() {
    let db = swiftfind_core::index_store::open_memory().unwrap();
    let item = swiftfind_core::model::SearchItem::new("1", "app", "Code", "C:\\Code.exe");
    swiftfind_core::index_store::upsert_item(&db, &item).unwrap();
    let got = swiftfind_core::index_store::get_item(&db, "1").unwrap().unwrap();
    assert_eq!(got.title, "Code");
}
