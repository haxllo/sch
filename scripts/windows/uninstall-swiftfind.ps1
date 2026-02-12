param(
  [switch]$PurgeUserData,
  [string]$InstallRoot = "$env:LOCALAPPDATA\Programs\SwiftFind"
)

$ErrorActionPreference = "Continue"

Write-Host "== SwiftFind Uninstall ==" -ForegroundColor Cyan
Write-Host "Install root: $InstallRoot"

$installedExe = Join-Path $InstallRoot "bin\swiftfind-core.exe"

if (Test-Path $installedExe) {
  Write-Host "[1/4] Signaling running instance to quit..."
  & $installedExe --quit | Out-Null
  Start-Sleep -Milliseconds 800
}
else {
  Write-Host "[1/4] Installed executable not found; skipping quit signal."
}

Write-Host "[2/4] Removing startup registration..."
reg delete "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v SwiftFind /f | Out-Null

Write-Host "[3/4] Removing installed files..."
if (Test-Path $InstallRoot) {
  Remove-Item -Recurse -Force $InstallRoot
}

if ($PurgeUserData) {
  Write-Host "[4/4] Removing user data (%APPDATA%\\SwiftFind)..."
  $userData = Join-Path $env:APPDATA "SwiftFind"
  if (Test-Path $userData) {
    Remove-Item -Recurse -Force $userData
  }
}
else {
  Write-Host "[4/4] Keeping user data. Use -PurgeUserData to remove."
}

Write-Host "Uninstall complete." -ForegroundColor Green
