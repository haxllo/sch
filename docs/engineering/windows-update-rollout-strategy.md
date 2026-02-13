# Windows Update and Rollout Strategy

This document defines SwiftFind Windows update behavior for Phase 3.

## Scope

- Applies to packaged Windows releases (`setup.exe` + zip).
- Keeps runtime lightweight: no always-on auto-update service.
- Uses release-channel policy and operator-driven rollout.

## Current Update Model

Current behavior is manual update:

1. User installs new `swiftfind-<version>-windows-x64-setup.exe`.
2. Installer upgrades in place under `%LOCALAPPDATA%\Programs\SwiftFind`.
3. Existing `%APPDATA%\SwiftFind\config.json` and index are preserved.
4. Runtime is restarted cleanly through installer/uninstall lifecycle rules.

No background update polling is performed by `swiftfind-core.exe`.

## Release Channels

Two channels are supported at release-management level:

1. `stable`
- General users.
- Must pass full automated gates and full Windows manual lifecycle validation.

2. `beta`
- Early testers.
- Can ship faster, but still requires core runtime and lifecycle checks.

Channel is communicated in release notes/tag naming; runtime binary remains the same architecture.

## Versioning and Publishing

Publishing baseline:

1. Build artifacts:
- `artifacts/windows/swiftfind-<version>-windows-x64.zip`
- `artifacts/windows/swiftfind-<version>-windows-x64-manifest.json`
- `artifacts/windows/swiftfind-<version>-windows-x64-setup.exe`

2. Publish GitHub release with clear channel label.
3. Attach validation evidence summary and known limitations.

Recommended tag style:

- Stable: `vX.Y.Z`
- Beta: `vX.Y.Z-beta.N`

## Upgrade and Rollback Policy

Upgrade:

- Supported path is in-place install of newer setup over existing installation.
- Upgrade must preserve config and keep launcher startup behavior functional.

Rollback:

- Reinstall previous known-good setup version.
- Verify runtime process, hotkey, query/launch/close flow immediately after rollback.

Rollback is required for every release candidate before broad stable rollout.

## Future Auto-Update Direction (Not Implemented Yet)

Future auto-update can be considered only if all constraints are met:

- no persistent always-on updater process in idle runtime path
- explicit user consent and clear channel selection
- signed payload verification before install
- rollback-safe failure behavior

Until then, manual setup-based updates remain the official update path.
