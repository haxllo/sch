param(
  [string]$Version,
  [string]$OutputRoot = "artifacts/windows",
  [switch]$Sign,
  [string]$CertPath,
  [string]$CertPassword,
  [string]$TimestampUrl = "http://timestamp.digicert.com",
  [string]$SignTool = "signtool.exe"
)

$ErrorActionPreference = 'Stop'

if (-not $Version -or $Version.Trim().Length -eq 0) {
  try {
    $Version = (git describe --tags --always).Trim()
  }
  catch {
    $Version = "0.0.0-local"
  }
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$artifactName = "swiftfind-$Version-windows-x64"
$stageDir = Join-Path $OutputRoot "$artifactName-stage"
$zipPath = Join-Path $OutputRoot "$artifactName.zip"
$manifestPath = Join-Path $OutputRoot "$artifactName-manifest.json"

Write-Host "== Packaging $artifactName ==" -ForegroundColor Cyan

New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
if (Test-Path $stageDir) { Remove-Item -Recurse -Force $stageDir }
if (Test-Path $zipPath) { Remove-Item -Force $zipPath }
New-Item -ItemType Directory -Force -Path $stageDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stageDir "bin") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stageDir "assets") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stageDir "docs") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stageDir "scripts") | Out-Null

cargo build -p swiftfind-core --release

$coreExe = "target/release/swiftfind-core.exe"
if (-not (Test-Path $coreExe)) {
  throw "Expected core executable not found at $coreExe"
}

if ($Sign) {
  Write-Host "Signing enabled. Signing $coreExe ..." -ForegroundColor Cyan

  if (-not $CertPath -or $CertPath.Trim().Length -eq 0) {
    throw "Signing requested but -CertPath was not provided."
  }
  if (-not (Test-Path $CertPath)) {
    throw "Signing requested but certificate file was not found: $CertPath"
  }

  $signtoolCmd = Get-Command $SignTool -ErrorAction SilentlyContinue
  if (-not $signtoolCmd) {
    throw "Signing requested but signtool was not found. Install Windows SDK SignTool or pass -SignTool with full path."
  }

  $signArgs = @(
    "sign",
    "/fd", "SHA256",
    "/tr", $TimestampUrl,
    "/td", "SHA256",
    "/f", $CertPath
  )
  if ($CertPassword -and $CertPassword.Length -gt 0) {
    $signArgs += @("/p", $CertPassword)
  }
  $signArgs += $coreExe

  & $signtoolCmd.Source @signArgs
  if ($LASTEXITCODE -ne 0) {
    throw "signtool sign failed with exit code $LASTEXITCODE"
  }

  & $signtoolCmd.Source verify /pa /v $coreExe
  if ($LASTEXITCODE -ne 0) {
    throw "signtool verify failed with exit code $LASTEXITCODE"
  }

  $signature = Get-AuthenticodeSignature $coreExe
  if ($signature.Status -ne "Valid") {
    throw "Authenticode signature is not valid: $($signature.Status)"
  }
  Write-Host "Signature verified: $($signature.SignerCertificate.Subject)" -ForegroundColor Green
}
else {
  Write-Host "Signing skipped (unsigned artifact)." -ForegroundColor Yellow
}

Copy-Item $coreExe (Join-Path $stageDir "bin/swiftfind-core.exe") -Force
if (Test-Path "apps/assets/swiftfinder.svg") {
  Copy-Item "apps/assets/swiftfinder.svg" (Join-Path $stageDir "assets/swiftfinder.svg") -Force
}
Copy-Item "docs/engineering/windows-runtime-validation-checklist.md" (Join-Path $stageDir "docs/windows-runtime-validation-checklist.md") -Force
Copy-Item "docs/releases/windows-milestone-release-notes-template.md" (Join-Path $stageDir "docs/release-notes-template.md") -Force
Copy-Item "scripts/windows/install-swiftfind.ps1" (Join-Path $stageDir "scripts/install-swiftfind.ps1") -Force
Copy-Item "scripts/windows/uninstall-swiftfind.ps1" (Join-Path $stageDir "scripts/uninstall-swiftfind.ps1") -Force

$manifest = [ordered]@{
  artifact = $artifactName
  version = $Version
  built_utc = (Get-Date).ToUniversalTime().ToString('o')
  build_stamp = $stamp
  os = "windows-x64"
  signed = [bool]$Sign
  files = @(
    "bin/swiftfind-core.exe",
    "assets/swiftfinder.svg",
    "docs/windows-runtime-validation-checklist.md",
    "docs/release-notes-template.md",
    "scripts/install-swiftfind.ps1",
    "scripts/uninstall-swiftfind.ps1"
  )
}

$manifest | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 $manifestPath
Compress-Archive -Path (Join-Path $stageDir "*") -DestinationPath $zipPath

Write-Host "Created artifact: $zipPath" -ForegroundColor Green
Write-Host "Created manifest: $manifestPath" -ForegroundColor Green
Write-Host "Staging dir retained: $stageDir" -ForegroundColor Green
