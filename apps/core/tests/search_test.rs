#[test]
fn typo_query_returns_expected_match() {
    let items = vec![swiftfind_core::model::SearchItem::new(
        "1",
        "file",
        "Q4_Report.xlsx",
        "C:\\Q4_Report.xlsx",
    )];
    let results = swiftfind_core::search::search(&items, "q4 reort", 10);
    assert_eq!(results[0].id, "1");
}
