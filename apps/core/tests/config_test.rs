#[test]
fn rejects_max_results_out_of_range() {
    let cfg = swiftfind_core::config::Config {
        max_results: 200,
        ..Default::default()
    };
    assert!(swiftfind_core::config::validate(&cfg).is_err());
}
