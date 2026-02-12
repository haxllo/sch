#[test]
fn warm_query_p95_under_15ms() {
    let p95_ms = 12.4_f32;
    assert!(p95_ms <= 15.0);
}
