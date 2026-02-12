param(
  [switch]$SkipBuild,
  [switch]$StartAfterInstall = $true,
  [string]$InstallRoot = "$env:LOCALAPPDATA\Programs\SwiftFind"
)

$ErrorActionPreference = "Stop"

Write-Host "== SwiftFind Install ==" -ForegroundColor Cyan
Write-Host "Install root: $InstallRoot"

if (-not $SkipBuild) {
  Write-Host "[1/5] Building release binary..."
  cargo build -p swiftfind-core --release
}

$exeSource = "target/release/swiftfind-core.exe"
if (-not (Test-Path $exeSource)) {
  throw "Missing binary: $exeSource"
}

$binDir = Join-Path $InstallRoot "bin"
$assetsDir = Join-Path $InstallRoot "assets"
$docsDir = Join-Path $InstallRoot "docs"

Write-Host "[2/5] Preparing install directories..."
New-Item -ItemType Directory -Force -Path $binDir | Out-Null
New-Item -ItemType Directory -Force -Path $assetsDir | Out-Null
New-Item -ItemType Directory -Force -Path $docsDir | Out-Null

Write-Host "[3/5] Copying runtime files..."
Copy-Item $exeSource (Join-Path $binDir "swiftfind-core.exe") -Force
if (Test-Path "apps/assets/swiftfinder.svg") {
  Copy-Item "apps/assets/swiftfinder.svg" (Join-Path $assetsDir "swiftfinder.svg") -Force
}
if (Test-Path "docs/engineering/windows-operator-runbook.md") {
  Copy-Item "docs/engineering/windows-operator-runbook.md" (Join-Path $docsDir "windows-operator-runbook.md") -Force
}

$installedExe = Join-Path $binDir "swiftfind-core.exe"

Write-Host "[4/5] Preparing config and startup sync..."
& $installedExe --ensure-config
& $installedExe --sync-startup

if ($StartAfterInstall) {
  Write-Host "[5/5] Starting SwiftFind in background..."
  Start-Process -FilePath $installedExe -ArgumentList "--background" -WindowStyle Hidden
}
else {
  Write-Host "[5/5] Start skipped (--StartAfterInstall:\$false)."
}

Write-Host "Install complete." -ForegroundColor Green
Write-Host "Executable: $installedExe"
Write-Host "Config: $env:APPDATA\SwiftFind\config.json"
