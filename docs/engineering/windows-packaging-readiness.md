# Windows Packaging Readiness

## Artifact Convention

- Output root: `artifacts/windows/`
- Staging directory: `artifacts/windows/swiftfind-<version>-windows-x64-stage/`
- Zip artifact: `artifacts/windows/swiftfind-<version>-windows-x64.zip`
- Build manifest: `artifacts/windows/swiftfind-<version>-windows-x64-manifest.json`

## Version Stamping Policy

- Preferred: explicit `-Version` argument in packaging command.
- Fallback: `git describe --tags --always`.
- If git metadata is unavailable: `0.0.0-local`.

## Packaging Command

Run on Windows PowerShell:

```powershell
scripts/windows/package-windows-artifact.ps1 -Version "0.4.0-milestone"
```

Signed packaging (PFX):

```powershell
scripts/windows/package-windows-artifact.ps1 `
  -Version "0.4.0-milestone" `
  -Sign `
  -CertPath "C:\secure\certs\swiftfind-signing.pfx" `
  -CertPassword "<PFX_PASSWORD>"
```

Expected outputs:

- `artifacts/windows/swiftfind-0.4.0-milestone-windows-x64.zip`
- `artifacts/windows/swiftfind-0.4.0-milestone-windows-x64-manifest.json`
- `artifacts/windows/swiftfind-0.4.0-milestone-windows-x64-stage/`

## Included Payload

- `bin/swiftfind-core.exe`
- `assets/swiftfinder.svg`
- `docs/windows-runtime-validation-checklist.md`
- `docs/release-notes-template.md`
- `scripts/install-swiftfind.ps1`
- `scripts/uninstall-swiftfind.ps1`

## Signing Requirements

- `signtool.exe` available in `PATH` (Windows SDK).
- Valid Authenticode certificate (`.pfx`).
- Timestamp server reachable (default: `http://timestamp.digicert.com`).

If `-Sign` is enabled, packaging script will:

1. Sign `target/release/swiftfind-core.exe`
2. Verify signature via `signtool verify`
3. Validate status via `Get-AuthenticodeSignature`

If any of these fail, packaging fails.

## Installer-Prep Notes

- This milestone prepares a deterministic zip payload and manifest.
- Installer generation (MSI/EXE) can consume the staging directory directly.
- Keep staging layout stable to avoid installer script churn.

## Local Install / Uninstall Commands

Run on Windows PowerShell:

```powershell
scripts/windows/install-swiftfind.ps1
```

End-user note:

- Rust/Cargo is **not required** when installing from the packaged release zip.
- The installer script uses the prebuilt `bin/swiftfind-core.exe` from the zip.

Optional flags:

- `-BuildFromSource` (developer mode; builds with Cargo if prebuilt exe is not present)
- `-SourceExe "<path>"` (explicit exe path override)
- `-StartAfterInstall:$false` (install only)

Uninstall:

```powershell
scripts/windows/uninstall-swiftfind.ps1
```

Optional:

- `-PurgeUserData` (also removes `%APPDATA%\SwiftFind`)
