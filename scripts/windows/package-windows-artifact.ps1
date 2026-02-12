param(
  [string]$Version,
  [string]$OutputRoot = "artifacts/windows"
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
New-Item -ItemType Directory -Force -Path (Join-Path $stageDir "docs") | Out-Null

cargo build -p swiftfind-core --release

$coreExe = "target/release/swiftfind-core.exe"
if (-not (Test-Path $coreExe)) {
  throw "Expected core executable not found at $coreExe"
}

Copy-Item $coreExe (Join-Path $stageDir "bin/swiftfind-core.exe") -Force
Copy-Item "docs/engineering/windows-runtime-validation-checklist.md" (Join-Path $stageDir "docs/windows-runtime-validation-checklist.md") -Force
Copy-Item "docs/releases/windows-milestone-release-notes-template.md" (Join-Path $stageDir "docs/release-notes-template.md") -Force

$manifest = [ordered]@{
  artifact = $artifactName
  version = $Version
  built_utc = (Get-Date).ToUniversalTime().ToString('o')
  build_stamp = $stamp
  os = "windows-x64"
  files = @(
    "bin/swiftfind-core.exe",
    "docs/windows-runtime-validation-checklist.md",
    "docs/release-notes-template.md"
  )
}

$manifest | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 $manifestPath
Compress-Archive -Path (Join-Path $stageDir "*") -DestinationPath $zipPath

Write-Host "Created artifact: $zipPath" -ForegroundColor Green
Write-Host "Created manifest: $manifestPath" -ForegroundColor Green
Write-Host "Staging dir retained: $stageDir" -ForegroundColor Green
