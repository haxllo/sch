# Windows Runtime Behavior (Sprint 5)

Current hotkey-to-launcher behavior in `swiftfind-core`:

1. Start runtime with `cargo run -p swiftfind-core`.
2. Runtime loads config, logs startup mode/hotkey/paths, builds or opens index.
3. Runtime registers global hotkey from config (default `Alt+Space`).
4. When hotkey fires, runtime opens a console launcher flow:
   - prompts for query
   - prints ranked search results
   - prompts for selection number
   - launches selected item by id/path validation rules
5. Launch errors are printed in console and not swallowed.

Known limitations in this milestone:

- Launcher UX is console-driven (temporary runtime stub), not a native overlay window yet.
- Hotkey callback currently runs on the message-loop thread; while launcher prompt is active, additional hotkey events are queued until prompt flow exits.
- Runtime must stay running in its own console window/process; closing the process unregisters hotkeys.
