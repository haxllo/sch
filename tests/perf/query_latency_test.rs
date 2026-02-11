use std::time::Instant;

use swiftfind_core::model::SearchItem;
use swiftfind_core::search::search;

fn p95_ms(samples: &mut [f64]) -> f64 {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let last = samples.len().saturating_sub(1);
    let idx = ((last as f64) * 0.95).round() as usize;
    samples[idx.min(last)]
}

#[test]
fn warm_query_p95_under_15ms() {
    let mut items: Vec<SearchItem> = (0..10_000)
        .map(|i| {
            SearchItem::new(
                &i.to_string(),
                "file",
                &format!("Document_{i:05}.txt"),
                &format!("C:\\Docs\\Document_{i:05}.txt"),
            )
        })
        .collect();

    items.push(SearchItem::new(
        "q4",
        "file",
        "Q4_Report.xlsx",
        "C:\\Reports\\Q4_Report.xlsx",
    ));

    let warmup = search(&items, "q4 reort", 20);
    assert!(!warmup.is_empty(), "warmup search returned no results");

    let mut samples = Vec::with_capacity(120);
    for _ in 0..120 {
        let start = Instant::now();
        let _ = search(&items, "q4 reort", 20);
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let measured_p95 = p95_ms(&mut samples);
    assert!(
        measured_p95 <= 15.0,
        "p95 query latency too high: {measured_p95:.3}ms (budget 15.0ms)",
    );
}
