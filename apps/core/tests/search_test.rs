use swiftfind_core::model::SearchItem;

#[test]
fn typo_query_returns_expected_match() {
    let items = vec![SearchItem::new(
        "1",
        "file",
        "Q4_Report.xlsx",
        "C:\\Q4_Report.xlsx",
    )];

    let results = swiftfind_core::search::search(&items, "q4 reort", 10);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "1");
}

#[test]
fn ranks_better_matches_first() {
    let items = vec![
        SearchItem::new("1", "app", "Code", "C:\\Code.exe"),
        SearchItem::new("2", "app", "Codeium", "C:\\Codeium.exe"),
        SearchItem::new("3", "doc", "Decode Notes", "C:\\DecodeNotes.txt"),
    ];

    let results = swiftfind_core::search::search(&items, "code", 10);

    let ids: Vec<&str> = results.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(ids, vec!["1", "2", "3"]);
}

#[test]
fn empty_query_returns_no_results() {
    let items = vec![SearchItem::new("1", "app", "Code", "C:\\Code.exe")];

    let results = swiftfind_core::search::search(&items, "   ", 10);

    assert!(results.is_empty());
}

#[test]
fn honors_result_limit() {
    let items = vec![
        SearchItem::new("1", "doc", "Document One", "C:\\Docs\\one.txt"),
        SearchItem::new("2", "doc", "Document Two", "C:\\Docs\\two.txt"),
        SearchItem::new("3", "doc", "Document Three", "C:\\Docs\\three.txt"),
    ];

    let results = swiftfind_core::search::search(&items, "document", 2);

    assert_eq!(results.len(), 2);
}
