# Windows Runtime Behavior (Current)

Current hotkey-to-launcher behavior in `swiftfind-core`:

1. Start runtime with `cargo run -p swiftfind-core`.
2. Runtime loads config (JSON/JSON5 with comments), logs startup mode/hotkey/paths, builds or opens index.
3. Runtime registers global hotkey from config (default `Ctrl+Shift+Space`).
4. Runtime creates a native borderless top-most launcher window (hidden by default).
5. Hotkey behavior:
   - configured hotkey shows launcher and focuses input when hidden.
   - configured hotkey hides launcher when launcher is already focused.
   - configured hotkey refocuses launcher if visible but not focused.
6. Launcher interaction:
   - typing runs core search against indexed items
   - `Up`/`Down` changes selection
   - `Enter` launches selected result and hides launcher immediately on success
   - single-click on a result launches it immediately
   - `Esc` hides launcher
   - clicking outside launcher hides launcher
   - any close path clears query/results so next open starts fresh
   - clicking `?` opens `%APPDATA%\SwiftFind\config.json` for manual edits
7. Search and launch errors are surfaced inside launcher status text.
8. Settings persistence behavior:
   - config remains source of truth (`%APPDATA%\SwiftFind\config.json`)
   - save path uses safe temp-write + replace flow
   - startup behavior is controlled by config values
9. Visual/UX polish:
   - compact Spotlight/Wofi-like default bar state
   - no results panel shown when query is empty
   - results panel expands downward only when query has matches
   - panel background `#101010` with border `#2A2A2A`
   - structured two-line result rows (`title` + `path`) with type glyph
   - rounded results panel; top edge flush to input section
   - bottom margin matches left/right margin
   - hover-driven row emphasis (no hard selected border)
   - lightweight show/hide and results expansion animations with smooth scroll behavior

## Runtime Host and Lifecycle

- `--background`: starts detached background runtime process.
- `--foreground`: explicit attached mode (primarily for debugging).
- `--status`: reports whether an existing instance is running.
- `--quit`: signals existing instance to close.
- `--restart`: signals existing instance to close, then starts runtime again.
- `--ensure-config`: creates config template if missing.
- `--sync-startup`: applies `launch_at_startup` from config to Windows Run key.

Single-instance behavior:

- Mutex guard enforces one active runtime.
- Duplicate launches signal existing instance to show/focus overlay instead of creating a second hotkey owner.

Known limitations in this milestone:

- Launcher is native Win32 shell (not a full React/WebView overlay).
- Animations are intentionally lightweight to prioritize runtime stability.
- Runtime must remain active in its process; stopping `swiftfind-core.exe` unregisters hotkey and closes launcher.
- Hotkey registration changes still require process restart to apply globally.
- Native settings UI is temporarily disabled from `?` pending design polish pass.

Operator steps and troubleshooting are documented in:
- `docs/engineering/windows-operator-runbook.md`

Screenshot capture checklist is documented in:
- `docs/engineering/windows-operator-runbook.md` (`Screenshot Notes (Before/After)`)
