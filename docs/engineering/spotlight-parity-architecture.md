# Spotlight-Parity Architecture (Windows SwiftFind)

## Goal

Deliver a Spotlight-like launcher quality bar while preserving SwiftFind's runtime performance targets.

Core principle:

- performance is a hard requirement, not a tradeoff against UX quality

## Product Guarantees

1. On-device first:
- search index and ranking signals stay local by default
- no network provider is enabled by default

2. Thin UI, heavy work in background:
- overlay stays render-focused
- indexing and metadata extraction are non-UI tasks

3. Incremental indexing:
- process changed/deleted/new items incrementally
- avoid full rebuilds during normal runtime

4. Federated search:
- merge results from providers (apps, files, actions) into one ranked list
- keep provider boundaries explicit for troubleshooting

5. Predictable ranking:
- score quality combines match strength + source priority + recency/frequency
- apps first for app-like intent, local files next, other sources after

## Indexing Model

Indexing pipeline (target):

1. Detect changes from configured roots and providers.
2. Extract stable metadata only (title, path, kind, timestamps, lightweight quality signals).
3. Upsert/delete into SQLite index store.
4. Refresh hot in-memory query structures.

Rules:

- initial startup can perform full index build
- runtime mode should prefer incremental updates
- stale entries are pruned on detection and at launch failure boundaries

## Query and Ranking Model

Result ranking is provider-agnostic and merged in one list.

Ranking inputs:

- textual match quality (exact/prefix/fuzzy)
- source priority (apps > local files > other)
- recency/frequency usage signals
- safety penalties for stale/missing targets

Output constraints:

- deterministic ordering for ties
- stable keyboard navigation indexes
- max results enforced after merge/rank

## Privacy and Data Boundaries

Default privacy posture:

- all indexing and ranking are local
- logs stay under `%APPDATA%\\SwiftFind\\logs`
- config and index paths are explicit in startup logs

If online suggestions are added later:

- feature must be opt-in
- provider separated from local index pipeline
- clear UI disclosure and disable switch

## Runtime and UX Contract

Spotlight-like behavior target:

- compact idle state
- downward-only expansion on query results
- smooth, lightweight animations
- single global hotkey entry and immediate typing
- strong error visibility without blocking interaction

Non-negotiable runtime constraints:

- no extra always-on UI process
- no polling loops in idle critical path
- bounded memory growth with cache trim strategy

## Engineering Plan (Hardened)

1. Indexing foundation hardening:
- add incremental update path and explicit stale-prune pass
- add provider health instrumentation in logs

2. Ranking parity pass:
- formalize weighted ranking function with tests
- add usage-signal update hooks on successful launch

3. Provider federation:
- keep apps/files/actions providers modular
- add provider-level timing and result-count diagnostics

4. Privacy and controls:
- add include/exclude roots strategy for user control
- keep offline-first default and explicit future opt-in boundaries

5. Performance guardrails:
- extend perf tests for warm and steady-state search
- track idle memory, query latency p95, and first-result time

## Acceptance Criteria

The Spotlight-parity initiative is accepted when:

- warm query p95 remains within the existing gate
- no startup regression for normal launcher flow
- no runtime responsiveness regression with overlay closed
- result relevance improves measurably on app/file mixed queries
- stale result launch failures are reduced and self-heal quickly

