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
- Config-file settings workflow and runtime reload commands
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
- Native overlay redesign and interaction hardening was prioritized ahead of broader post-beta feature additions.
- Icon pipeline hardening for Windows shortcuts was prioritized to improve result quality on real user machines.
- Runtime close/focus behavior and result-list stability fixes were prioritized based on live user validation.

Current interpretation:
- Phase 2 is functionally complete for launcher quality and stability.
- No native Settings window is planned in the near term; config-file workflow remains the supported settings path.
- Phase 3 is now focused on release polish, distribution discipline, and long-tail reliability.

## Phase 3 Status (Verified 2026-02-13)

Item status:
- Installer path: completed (zip artifact + Inno Setup `setup.exe` packaging flow).
- Auto-update path: not started.
- Crash diagnostics: completed (panic hook + runtime diagnostics logs).
- Security review: pending final pre-beta review pass.
- Release documentation: completed baseline (packaging/readiness/runbook docs present).
- Spotlight-parity hardening: in progress (ranking/UX/shortcut icon quality improved).
- Include/exclude roots controls: not started.
- Offline-first policy: completed by default architecture (local index/ranking path).

## Phase 3 Status (Verified 2026-02-14)

Item status:
- Installer path: completed (zip artifact + Inno Setup `setup.exe` packaging flow).
- Update strategy: completed at operations/docs level (`stable`/`beta`, upgrade, rollback policy).
- Runtime auto-update mechanism: not implemented (intentionally deferred).
- Crash diagnostics: completed (panic hook + runtime diagnostics logs).
- Security review: pending final pre-beta review pass.
- Release documentation: completed baseline with lifecycle validation and rollout strategy docs.
- Spotlight-parity hardening: in progress (ranking/UX/shortcut icon quality improved).
- Include/exclude roots controls: not started.
- Offline-first policy: completed by default architecture (local index/ranking path).
