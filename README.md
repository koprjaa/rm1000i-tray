# rm1000i-tray

Windows system tray app that shows live power consumption from a **Corsair RM1000i** PSU.

iCUE doesn't expose wattage data — this tool reads it directly via USB HID using [liquidctl](https://github.com/liquidctl/liquidctl).

![tray icon showing wattage](https://i.imgur.com/placeholder.png)

## Features

- Live wattage in the system tray, updates every second
- Hover tooltip with full stats:
  - Total power draw (W)
  - Efficiency (%)
  - Per-rail power: +12V, +5V, +3.3V
  - VRM temperature
  - Fan speed
- Red icon when power exceeds 800W
- Keeps last known value on read errors (auto-reconnects)
- Autostart on login

## Requirements

- Windows 10/11
- Python 3.10+
- Corsair RM1000i (or other RMi/HXi series PSU)

## Install

```powershell
git clone https://github.com/koprjaa/rm1000i-tray.git
cd rm1000i-tray
powershell -ExecutionPolicy Bypass -File install.ps1
```

> **Note:** iCUE must be closed or the USB device will be locked.

## Manual run

```bash
pythonw rm1000i-tray.py
```

## Dependencies

- [liquidctl](https://github.com/liquidctl/liquidctl) — PSU communication
- [pystray](https://github.com/moses-palmer/pystray) — system tray
- [Pillow](https://python-pillow.org/) — icon rendering
