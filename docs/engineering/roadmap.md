# Engineering Roadmap

## Phase 0: Foundation (Week 1 to 2)

Deliverables:
- Repository scaffolding
- Rust core service skeleton
- Tauri UI shell with floating window
- Config load and save plumbing

Exit criteria:
- Hotkey opens and closes launcher window reliably

## Phase 1: Search MVP (Week 3 to 5)

Deliverables:
- App and file discovery pipeline
- SQLite cache and in-memory search index
- Fuzzy ranking and keyboard navigation
- Launch and open-folder actions

Exit criteria:
- End-to-end local search and launch flow working
- Baseline performance metrics collected

## Phase 2: Quality and Settings (Week 6 to 7)

Deliverables:
- Settings UI and runtime reload
- Usage-based ranking improvements
- Incremental indexing path (changed-items first, bounded rebuilds)
- Provider federation diagnostics (timings, counts, stale-prune visibility)
- Error and recovery UX polish
- Test coverage expansion and perf regression suite

Exit criteria:
- Meets performance and reliability targets for beta

## Phase 3: Beta Hardening (Week 8+)

Deliverables:
- Installer and auto-update path
- Crash reporting (opt-in) and diagnostics
- Security review and release documentation
- Spotlight-parity relevance and privacy hardening pass
- Include/exclude roots controls and offline-first policy enforcement

Exit criteria:
- Public beta candidate approved

## Progress Update (2026-02-13)

Completed now:
- Phase 0 exit criteria met.
- Phase 1 exit criteria met.
- Major Phase 2 quality work shipped in runtime and overlay UX.
- Installer packaging path shipped (zip artifact + setup executable workflow), moving Phase 3 forward early.

Delivered beyond original sequence:
- Native overlay redesign and interaction hardening was prioritized ahead of full Settings UI polish.
- Icon pipeline hardening for Windows shortcuts was prioritized to improve result quality on real user machines.
- Runtime close/focus behavior and result-list stability fixes were prioritized based on live user validation.

Current interpretation:
- Phase 2 is functionally complete for launcher quality and stability.
- Settings UI remains intentionally deferred for visual polish before re-enabling from `?`.
- Phase 3 is now focused on release polish, distribution discipline, and long-tail reliability.
