# Project: rm1000i-tray
# File: install.ps1
# Description: Build and register login autostart for the current user.
# Author: Jan Alexandr Kopřiva
# License: MIT
$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $scriptDir
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
    $exe = Join-Path $scriptDir "target\release\rm1000i-tray.exe"
    if (-not (Test-Path $exe)) { throw "Executable not found: $exe" }
    $runKey = "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run"
    if (-not (Test-Path $runKey)) { New-Item -Path $runKey | Out-Null }
    Set-ItemProperty -Path $runKey -Name "RM1000iTray" -Value ('"' + $exe + '"')
    Start-Process -FilePath $exe
    Write-Host "Done! RM1000i Tray is running. Look for the wattage icon in your system tray."
} finally {
    Pop-Location
}
