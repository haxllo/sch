param(
  [string]$Version,
  [string]$OutputRoot = "artifacts/windows",
  [string]$InnoCompiler = "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
)

$ErrorActionPreference = "Stop"

if (-not $Version -or $Version.Trim().Length -eq 0) {
  try {
    $Version = (git describe --tags --always).Trim()
  }
  catch {
    $Version = "0.0.0-local"
  }
}

$artifactName = "swiftfind-$Version-windows-x64"
$stageDir = Join-Path $OutputRoot "$artifactName-stage"
$issPath = "scripts/windows/swiftfind.iss"

Write-Host "== Building SwiftFind Setup.exe for $Version ==" -ForegroundColor Cyan

if (-not (Test-Path $InnoCompiler)) {
  throw "Inno Setup compiler not found at '$InnoCompiler'. Install Inno Setup or pass -InnoCompiler with the full ISCC.exe path."
}

if (-not (Test-Path $issPath)) {
  throw "Installer spec not found at '$issPath'."
}

if (-not (Test-Path $stageDir)) {
  Write-Host "Staged artifact not found. Building package stage first..." -ForegroundColor Yellow
  & "scripts/windows/package-windows-artifact.ps1" -Version $Version -OutputRoot $OutputRoot
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to build staged artifact."
  }
}

if (-not (Test-Path (Join-Path $stageDir "bin/swiftfind-core.exe"))) {
  throw "Missing staged executable at '$stageDir/bin/swiftfind-core.exe'."
}

& $InnoCompiler "/DAppVersion=$Version" $issPath $stageDir
if ($LASTEXITCODE -ne 0) {
  throw "Inno Setup compilation failed with exit code $LASTEXITCODE."
}

$setupPath = Join-Path $OutputRoot "swiftfind-$Version-windows-x64-setup.exe"
if (-not (Test-Path $setupPath)) {
  throw "Expected installer was not generated at '$setupPath'."
}

Write-Host "Created installer: $setupPath" -ForegroundColor Green
