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

Expected outputs:

- `artifacts/windows/swiftfind-0.4.0-milestone-windows-x64.zip`
- `artifacts/windows/swiftfind-0.4.0-milestone-windows-x64-manifest.json`
- `artifacts/windows/swiftfind-0.4.0-milestone-windows-x64-stage/`

## Included Payload

- `bin/swiftfind-core.exe`
- `docs/windows-runtime-validation-checklist.md`
- `docs/release-notes-template.md`

## Installer-Prep Notes

- This milestone prepares a deterministic zip payload and manifest.
- Installer generation (MSI/EXE) can consume the staging directory directly.
- Keep staging layout stable to avoid installer script churn.
