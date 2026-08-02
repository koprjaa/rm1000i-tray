# rm1000i-tray

Windows system tray widget that shows the live power draw of a Corsair RM1000i power supply. iCUE does not expose this number.

![python](https://img.shields.io/badge/python-3.10+-3776AB?style=flat-square&logo=python&logoColor=white)
![license](https://img.shields.io/badge/license-MIT-A31F34?style=flat-square)
![status](https://img.shields.io/badge/status-active-22863A?style=flat-square)
![platform](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6?style=flat-square&logo=windows&logoColor=white)

<img src="screenshot.png" width="320" alt="Tray icon showing wattage" />

The widget reads wattage, rail voltages, efficiency, VRM temperature, and fan speed from the power supply over USB HID with [liquidctl](https://github.com/liquidctl/liquidctl). It updates the icon every second. The icon turns red above 800 W, which helps when you tune a GPU undervolt or look for a runaway transient.

The tooltip shows total power draw, efficiency, power on the +12 V, +5 V, and +3.3 V rails, VRM temperature, and fan speed.

## Install

```powershell
git clone https://github.com/koprjaa/rm1000i-tray.git
cd rm1000i-tray
powershell -ExecutionPolicy Bypass -File install.ps1
```

The installer adds the widget to autostart.

Close iCUE before you start the widget. iCUE locks the USB HID device, and this tool cannot open it while iCUE runs.

## Use

```bash
pythonw rm1000i-tray.py
```

Use `pythonw` instead of `python` so that no console window appears.

## Hardware

The widget works with any Corsair RMi or HXi power supply that liquidctl supports. This covers the RM650i, RM750i, RM850i, RM1000i, HX750i, HX850i, HX1000i, and HX1200i. Daily testing runs on an RM1000i.

## How it works

A failed USB read keeps the last known value on the icon instead of showing an error. This hides transient read errors. A full disconnect starts a reconnect loop.

## Limits

- Windows only. The tray integration uses pystray.
- iCUE and this widget cannot run at the same time.
- No tests.

## License

[MIT](LICENSE)
