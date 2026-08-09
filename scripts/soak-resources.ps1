<#
.SYNOPSIS
  Sample CPU% and WorkingSet for NanoPrayer processes over a soak window.

.DESCRIPTION
  Legitimate resource soak: attaches to running Electron and/or Tauri processes
  (or launches them), samples every -IntervalSec, writes CSV under -OutDir.

.PARAMETER DurationMin
  How long to sample (default 15 minutes; use 240+ for full soak).

.PARAMETER IntervalSec
  Sample period (default 30).

.PARAMETER Launch
  If set, start Electron and/or Tauri when not already running.

.PARAMETER Target
  electron | tauri | both (default both)

.EXAMPLE
  pwsh -File scripts/soak-resources.ps1 -DurationMin 15 -Target electron -Launch
#>
param(
  [double]$DurationMin = 15,
  [int]$IntervalSec = 30,
  [ValidateSet("electron", "tauri", "both")]
  [string]$Target = "both",
  [switch]$Launch,
  [string]$OutDir = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $OutDir) {
  $OutDir = Join-Path $repoRoot "profiling-output"
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$csvPath = Join-Path $OutDir "soak-$stamp.csv"

function Get-NanoProcesses {
  param([string]$Which)
  $list = @()
  if ($Which -eq "electron" -or $Which -eq "both") {
    $list += Get-Process -Name "electron","NanoPrayReminder-Electron*" -ErrorAction SilentlyContinue
  }
  if ($Which -eq "tauri" -or $Which -eq "both") {
    $list += Get-Process -Name "nano-pray-reminder","NanoPrayReminder" -ErrorAction SilentlyContinue
  }
  $list | Where-Object { $_ } | Sort-Object Id -Unique
}

function Start-TargetsIfNeeded {
  param([string]$Which)
  $running = Get-NanoProcesses -Which $Which
  if ($running) { return }

  if ($Which -eq "electron" -or $Which -eq "both") {
    $mainJs = Join-Path $repoRoot "electron-app\dist\main.js"
    $electronExe = Join-Path $repoRoot "electron-app\node_modules\electron\dist\electron.exe"
    if (-not (Test-Path $mainJs)) {
      Write-Host "Building electron main..."
      Push-Location (Join-Path $repoRoot "electron-app"); npm run build; Pop-Location
    }
    if (Test-Path $electronExe) {
      Write-Host "Launching Electron..."
      $env:ELECTRON_RENDERER_URL = "http://127.0.0.1:4173"
      Start-Process -FilePath $electronExe -ArgumentList "." -WorkingDirectory (Join-Path $repoRoot "electron-app") -WindowStyle Minimized
      Start-Sleep -Seconds 4
    } else {
      Write-Warning "electron.exe not found; install electron-app deps first"
    }
  }

  if ($Which -eq "tauri" -or $Which -eq "both") {
    $tauri = Join-Path $repoRoot "target\release\nano-pray-reminder.exe"
    if (-not (Test-Path $tauri)) {
      $tauri = Join-Path $repoRoot "target\debug\nano-pray-reminder.exe"
    }
    if (Test-Path $tauri) {
      Write-Host "Launching Tauri: $tauri"
      Start-Process -FilePath $tauri -WindowStyle Minimized
      Start-Sleep -Seconds 4
    } else {
      Write-Warning "Tauri binary not found. Build with: cargo build -p nano-pray-reminder --release"
    }
  }
}

if ($Launch) {
  Start-TargetsIfNeeded -Which $Target
}

$end = (Get-Date).AddMinutes($DurationMin)
"timestamp,process,pid,cpu_seconds_total,working_set_mb,private_mb,handles" | Set-Content -Path $csvPath -Encoding utf8
Write-Host "Soak writing to $csvPath until $end (interval ${IntervalSec}s)"

# CPU% approximation via delta ProcessorTime
$prevCpu = @{}

while ((Get-Date) -lt $end) {
  $procs = Get-NanoProcesses -Which $Target
  if (-not $procs) {
    Write-Warning "No matching processes; waiting..."
  }
  foreach ($p in $procs) {
    try {
      $p.Refresh()
      $cpu = $p.TotalProcessorTime.TotalSeconds
      $prev = $prevCpu[$p.Id]
      $deltaCpu = if ($null -ne $prev) { [math]::Round($cpu - $prev, 3) } else { 0 }
      $prevCpu[$p.Id] = $cpu
      $ws = [math]::Round($p.WorkingSet64 / 1MB, 2)
      $priv = [math]::Round($p.PrivateMemorySize64 / 1MB, 2)
      $line = "{0},{1},{2},{3},{4},{5},{6}" -f `
        (Get-Date -Format "o"), $p.ProcessName, $p.Id, $deltaCpu, $ws, $priv, $p.HandleCount
      Add-Content -Path $csvPath -Value $line
      Write-Host $line
    } catch {
      Write-Warning "Sample failed for pid $($p.Id): $_"
    }
  }
  Start-Sleep -Seconds $IntervalSec
}

# Summary
Write-Host "`n=== Soak summary ==="
$rows = Import-Csv $csvPath
if ($rows) {
  $byProc = $rows | Group-Object process
  foreach ($g in $byProc) {
    $ws = $g.Group | ForEach-Object { [double]$_.working_set_mb }
    $cpu = $g.Group | ForEach-Object { [double]$_.cpu_seconds_total }
    "Process=$($g.Name) samples=$($g.Count) ws_min=$([math]::Round(($ws | Measure-Object -Minimum).Minimum,2)) ws_max=$([math]::Round(($ws | Measure-Object -Maximum).Maximum,2)) ws_avg=$([math]::Round(($ws | Measure-Object -Average).Average,2)) cpu_delta_sum=$([math]::Round(($cpu | Measure-Object -Sum).Sum,2))"
  }
}
Write-Host "CSV: $csvPath"
# Exit 0 always – analysis is offline; CI can gate on ws_max growth later
exit 0
