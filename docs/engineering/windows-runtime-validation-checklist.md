# Windows Runtime Validation Checklist

Use this checklist on a real Windows host after building the current branch.

## Preconditions

- Rust toolchain available (`cargo --version`)
- Node + pnpm available (`node -v`, `pnpm -v`)
- Repository dependencies installed (`pnpm install`)

## Automated Validation

Run:

```powershell
scripts/windows/run-sprint4-validation.ps1
```

Expected:

- Windows runtime smoke harness test passes.
- Launcher UI flow tests pass.

## Manual E2E Flow (Required)

1. Start runtime/application process for this milestone build.
- Expected: process is running without immediate crash.

2. Press `Alt+Space` with another app focused.
- Expected: launcher overlay opens; query input is focused.
- Expected: launcher opens in compact bar state (no visible results list).

3. Type a query that should match indexed content (for example `code` or `report`).
- Expected: result list updates with real indexed items.
- Expected: results panel expands downward only (top edge remains fixed).
- Expected: rows show clean title + path hierarchy (no raw tab separators).

4. Use `ArrowDown` / `ArrowUp` to change selected result.
- Expected: selected row changes as keys are pressed.
- Expected: moving mouse over rows updates the same active row state (no separate stale selected+hovered highlight).
- Expected: first wheel movement after query update scrolls in discrete steps (3 rows per notch), without one-time easing.

5. Press `Enter` on a valid result.
- Expected: selected launch path is executed.
 - Expected: launcher closes immediately after successful launch.

6. Single-click a valid result row.
- Expected: clicked result launches immediately (no separate confirm click required).

7. Trigger an invalid launch target (missing path or denied access).
- Expected: user-visible error message appears in launcher UI.

8. Close behavior checks.
- Press `Esc`: launcher hides and query clears.
- Click outside launcher: launcher hides and query clears.
- Reopen with hotkey: input starts clean with no stale query text.

9. Settings access checks.
- Click `?` in launcher input area.
- Expected: `%APPDATA%\SwiftFind\config.json` opens for manual edits.
- Edit `hotkey` or `max_results`, save, restart runtime, verify behavior updates.

10. Future settings UI note.
- Native settings UI is intentionally disabled from `?` for now.
- Expected: no crash; manual config path remains available.

11. Lifecycle command checks.
- Run `swiftfind-core.exe --status` while runtime is active.
- Expected: reports running.
- Run `swiftfind-core.exe --quit`, then `--status`.
- Expected: reports stopped after quit.

12. Clean install checks.
- Install from packaged artifact (`setup.exe` or install script from zip).
- Expected: install completes without requiring Rust/Cargo.
- Expected: runtime can start and hotkey works on first launch.

13. Upgrade-over-existing checks.
- Install a newer build over an existing installed build.
- Expected: install succeeds without manual uninstall.
- Expected: runtime restarts cleanly and hotkey registration remains valid.
- Expected: config file in `%APPDATA%\SwiftFind\config.json` is preserved.

14. Uninstall + reinstall checks.
- Uninstall from Windows Apps settings or installer uninstaller.
- Expected: runtime process is no longer present in Task Manager.
- Expected: hotkey no longer triggers launcher.
- Reinstall latest setup.
- Expected: launcher works again and startup registration can be applied.

15. Rollback checks.
- After installing a newer build, reinstall the previous known-good build.
- Expected: older runtime starts successfully.
- Expected: no stuck background process from replaced version.
- Expected: core launcher flow (open, query, launch, close) works after rollback.

Record pass/fail evidence:

```powershell
scripts/windows/record-manual-e2e.ps1
```

Expected output file:

- `artifacts/windows/manual-e2e-result.json`
- `all_passed: true` for release readiness.

## Release Blockers

Do not mark the milestone release-ready if any manual check above fails.
