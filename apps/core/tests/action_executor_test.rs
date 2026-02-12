#[test]
fn rejects_empty_launch_path() {
    let result = swiftfind_core::action_executor::launch_path("");
    assert!(result.is_err());
}
