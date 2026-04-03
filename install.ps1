# RM1000i Tray - Installer
# Installs dependencies and sets up autostart via registry

$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

Write-Host "Installing dependencies..."
pip install -r "$scriptDir\requirements.txt"

$pythonw = (Get-Command pythonw.exe).Source
$trayScript = "$scriptDir\rm1000i-tray.py"

Write-Host "Setting up autostart..."
Set-ItemProperty `
    -Path "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run" `
    -Name "RM1000iTray" `
    -Value "powershell.exe -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -Command `"Start-Process '$pythonw' -ArgumentList '$trayScript'`""

Write-Host "Starting tray app..."
Start-Process $pythonw -ArgumentList $trayScript

Write-Host "Done! RM1000i Tray is running. Look for the wattage icon in your system tray."
