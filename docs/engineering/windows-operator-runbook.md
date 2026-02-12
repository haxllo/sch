# Windows Operator Runbook (Sprint 6)

This runbook covers how to run and validate the SwiftFind native Windows overlay runtime.

## Start Runtime

From repo root:

```powershell
cargo run -p swiftfind-core
```

Expected startup logs include:

- runtime mode
- configured hotkey
- config path
- index db path
- indexed item count

## Process Visibility

When running on Windows:

- `cargo run` starts child process `swiftfind-core.exe`.
- In Task Manager, look for `swiftfind-core.exe` (and `cargo.exe` while running from Cargo).

## Default Hotkey and Config

- Default hotkey: `Alt+Space`
- Config file: `%APPDATA%\SwiftFind\config.json`
- Index DB: `%APPDATA%\SwiftFind\index.sqlite3`

If config file does not exist, runtime creates defaults in the stable app-data location.

## Launcher Flow (Native Overlay)

1. Press hotkey (`Alt+Space` by default).
2. A centered floating launcher overlay appears and input is focused.
3. Type query text to fetch ranked results.
4. Use `Up` / `Down` to move selection.
5. Press `Enter` to launch selected result.
6. Press `Esc` to hide launcher.

Behavior details:

- Pressing `Alt+Space` while launcher is focused hides it.
- Pressing `Alt+Space` while launcher is visible but not focused brings focus back.
- Search and launch failures are shown in launcher status text.

## Manual Validation Checklist

Run these on a real Windows host:

1. Start runtime:
   - `cargo run -p swiftfind-core`
2. Confirm process visibility:
   - Task Manager shows `swiftfind-core.exe`.
3. Confirm hotkey:
   - `Alt+Space` opens the overlay.
4. Confirm search + launch:
   - Type a query with known indexed result.
   - Press `Enter` and verify target launches.
5. Confirm hide behavior:
   - Press `Esc` to hide overlay.
   - Press `Alt+Space` while focused to hide overlay.

## Troubleshooting Checklist

1. Hotkey does not trigger:
   - Check startup log for `hotkey registered native_id=...`.
   - Try changing hotkey in `%APPDATA%\SwiftFind\config.json` to avoid OS/app conflicts.
   - Restart runtime after config change.
   - Check if another launcher utility (PowerToys, Flow Launcher, etc.) is intercepting `Alt+Space`.
2. Overlay does not focus or appears behind apps:
   - Trigger hotkey twice (`Alt+Space`) to force refocus.
   - Ensure runtime is still active (`swiftfind-core.exe`).
   - Disable conflicting always-on-top/focus-stealing utilities while validating.
3. No results returned:
   - Check startup `indexed_items` value.
   - Confirm discovery roots in config are valid and accessible.
   - Re-run runtime to rebuild index.
4. Launch fails:
   - Read launcher status text (`Launch error: ...`).
   - Confirm selected item path still exists.
   - Verify the path is launchable from Explorer/Run dialog.
5. Process not visible:
   - Ensure `cargo run -p swiftfind-core` is still running.
   - Verify `swiftfind-core.exe` in Task Manager Details tab.
6. JS tests flaky on Windows:
   - Use `pnpm vitest --run` with repo `vitest.config.ts` (single-fork mode configured).

## Validation Commands

```powershell
pnpm vitest --run
cargo test -p swiftfind-core
cargo test -p swiftfind-core --test perf_query_latency_test -- --exact warm_query_p95_under_15ms
```
