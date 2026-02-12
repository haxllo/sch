# Risk Register

## `R-001` Index Size Growth Hurts Latency

- Impact: query slowdown, poor UX
- Likelihood: medium
- Mitigation:
- Keep hot index in memory with bounded metadata
- Segment large path roots and prioritize recent paths
- Add perf tests at medium and large dataset sizes

## `R-002` File Watcher Event Loss

- Impact: stale or missing results
- Likelihood: medium
- Mitigation:
- Periodic reconciliation scan
- Track watcher health metrics
- Manual rebuild index command

## `R-003` Hotkey Conflicts

- Impact: launcher not opening for some users
- Likelihood: high
- Mitigation:
- Detect registration failure and show fallback suggestions
- Offer first-run hotkey setup

## `R-004` Unsafe Launch Semantics

- Impact: security incidents or broken launches
- Likelihood: low to medium
- Mitigation:
- Strict target validation and action allowlist
- Security-focused tests for command injection paths

## `R-005` UI Framework Overhead

- Impact: memory budget misses
- Likelihood: medium
- Mitigation:
- Keep UI process idle/offscreen when not active
- Shift ranking and indexing to core service
- Profile memory early in Phase 1
