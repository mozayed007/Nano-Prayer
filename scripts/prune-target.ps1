param(
  [switch]$KeepDebug,
  [switch]$DryRun
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
    $resolved = Resolve-Path $path -ErrorAction SilentlyContinue
    if ($resolved) {
      $workspace = (Resolve-Path ".").Path
      $targetRoot = Join-Path $workspace "target"
      $resolvedPath = $resolved.Path
      $targetRootWithSeparator = $targetRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
      if (-not $resolvedPath.StartsWith($targetRootWithSeparator)) {
        throw "Refusing to prune outside workspace target folder: $resolvedPath"
      }
      if ($DryRun) {
        Write-Output "Would remove $resolvedPath"
      } else {
        Remove-Item -LiteralPath $resolvedPath -Recurse -Force
      }
      $removed++
    }
  }
}

$scope = if ($KeepDebug) { "release only" } else { "release + debug" }
$mode = if ($DryRun) { "dry run" } else { "pruned" }
Write-Output "Prune complete ($scope, $mode). Matched $removed entries from target."
