param(
  [string]$OutputPath = "artifacts/windows/manual-e2e-result.json"
)

$ErrorActionPreference = 'Stop'

$questions = @(
  @{ key = 'hotkey_opens_launcher'; prompt = 'Press Alt+Space: launcher appears with focused query box? (y/n)' },
  @{ key = 'query_returns_results'; prompt = 'Type query: results update with indexed items? (y/n)' },
  @{ key = 'keyboard_navigation'; prompt = 'Arrow keys change selected result? (y/n)' },
  @{ key = 'enter_launches_selected'; prompt = 'Press Enter: selected item launch attempted? (y/n)' },
  @{ key = 'launch_error_visible'; prompt = 'Invalid launch path shows visible error in launcher UI? (y/n)' }
)

$result = [ordered]@{
  timestamp_utc = (Get-Date).ToUniversalTime().ToString('o')
  machine = $env:COMPUTERNAME
  user = $env:USERNAME
  checks = @{}
}

foreach ($q in $questions) {
  do {
    $answer = Read-Host $q.prompt
  } while ($answer -notin @('y', 'n', 'Y', 'N'))

  $result.checks[$q.key] = ($answer -in @('y', 'Y'))
}

$allPassed = ($result.checks.Values | Where-Object { -not $_ }).Count -eq 0
$result['all_passed'] = $allPassed

$parent = Split-Path -Parent $OutputPath
if ($parent) {
  New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

$result | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 $OutputPath
Write-Host "Saved manual E2E result: $OutputPath" -ForegroundColor Green

if (-not $allPassed) {
  Write-Host 'One or more manual checks failed. Review launcher/runtime integration before release.' -ForegroundColor Red
  exit 1
}
