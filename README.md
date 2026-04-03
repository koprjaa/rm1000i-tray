# rm1000i-tray

<img src="screenshot.png" width="320" alt="tray icon showing wattage" />

## What it does

A Windows system tray application that reads live power metrics from a Corsair RM1000i PSU and displays the current total wattage as a numeric tray icon. Hovering over the icon shows a tooltip with per-rail power output (+12V, +5V, +3.3V), estimated efficiency, VRM temperature, and fan speed. The icon turns red when output exceeds 800W. The application reconnects automatically if the USB connection drops and retains the last known reading during the gap.

## Why it was built

Corsair's iCUE software does not expose PSU wattage data in any readable form. The RM1000i communicates over USB HID and supports telemetry queries, but no official tool surfaces this data in the system tray. This project fills that gap by talking to the device directly.

## Architecture

The application has two components running concurrently. The main thread initializes a `pystray` tray icon and enters its event loop. A background daemon thread (`updater`) polls the PSU via `liquidctl` once per second, renders a new icon image using Pillow, and updates the tray. The icon is a 256×256 RGBA image rendered at high resolution so Windows can scale it cleanly to the 16×16 tray slot. State shared between the threads is limited to the `pystray.Icon` object, which handles its own thread safety.

## Key decisions

**liquidctl over raw HID or Corsair SDK**: liquidctl already implements the Corsair HID PSU protocol and supports the RM1000i. Writing a raw HID driver would duplicate existing work without adding capability.

**Pillow-rendered icon over a static image set**: Wattage is a continuous value. Rendering text dynamically at runtime avoids maintaining a separate image asset for every possible number.

**Persistent connection with reconnect on error**: Opening and closing the USB device on every poll caused intermittent device lock errors. Holding the connection open and reconnecting only on failure is more stable.

**Registry Run key for autostart**: The installer targets the current user's `HKCU` Run key, which does not require elevated privileges. Task Scheduler with SYSTEM-level execution was attempted but failed silently due to path encoding issues with non-ASCII characters in the user profile path.

## Trade-offs

The tooltip label strings (`Prikon`, `Ucinnost`) are Czech rather than English. This is a side effect of incremental development and has no functional impact, but it is inconsistent with the English codebase.

The icon font path is hardcoded to `C:/Windows/Fonts/arialbd.ttf`. If that file is absent, Pillow falls back to its built-in bitmap font, which renders at a much smaller size. No other font paths are tried.

The 800W threshold for the red icon color is hardcoded with no configuration mechanism.

## Limitations

- Only the first detected RM1000i device is used. Multiple PSUs are not supported.
- The installer resolves `pythonw.exe` at install time via `Get-Command`. If Python is later updated or moved, the registry entry will point to a stale path.
- The autostart registry value embeds the absolute path to the script. Moving the project directory after installation requires re-running the installer.
- No logging is written at runtime. Errors are silently suppressed and surfaced only as a stale tray reading.

## How to run

Requires Python 3.10+ and a Corsair RM1000i connected via USB. Close iCUE before running — it holds an exclusive lock on the device.

```powershell
git clone https://github.com/koprjaa/rm1000i-tray.git
cd rm1000i-tray
powershell -ExecutionPolicy Bypass -File install.ps1
```

To run without installing autostart:

```bash
pythonw rm1000i-tray.py
```
