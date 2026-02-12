# ADR-001: Core Stack and Process Model

- Status: accepted
- Date: 2026-02-11

## Context

The product requires:
- Very fast query handling
- Low idle resource usage
- Modern UI
- Stable integration with Windows hotkeys and file discovery APIs

## Decision

Use:
- Rust for the core service (`swiftfind-core`)
- Tauri with React and TypeScript for UI (`swiftfind-ui`)
- SQLite for local cache and metadata persistence

Use a two-process model:
- Always-on core process
- On-demand UI process and window

## Rationale

- Rust gives high performance and predictable memory behavior
- Tauri allows modern UI without shipping a heavy runtime
- Separation keeps search/indexing independent from UI rendering

## Consequences

Positive:
- Better performance control
- Easier profiling and reliability isolation
- UI technology flexibility

Negative:
- IPC complexity between core and UI
- More packaging and process orchestration work

## Follow-up ADRs

- `ADR-002`: IPC protocol and versioning strategy
- `ADR-003`: Index storage schema and migration approach
- `ADR-004`: Plugin or extension model boundaries
