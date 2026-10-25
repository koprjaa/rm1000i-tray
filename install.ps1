# Project: rm1000i-tray
# File: install.ps1
# Description: Build and register login autostart for the current user.
# Author: Jan Alexandr Kopřiva
# License: AGPL-3.0-or-later
$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $scriptDir
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
    $built = Join-Path $scriptDir "target\release\rm1000i-tray.exe"
    if (-not (Test-Path $built)) { throw "Executable not found: $built" }
    # Run a copy so cargo clean or a rebuild does not touch the running widget.
    $installDir = Join-Path $env:LOCALAPPDATA "Programs\rm1000i-tray"
    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
    Get-Process rm1000i-tray -ErrorAction SilentlyContinue | Stop-Process -Force
    $exe = Join-Path $installDir "rm1000i-tray.exe"
    Copy-Item -Force $built $exe
    $runKey = "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run"
    if (-not (Test-Path $runKey)) { New-Item -Path $runKey | Out-Null }
    Set-ItemProperty -Path $runKey -Name "RM1000iTray" -Value ('"' + $exe + '"')
    Start-Process -FilePath $exe
    Write-Host "Done! RM1000i Tray is running. Look for the wattage icon in your system tray."
} finally {
    Pop-Location
}
