param(
  [switch]$SkipUi = $false,
  [switch]$SkipRust = $false
)

$ErrorActionPreference = 'Stop'

Write-Host '== Sprint 4 Windows Validation Preflight ==' -ForegroundColor Cyan

if (-not $SkipRust) {
  $env:SWIFTFIND_WINDOWS_RUNTIME_SMOKE = '1'
  Write-Host '[1/3] Running Windows runtime smoke harness...' -ForegroundColor Yellow
  cargo test -p swiftfind-core --test windows_runtime_smoke_test -- --exact windows_runtime_smoke_registers_hotkey_and_transport_roundtrip
}

if (-not $SkipUi) {
  Write-Host '[2/3] Running UI launcher flow tests...' -ForegroundColor Yellow
  ./node_modules/.bin/vitest --run tests/e2e/hotkey-search-launch.spec.ts apps/ui/src/components/LauncherOverlay.test.tsx
}

Write-Host '[3/3] Checklist location:' -ForegroundColor Yellow
Write-Host 'docs/engineering/windows-runtime-validation-checklist.md' -ForegroundColor Green
Write-Host 'Run scripts/windows/record-manual-e2e.ps1 to capture manual pass/fail evidence.' -ForegroundColor Green
