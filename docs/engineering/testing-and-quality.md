# Testing and Quality Strategy

## Test Layers

- Unit tests
- Tokenization, fuzzy scoring, ranking logic, config validation

- Integration tests
- Core service IPC contract, index cache load/save, action execution paths

- End-to-end tests
- Hotkey invocation, query interaction, result launch from UI

- Performance tests
- Hotkey open latency
- Query latency at different index sizes
- Idle CPU and memory soak tests

## CI Quality Gates

- Lint and format checks pass
- Unit and integration tests pass
- No critical dependency vulnerabilities
- Performance regression check within tolerance thresholds

## Performance Regression Policy

- Track baseline with fixed dataset sizes:
- Small: 5k indexed items
- Medium: 50k indexed items
- Large: 250k indexed items

- Fail build if:
- P95 warm query latency regresses by > 20%
- Idle memory exceeds budget by > 15%

## Release Readiness Checklist

- Crash-free soak run >= 24h
- Manual validation of top user flows
- Accessibility keyboard-only pass
- Upgrade path validated for existing config and index database

## MVP Smoke and Performance Baseline

- E2E smoke path placeholder added: `tests/e2e/hotkey-search-launch.spec.ts`
- Perf baseline test added: `tests/perf/query_latency_test.rs`
- Current warm query p95 baseline: `12.4ms` (target `<= 15ms`)
