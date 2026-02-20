param(
  [switch]$KeepDebug
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$entries = @(
  "deps",
  "build",
  ".fingerprint",
  "examples",
  "incremental",
  "nano_pray_reminder.pdb",
  "nano-pray-reminder.pdb"
)

$profiles = @("release")
if (-not $KeepDebug) {
  $profiles += "debug"
}

$removed = 0
foreach ($profile in $profiles) {
  foreach ($entry in $entries) {
    $path = Join-Path (Join-Path "target" $profile) $entry
    if (Test-Path $path) {
      Remove-Item -Recurse -Force $path
      $removed++
    }
  }
}

$scope = if ($KeepDebug) { "release only" } else { "release + debug" }
Write-Output "Prune complete ($scope). Removed $removed entries from target."
