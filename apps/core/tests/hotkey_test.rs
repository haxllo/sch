#[test]
fn parses_default_hotkey() {
    let parsed = swiftfind_core::hotkey::parse_hotkey("Alt+Space").unwrap();
    assert_eq!(parsed.key, "Space");
}
