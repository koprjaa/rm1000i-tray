# rm1000i-tray

Displays Corsair RM1000i power readings in the Windows system tray.

[![CI](https://github.com/koprjaa/rm1000i-tray/actions/workflows/ci.yml/badge.svg)](https://github.com/koprjaa/rm1000i-tray/actions/workflows/ci.yml)
![license](https://img.shields.io/badge/license-AGPL--3.0-A31F34?style=flat-square)

<img src="screenshot.png" width="320" alt="Tray icon showing wattage" />

Protocol references include [liquidctl](https://github.com/liquidctl/liquidctl), by Jonas Malaco and contributors, whose Corsair driver credits corsaiRMi by notaz and realies and reference work by Sean Nelson. The bundled [DejaVu font](https://dejavu-fonts.github.io/) credits Stepan Roh and retains the Bitstream copyright and license in `assets/FONT-LICENSE`.

## What it does

Reads the PSU over USB HID. The icon turns red above 800 W. Hover for rail power, VRM temperature, fan speed and estimated efficiency. Input power and efficiency are estimates, not AC measurements. It does not change fan or OCP settings or initialize the PSU.

## Install

On Windows, install stable Rust and its Windows build tools. From the repository directory:

```powershell
cargo install --path .
```

Close iCUE and other PSU polling tools before use. Only the RM1000i is selected.

For startup at login, run from the repository directory:

```powershell
powershell -ExecutionPolicy Bypass -File install.ps1
```

The script builds, copies the executable to `%LOCALAPPDATA%\Programs\rm1000i-tray`, registers autostart and launches it. Remove `RM1000iTray` from `HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run` to disable autostart.

## Use

```powershell
rm1000i-tray
rm1000i-tray --once
```

Tray mode waits 10 seconds before appearing, then polls with a one-second pause between attempts. Right-click and choose Quit to exit. Failed reads keep the last reading while reconnecting.

`--once` prints readings with units and exits. Linux supports this diagnostic mode only. Use `--help` for usage. After boot or resume, the PSU may need initialization by another tool before reads succeed.

## License

[AGPL-3.0-or-later](LICENSE). The bundled fallback font retains its own license.
