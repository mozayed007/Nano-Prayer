<#
.SYNOPSIS
  Build Electron main and open an inspectable session for Chrome DevTools flame profiling.

.EXAMPLE
  pwsh -File scripts/profile-electron.ps1
  # Chrome → chrome://inspect → Open dedicated DevTools for Node
#>
param(
  [int]$InspectPort = 9229,
  [int]$Seconds = 0,
  [string]$OutDir = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $OutDir) { $OutDir = Join-Path $repoRoot "profiling-output" }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$readme = Join-Path $OutDir "electron-profile-$stamp.txt"

$electronDir = Join-Path $repoRoot "electron-app"
$electronExe = Join-Path $electronDir "node_modules\electron\dist\electron.exe"

Write-Host "=== Building electron main ==="
Push-Location $electronDir
npm run build
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "electron build failed" }
Pop-Location

if (-not (Test-Path $electronExe)) {
  throw "Missing $electronExe – run npm install in electron-app"
}

$env:ELECTRON_RENDERER_URL = if ($env:ELECTRON_RENDERER_URL) { $env:ELECTRON_RENDERER_URL } else { "http://127.0.0.1:4173" }

Write-Host "Starting Electron with --inspect=$InspectPort"
$args = @("--inspect=$InspectPort", ".")
$p = Start-Process -FilePath $electronExe -ArgumentList $args -WorkingDirectory $electronDir -PassThru -WindowStyle Normal

@"
Electron main process inspect
PID: $($p.Id)
Port: $InspectPort

Flame / CPU profile:
1. Open Chrome or Edge
2. Navigate to chrome://inspect
3. Under "Remote Target", click "inspect" on the electron main process
4. Performance tab → Record → leave app idle tray / trigger settings preview audio → Stop
5. Save profile (Save profile...) into:
   $OutDir

Optional heap:
   Memory tab → Take heap snapshot

Auto-stop: $(if ($Seconds -gt 0) { "$Seconds seconds" } else { "manual (close the app)" })
"@ | Set-Content $readme

Write-Host (Get-Content $readme -Raw)

if ($Seconds -gt 0) {
  Start-Sleep -Seconds $Seconds
  if (-not $p.HasExited) {
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    Get-Process -Name "electron" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
  }
  Write-Host "Stopped after ${Seconds}s"
}

exit 0
