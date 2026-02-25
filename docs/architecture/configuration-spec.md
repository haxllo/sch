# Configuration Specification

## Config File Location

- Path: `%APPDATA%\\SwiftFind\\config.toml`
- Write strategy: atomic temp-write + replace
- Format: JSON5-compatible (inline `//` comments are allowed)

## Runtime Schema (Current)

```json5
{
  "hotkey": "Ctrl+Space",
  "launch_at_startup": false,
  "max_results": 20,
  "discovery_roots": [
    "C:\\Users\\<user>"
  ],
  "discovery_exclude_roots": [
    "C:\\Users\\<user>\\AppData\\Local\\Temp",
    "C:\\Users\\<user>\\AppData\\Local\\Microsoft\\Windows\\INetCache"
  ]
}
```

Additional generated fields may also exist in persisted config (for example `version`, `index_db_path`, `config_path`, and hotkey help metadata).

## Validation Rules

- `hotkey` must parse as Modifier+Key and pass runtime hotkey validation
- `max_results` range: `5..100`
- `index_db_path` and `config_path` must be present
- `discovery_roots` entries must be non-empty paths
- `discovery_exclude_roots` entries must be non-empty paths

## Discovery Include/Exclude Behavior

- Local file discovery scans only `discovery_roots`.
- Any file/folder path under `discovery_exclude_roots` is skipped.
- Exclusion is path-root based (case-insensitive normalized path comparison).
- Start-menu app discovery is independent of these filesystem roots.

## Reload/Apply Behavior

- Runtime reads config at startup.
- Hotkey changes require runtime restart to re-register globally.
- Discovery root changes apply on next index rebuild/runtime restart.

## Settings Direction

- Settings are file-driven in current product direction.
- `?` in launcher opens `%APPDATA%\\SwiftFind\\config.toml`.
- No native settings window is required for this phase.
