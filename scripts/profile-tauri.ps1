<#
.SYNOPSIS
  Build release Tauri binary and capture a CPU profile (samply or WPR fallback).

.DESCRIPTION
  Rigorous profiling path for nano-pray-reminder:
  1. cargo build --release
  2. Prefer `samply record` (Firefox Profiler JSON) if samply is installed
  3. Else emit WPR instructions / try wpr if available

.EXAMPLE
  pwsh -File scripts/profile-tauri.ps1 -Seconds 60
  # open the .json.gz in https://profiler.firefox.com
#>
param(
  [int]$Seconds = 60,
  [string]$OutDir = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $OutDir) { $OutDir = Join-Path $repoRoot "profiling-output" }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"

$env:PATH = @(
  "$env:USERPROFILE\.cargo\bin",
  "G:\Relocated\Users\MoZayed\.cargo\bin",
  "C:\Users\MoZayed\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin",
  $env:PATH
) -join ";"

Write-Host "=== Building release nano-pray-reminder ==="
Push-Location $repoRoot
cargo build -p nano-pray-reminder --release 2>&1 | Tee-Object (Join-Path $OutDir "tauri-build-$stamp.log")
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "cargo build failed" }
Pop-Location

$exe = Join-Path $repoRoot "target\release\nano-pray-reminder.exe"
if (-not (Test-Path $exe)) { throw "Missing $exe" }

$samply = Get-Command samply -ErrorAction SilentlyContinue
$profileOut = Join-Path $OutDir "tauri-samply-$stamp.json.gz"
$readme = Join-Path $OutDir "tauri-profile-$stamp.txt"

if ($samply) {
  Write-Host "=== Recording with samply for ${Seconds}s (save-only) ==="
  # -d limits duration; -s skips local server; -- separates the app path
  & $samply.Source record -s -d $Seconds -o $profileOut -- $exe
  $code = $LASTEXITCODE
  Get-Process -Name "nano-pray-reminder" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
  if (-not (Test-Path $profileOut)) {
    Write-Warning "samply finished without output file (exit $code). Check Windows ETW permissions."
  }
  @"
Tauri profile complete (samply).
File: $profileOut
Exists: $(Test-Path $profileOut)
Open: https://profiler.firefox.com  → Load profile
Duration: ${Seconds}s
Exit: $code
"@ | Set-Content $readme
  Write-Host (Get-Content $readme -Raw)
  exit $code
}

# Fallback: launch + WPR guidance
Write-Host "samply not found. Install: cargo install samply"
Write-Host "Fallback: launching release binary for ${Seconds}s while you can attach WPR..."

$log = Join-Path $OutDir "tauri-run-$stamp.log"
$p = Start-Process -FilePath $exe -PassThru -WindowStyle Minimized -RedirectStandardOutput $log -RedirectStandardError (Join-Path $OutDir "tauri-run-$stamp.err.log")
Start-Sleep -Seconds $Seconds
if (-not $p.HasExited) {
  Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
}

$wpr = Get-Command wpr -ErrorAction SilentlyContinue
@"
Tauri release binary exercised for ${Seconds}s.
Binary: $exe
Console: $log

Flamegraph options on Windows:
1) Recommended: cargo install samply
   Then re-run: pwsh -File scripts/profile-tauri.ps1 -Seconds 90

2) Windows Performance Recorder (admin):
   wpr -start CPU
   # launch $exe minimized for ~1 min
   wpr -stop $(Join-Path $OutDir "tauri-wpr-$stamp.etl")
   Open .etl in Windows Performance Analyzer (WPA)

3) cargo-flamegraph is Linux/macOS-oriented; prefer samply on Windows.
"@ | Set-Content $readme

if ($wpr) {
  Write-Host "wpr is available. Example:"
  Write-Host "  wpr -start CPU"
  Write-Host "  # run app"
  Write-Host "  wpr -stop $(Join-Path $OutDir "tauri-wpr-$stamp.etl")"
}

Write-Host (Get-Content $readme -Raw)
exit 0
