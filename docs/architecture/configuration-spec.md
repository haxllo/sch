# Configuration Specification

## Config File Location

- Path: `%AppData%\\SwiftFind\\config.json`
- Write strategy: atomic write with temp file + rename

## Schema (MVP)

```json
{
  "hotkey": "Alt+Space",
  "maxResults": 20,
  "theme": "system",
  "indexedPaths": [
    "C:\\\\Users\\\\<user>\\\\Documents",
    "D:\\\\Projects"
  ],
  "excludePatterns": [
    "**\\\\node_modules\\\\**",
    "**\\\\.git\\\\**",
    "**\\\\bin\\\\**"
  ],
  "search": {
    "enableFuzzy": true,
    "typoTolerance": 2,
    "usageWeight": 0.35,
    "recencyWeight": 0.25
  },
  "actions": {
    "confirmRunAsAdmin": true,
    "openFolderShortcut": "Ctrl+Enter"
  },
  "privacy": {
    "telemetryEnabled": false
  }
}
```

## Validation Rules

- `hotkey` must be globally registerable and not empty
- `maxResults` range: 5 to 100
- `indexedPaths` must be absolute paths
- `typoTolerance` range: 0 to 3
- `usageWeight` and `recencyWeight` range: 0.0 to 1.0

## Dynamic Reload

- Core watches config file for changes
- Valid updates apply without full process restart
- Invalid config retains last known good state and emits error in settings UI

## Future Extensions

- Per-source ranking weights
- Per-path indexing profiles
- Import and export settings bundles
