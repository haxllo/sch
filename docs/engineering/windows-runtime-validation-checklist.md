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

4. Use `ArrowDown` / `ArrowUp` to change selected result.
- Expected: selected row changes as keys are pressed.

5. Press `Enter` on a valid result.
- Expected: selected launch path is executed.

6. Trigger an invalid launch target (missing path or denied access).
- Expected: user-visible error message appears in launcher UI.

Record pass/fail evidence:

```powershell
scripts/windows/record-manual-e2e.ps1
```

Expected output file:

- `artifacts/windows/manual-e2e-result.json`
- `all_passed: true` for release readiness.

## Release Blockers

Do not mark the milestone release-ready if any manual check above fails.
