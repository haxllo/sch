pub mod action_executor;
pub mod config;
pub mod hotkey;
pub mod index_store;
pub mod model;
pub mod search;

#[cfg(test)]
mod tests {
    mod query_latency_test {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/perf/query_latency_test.rs"
        ));
    }
}
