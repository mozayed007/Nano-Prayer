<#
.SYNOPSIS
  Build frontend + Electron main, then run Playwright E2E against the real app.
#>
param(
  [switch]$SkipFrontendBuild
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

if (-not $SkipFrontendBuild) {
  Write-Host "=== Frontend build ==="
  npm run build
  if ($LASTEXITCODE -ne 0) { throw "frontend build failed" }
}

Write-Host "=== Electron main build ==="
npm --prefix electron-app run build
if ($LASTEXITCODE -ne 0) { throw "electron tsc failed" }

Write-Host "=== Ensure Electron binary ==="
$electronExe = Join-Path $repoRoot "electron-app\node_modules\electron\dist\electron.exe"
if (-not (Test-Path $electronExe)) {
  Write-Host "electron.exe missing – attempting extract from local Electron cache..."
  $zip = Get-ChildItem "$env:LOCALAPPDATA\electron\Cache" -Recurse -Filter "electron-v*-win32-x64.zip" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
  if ($zip) {
    $dest = Join-Path $repoRoot "electron-app\node_modules\electron\dist"
    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    Expand-Archive -Path $zip.FullName -DestinationPath $dest -Force
  } else {
    Push-Location (Join-Path $repoRoot "electron-app")
    node node_modules\electron\install.js
    Pop-Location
  }
}
if (-not (Test-Path $electronExe)) {
  throw "electron.exe still missing after install attempt"
}

Write-Host "=== Install e2e deps if needed ==="
Push-Location electron-app
if (-not (Test-Path "node_modules\@playwright\test")) {
  npm install --save-dev @playwright/test serve
}
# Playwright does not need browser download for pure _electron tests, but serve is used
npm run test:e2e
$code = $LASTEXITCODE
Pop-Location
exit $code
