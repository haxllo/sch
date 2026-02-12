# Windows Operator Runbook (Sprint 5)

This runbook covers how to run and validate the current SwiftFind Windows runtime milestone.

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

## Launcher Flow (Current Milestone)

1. Press hotkey (`Alt+Space` by default).
2. Runtime enters console launcher prompt.
3. Type query and press Enter.
4. Select numbered result and press Enter.
5. Runtime attempts launch and prints success/error.

## Troubleshooting Checklist

1. Hotkey does not trigger:
   - Check startup log for `hotkey registered native_id=...`.
   - Try changing hotkey in `%APPDATA%\SwiftFind\config.json` to avoid OS/app conflicts.
   - Restart runtime after config change.
2. No results returned:
   - Check startup `indexed_items` value.
   - Confirm discovery roots in config are valid and accessible.
   - Re-run runtime to rebuild index.
3. Launch fails:
   - Read printed `launcher error` text.
   - Confirm selected item path still exists.
4. Process not visible:
   - Ensure `cargo run -p swiftfind-core` is still running.
   - Verify `swiftfind-core.exe` in Task Manager Details tab.
5. JS tests flaky on Windows:
   - Use `pnpm vitest --run` with repo `vitest.config.ts` (single-fork mode configured).

## Validation Commands

```powershell
pnpm vitest --run
cargo test -p swiftfind-core
cargo test -p swiftfind-core --test perf_query_latency_test -- --exact warm_query_p95_under_15ms
```
