# rm1000i-tray

Windows system tray widget that shows the live power output of a Corsair RM1000i power supply. iCUE does not expose this number.

![rust](https://img.shields.io/badge/rust-stable-DEA584?style=flat-square&logo=rust&logoColor=white)
![license](https://img.shields.io/badge/license-MIT-A31F34?style=flat-square)
![status](https://img.shields.io/badge/status-active-22863A?style=flat-square)
[![ci](https://github.com/koprjaa/rm1000i-tray/actions/workflows/ci.yml/badge.svg)](https://github.com/koprjaa/rm1000i-tray/actions/workflows/ci.yml)

<img src="screenshot.png" width="320" alt="Tray icon showing wattage" />

The widget reads the PSU over USB HID every second. Its icon turns red above 800 W. The tooltip shows total output power, estimated efficiency, power on all three rails, VRM temperature, and fan speed, limited to 127 characters.

## Origin

Based on [rm1000i-tray](https://github.com/koprjaa/rm1000i-tray) by Jan Alexandr Kopřiva. The original MIT copyright is preserved in LICENSE. Protocol references include [liquidctl](https://github.com/liquidctl/liquidctl), by Jonas Malaco and contributors, whose Corsair driver credits corsaiRMi by notaz and realies and reference work by Sean Nelson. The bundled [DejaVu font](https://dejavu-fonts.github.io/) credits Stepan Roh and retains the Bitstream copyright and license in `assets/FONT-LICENSE`.

## Install

Install stable Rust with its Windows build tools, then run:

```powershell
cargo install --path .
rm1000i-tray
```

For automatic startup at login, run this from the project directory:

```powershell
powershell -ExecutionPolicy Bypass -File install.ps1
```

The script runs `cargo build --release`, registers the quoted executable path in the current user's registry Run key, and starts it. Keep `target/release/rm1000i-tray.exe` at that location. To uninstall autostart, remove the `RM1000iTray` value from `HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run`.

Close iCUE before starting the widget: competing HID clients can prevent access or corrupt responses.

## Use

```powershell
rm1000i-tray
rm1000i-tray --once
```

Tray mode waits 10 seconds before registering its loading icon. Right-click the icon and select **Quit** to exit. Release builds have no console window.

`--once` skips the tray and startup delay, prints every supported metric as `key: value unit`, and exits. It returns 1 if no RM1000i is available or reading fails, and 2 for invalid arguments. On Windows, use PowerShell's `Start-Process -Wait` if the shell does not wait for a GUI executable; stdout can also be redirected for comparison with `liquidctl status`.

## Hardware and safety

Discovery selects the first USB device with Corsair VID `0x1b1c` and RM1000i PID `0x1c0d`. Other RMi/HXi models are not selected. The original widget's `RM1000i` match also restricted actual discovery despite the broader hardware list in its documentation.

The protocol is implemented directly using `hidapi`. The only outgoing commands are reads of input voltage, rail voltage/current/power, temperatures, fan speed, total output power, and PAGE selection (0–2). An explicit whitelist rejects unsupported packets with an error before each write. No OCP, fan, configuration, or initialization commands are sent.

The [liquidctl Corsair driver](https://github.com/liquidctl/liquidctl/blob/main/liquidctl/driver/corsair_hid_psu.py) and [PMBus decoder](https://github.com/liquidctl/liquidctl/blob/main/liquidctl/pmbus.py) were used as protocol references. Connecting only opens a HID handle. Its separate `initialize` method sends a `fe 03` handshake, writes OCP mode, and restores hardware fan control. This app omits that entire sequence, as did the original widget. A freshly booted or resumed PSU may therefore reject reads until initialized outside this app. Invalid/uninitialized replies cause a reconnect, never an automatic configuration change.

Estimated input power uses liquidctl's RM1000i quadratic curves at 115/230 V, interpolates by measured input voltage, and rounds to the nearest even integer. Estimated efficiency is output divided by that rounded estimate, rounded again. These are estimates, not measured AC input power.

## How it works

A failed read drops the handle and retries discovery on the next tick. The icon and tooltip keep the last complete reading. Each HID response has a one-second timeout; the one-second polling interval follows each read attempt, so slow or failed reads can lengthen the update period. Shutdown does not wait for the polling worker.

Icons are 256×256 RGBA, transparent, with centered Arial Bold from `C:/Windows/Fonts/arialbd.ttf`, at em size 155 for at most three characters and 120 otherwise. A bundled DejaVu Sans Bold font is used if Arial cannot be loaded; its license is in `assets/FONT-LICENSE`.

## Changes from the Python version

- Both versions check reply headers. Rust skips mismatched or short reports until a matching full report arrives or the one-second deadline expires; liquidctl 1.16 raises immediately for a mismatched header. Neither version can distinguish another client's matching command or prevent races on rail PAGE selection. Exclusive access remains necessary for reliable readings.
- A compiled Rust executable replaces the interpreter and runtime packages. Release tray mode needs no console; debug builds retain one. Installation and registry autostart point directly to the executable rather than a PowerShell/Python launcher. The installer checks Cargo failures and creates a missing Run key.
- Added `--once`, `--help`/`-h`, explicit argument validation and exit codes. Python ignored all CLI arguments and always started its tray. Invalid Unicode arguments are rejected without a panic. No files or stdin are parsed; BOM/CRLF are literal argument characters. On Linux only diagnostics are available; tray mode reports an error immediately.
- Direct HID replaces liquidctl; Linux diagnostics use HID rather than liquidctl's optional hwmon path, and may require device permissions or release of the kernel driver. Only the original RM1000i hardware match is implemented, using exact USB IDs instead of a description substring.
- Status contains 16 numeric metrics. It omits liquidctl's current/total uptime, fan control mode, and OCP mode because the widget never displayed them. Failures of those unused commands therefore no longer prevent an update. Voltage/current on all rails, case temperature, input voltage, and estimated input power are available through `--once`.
- Status preserves liquidctl's integer versus float representation, including integer fan RPM and floats with `.0`. Float formatting matches Python for every LINEAR11 value and the tested scientific-notation boundaries, including negative zero and signed, padded exponents. The Rust library accepts numeric status values only; Python accepted arbitrary dictionary values (strings, None, booleans and unbounded integers). Non-finite wattage is shown as `??` instead of crashing the update thread during integer conversion.
- Font rasterization uses `ab_glyph`, so antialiasing and pixel bounds can differ slightly from Pillow. The fallback is bundled DejaVu Sans Bold at the requested size instead of Pillow's small default font.
- HID transactions reject short writes and timeouts; short reports and mismatched echoes are skipped within the deadline. A zero-byte timeout ends the attempt immediately without busy polling. Forbidden library packet requests return an error rather than panicking. Nonpositive estimated input power is rejected; Python only fails at zero division and may display negative estimates from invalid sensor data. Queue draining is bounded to 65 reads. Read/connection failures still preserve the previous display and reconnect. The app does not initialize hardware or change configuration, matching the original widget's omission of `initialize`.
- Tray creation failures produce an error and exit status 1 instead of silently succeeding. Python lets tray setup exceptions escape with a traceback. Driver errors in diagnostics use plain English messages and exit 1; background read failures remain silent.
- Menu events are checked at most 100 ms apart. USB work runs on a background thread and display updates run on the Windows event loop. Handles use Rust ownership and errors use Result values rather than Python disconnect calls and caught exceptions. The visible name of the executable and its tray implementation change; the loading label, tooltip fields, thresholds, startup delay and polling interval remain the same.

## Development

```sh
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo check --target x86_64-pc-windows-gnu
```

The last command on Linux needs `rustup target add x86_64-pc-windows-gnu` and a MinGW GCC toolchain. CI checks Linux and Windows. Tests cover the original formatting, icon and reconnect scenarios, LINEAR11 signed decoding, efficiency estimates, exhaustive opcode whitelist checks, mocked HID failures and the CLI. Oracle fixtures were generated by the original Python functions and liquidctl 1.16.0: all 65,536 LINEAR11 bit patterns, 33,806 formatting/label rows, and 63 input-power estimates. No PSU is needed; real hardware readings and the Windows tray still require manual validation.

Dependencies: `hidapi` provides USB access (the Linux basic-udev backend avoids a system libudev build dependency); `image` stores RGBA pixels without codec features; `ab_glyph` rasterizes TrueType fonts. Windows additionally uses `tray-icon` for the tray/menu, `winit` for its event loop, and `windows-sys` to attach diagnostic output to the invoking console. CLI tests use only the standard library.

## License

[MIT](LICENSE). The bundled fallback font retains its own license.
