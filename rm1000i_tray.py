#
# Project: rm1000i-tray
# File:    rm1000i_tray.py
#
# Description:
# System tray widget that reads live power metrics from a Corsair RM1000i over USB HID and shows the current wattage as the icon.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# Created: 2026-04-03
#
# License: MIT
#

import contextlib
import threading
import time

from PIL import Image, ImageDraw, ImageFont

INTERVAL = 1  # poll rate in seconds; PSU firmware updates at ~1Hz

# Above this the icon turns red. Useful when tuning a GPU undervolt or looking
# for a transient the PSU is riding out.
HIGH_WATTAGE_THRESHOLD = 800

WHITE = (255, 255, 255)
RED = (255, 80, 80)

# Rendered large so Windows scales it down to tray size cleanly.
ICON_SIZE = 256

# Windows caps a tray tooltip at 128 characters.
TOOLTIP_LIMIT = 127

# Seconds to wait before registering the icon, so the shell and the tray have
# finished starting up. Without it the icon can be dropped on login.
STARTUP_DELAY = 10

DEVICE_MATCH = "RM1000i"

TOOLTIP_FIELDS = [
    ("Power", "Total power output", "W"),
    ("Efficiency", "Estimated efficiency", "%"),
    ("+12V", "+12V output power", "W"),
    ("+5V", "+5V output power", "W"),
    ("+3.3V", "+3.3V output power", "W"),
    ("VRM", "VRM temperature", "C"),
    ("Fan", "Fan speed", "rpm"),
]


def get_stats(device):
    """Status of the PSU as a plain dict, or None when the read fails.

    liquidctl yields 3-tuples of key, value and unit. The unit is already in the
    tooltip labels, so only the value is kept.
    """
    try:
        return {k: v for k, v, *_ in device.get_status()}
    except Exception:
        # Any driver or USB error means no reading this tick. The caller
        # reconnects, so the specific cause does not change what happens next.
        return None


def make_icon(text, color=WHITE):
    """Draw the wattage as a square RGBA image."""
    img = Image.new(mode="RGBA", size=(ICON_SIZE, ICON_SIZE), color=(0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    try:
        font = ImageFont.truetype(
            "C:/Windows/Fonts/arialbd.ttf", size=155 if len(text) <= 3 else 120
        )
    except OSError:
        # No Arial, for example on a machine that is not Windows.
        font = ImageFont.load_default()

    left, top, right, bottom = draw.textbbox((0, 0), text, font=font)
    x = (ICON_SIZE - (right - left)) / 2 - left
    y = (ICON_SIZE - (bottom - top)) / 2 - top
    draw.text((x, y), text, font=font, fill=color)
    return img


def format_tooltip(status):
    """One line per metric. A missing reading shows as a question mark."""
    lines = [f"{label}: {status.get(key, '?')} {unit}" for label, key, unit in TOOLTIP_FIELDS]
    return "\n".join(lines)[:TOOLTIP_LIMIT]


def label_and_color(status):
    """Text for the icon and the color it is drawn in."""
    watts = status.get("Total power output")
    if watts is None:
        return "??", WHITE
    color = RED if watts > HIGH_WATTAGE_THRESHOLD else WHITE
    return f"{int(watts)}", color


def open_device():
    """Connect to the first matching PSU, or None when none is attached."""
    import liquidctl

    devices = list(liquidctl.find_liquidctl_devices(match=DEVICE_MATCH))
    if not devices:
        return None
    device = devices[0]
    device.connect()
    return device


def close_device(device):
    """Disconnect without caring whether the handle was still usable."""
    if device is None:
        return
    with contextlib.suppress(Exception):
        # The handle is being thrown away either way.
        device.disconnect()


def poll_once(icon, device):
    """One update tick. Returns the device to use on the next tick.

    Returning None asks the caller to reconnect. A failed read leaves whatever
    the icon last showed in place, so a brief USB hiccup does not blank the
    reading or flash an error at the user.
    """
    try:
        if device is None:
            device = open_device()
        status = get_stats(device) if device is not None else None
    except Exception:
        # Any driver or USB failure means reconnect on the next tick. The widget
        # has to outlive them all, so nothing here is allowed to escape.
        close_device(device)
        return None

    if not status:
        close_device(device)
        return None

    label, color = label_and_color(status)
    icon.icon = make_icon(label, color)
    icon.title = format_tooltip(status)
    return device


def updater(icon):
    device = None
    while True:
        device = poll_once(icon, device)
        time.sleep(INTERVAL)


def main():
    import pystray

    time.sleep(STARTUP_DELAY)
    icon = pystray.Icon(
        name="rm1000i",
        icon=make_icon("..."),
        title="RM1000i: loading...",
        menu=pystray.Menu(pystray.MenuItem("Quit", lambda icon, item: icon.stop())),
    )
    threading.Thread(target=updater, args=(icon,), daemon=True).start()
    icon.run()


if __name__ == "__main__":
    main()
