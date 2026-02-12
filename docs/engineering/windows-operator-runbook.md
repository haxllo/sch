# Windows Operator Runbook (Current)

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

- Default hotkey: `Ctrl+Shift+Space`
- Config file: `%APPDATA%\SwiftFind\config.json`
- Index DB: `%APPDATA%\SwiftFind\index.sqlite3`

If config file does not exist, runtime writes defaults to the stable app-data location on startup.

## Change Hotkey Safely

1. Open `%APPDATA%\SwiftFind\config.json`.
2. Update the `hotkey` value.
3. Restart `swiftfind-core`.

Recommended low-conflict hotkeys on Windows:

- `Ctrl+Shift+Space` (default)
- `Ctrl+Alt+Space`
- `Alt+Shift+Space`
- `Ctrl+Shift+P`

Avoid these common system/reserved shortcuts:

- `Win+...` combinations
- `Alt+Tab`
- `Ctrl+Esc`
- `Alt+Space` (can conflict with the window system menu)

## Launcher Flow (Native Overlay)

1. Press hotkey (`Ctrl+Shift+Space` by default).
2. A centered floating launcher overlay appears and input is focused.
3. Type query text to fetch ranked results.
4. Use `Up` / `Down` to move selection.
5. Press `Enter` to launch selected result, or single-click a result row.
6. Press `Esc` to hide launcher.

Behavior details:

- Pressing the configured hotkey while launcher is focused hides it.
- Pressing the configured hotkey while launcher is visible but not focused brings focus back.
- Clicking outside launcher hides it.
- Closing launcher clears current query/results (next open is clean).
- Search and launch failures are shown in launcher status text.

## UI Characteristics (Final)

- Compact centered launcher bar (default compact height; no oversized blank state).
- Panel colors:
  - background `#101010`
  - border `#2A2A2A` (1px)
- Rounded panel with subtle depth and improved typography hierarchy.
- Input placeholder shown directly in the input box (`Type to search`).
- Result rows are structured two-line cards:
  - primary title line (higher contrast, semibold)
  - secondary path line (muted, ellipsized)
- Per-item glyph marks item type (`app`, `file`, `folder`) with restrained icon boxes.
- Active row uses soft hover-style emphasis without a hard selection border.
- Input focus is forced on open and text is selected for immediate re-query.
- Results panel stays collapsed for empty query, and expands downward only after matching query text.
- Results panel has no top gap and uses matched side/bottom spacing.
- Overlay and results transitions are short and smooth:
  - show/hide fade + scale (~150ms)
  - results expand/collapse height + opacity (~150ms)
  - selection transition (~90ms)

## Results Section Rationale (Before/After)

Before:
- rows used compact single-line text and weak hierarchy; long paths reduced legibility.
- selection state was visible but visually close to non-selected rows.

After:
- two-line row hierarchy improves scan speed and disambiguation for similarly named items.
- type glyph gives quick context without visual noise.
- path line remains readable while preserving compact density.
- selection and hover are distinct but harmonious for keyboard + mouse workflows.

## Manual Validation Checklist

Run these on a real Windows host:

1. Start runtime:
   - `cargo run -p swiftfind-core`
2. Confirm process visibility:
   - Task Manager shows `swiftfind-core.exe`.
3. Confirm hotkey:
   - configured hotkey opens the overlay.
4. Confirm search + launch:
   - Type a query with known indexed result.
   - Press `Enter` and verify target launches.
5. Confirm hide behavior:
   - Press `Esc` to hide overlay.
   - Press configured hotkey while focused to hide overlay.
   - Click outside overlay and verify it hides immediately.
   - Reopen overlay and verify prior query text is cleared.
6. Confirm visual polish:
   - Window appears compact and centered (not oversized).
   - Empty query shows compact bar only (no visible results list).
   - Typing matching query expands results downward from a fixed top edge.
   - Result rows render as clean two-line title/path entries (no literal tab separators).
   - Result rows have no hard border selection box; emphasis is subtle/hover-style.
   - Status line color changes for error states.

## Screenshot Notes (Before/After)

Capture two screenshots on a Windows host for release notes:

1. Before polish reference:
   - plain large launcher shell from earlier Sprint 6 state.
2. After polish reference:
   - compact rounded launcher shell with styled input/list/status and fade transitions.

Recommended capture points:

- `overlay-idle.png`: just opened (configured hotkey), empty query, focused input.
- `overlay-results.png`: populated results with one selected row.
- `overlay-error.png`: launch/search error status visible.
- `overlay-expand.gif` (or short video): compact bar expanding downward while typing.
- `overlay-before-after-rows.png`: side-by-side comparison of old vs redesigned results rows.

## Troubleshooting Checklist

1. Hotkey does not trigger:
   - Check startup log for `hotkey registered native_id=...`.
   - Try changing hotkey in `%APPDATA%\SwiftFind\config.json` to avoid OS/app conflicts.
   - Restart runtime after config change.
   - Check if another launcher utility (PowerToys, Flow Launcher, etc.) is intercepting your chosen hotkey.
2. Overlay does not focus or appears behind apps:
   - Trigger the configured hotkey twice to force refocus.
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
