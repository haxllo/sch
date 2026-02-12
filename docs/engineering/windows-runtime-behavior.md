# Windows Runtime Behavior (Sprint 6)

Current hotkey-to-launcher behavior in `swiftfind-core`:

1. Start runtime with `cargo run -p swiftfind-core`.
2. Runtime loads config, logs startup mode/hotkey/paths, builds or opens index.
3. Runtime registers global hotkey from config (default `Alt+Space`).
4. Runtime creates a native borderless top-most launcher window (hidden by default).
5. Hotkey behavior:
   - `Alt+Space` shows launcher and focuses input when hidden.
   - `Alt+Space` hides launcher when launcher is already focused.
   - `Alt+Space` refocuses launcher if visible but not focused.
6. Launcher interaction:
   - typing runs core search against indexed items
   - `Up`/`Down` changes selection
   - `Enter` launches selected result and hides launcher on success
   - `Esc` hides launcher
7. Search and launch errors are surfaced inside launcher status text.

Known limitations in this milestone:

- Launcher is native Win32 shell (not yet a full React/WebView overlay).
- Custom theming/animations are minimal and intentionally lightweight for runtime stability.
- Runtime must remain active in its process; stopping `swiftfind-core.exe` unregisters hotkey and closes launcher.

Operator steps and troubleshooting are documented in:
- `docs/engineering/windows-operator-runbook.md`
