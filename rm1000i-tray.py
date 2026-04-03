import threading
import time
import pystray
from PIL import Image, ImageDraw, ImageFont
import liquidctl

INTERVAL = 1  # seconds between updates

def get_stats(device):
    try:
        return {k: v for k, v, *_ in device.get_status()}
    except Exception:
        return None

def make_icon(text, color=(255, 255, 255)):
    size = 256
    img = Image.new(mode="RGBA", size=(size, size), color=(0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    try:
        font = ImageFont.truetype("C:/Windows/Fonts/arialbd.ttf", size=155 if len(text) <= 3 else 120)
    except Exception:
        font = ImageFont.load_default()
    bbox = draw.textbbox((0, 0), text, font=font)
    w = bbox[2] - bbox[0]
    h = bbox[3] - bbox[1]
    x = (size - w) / 2 - bbox[0]
    y = (size - h) / 2 - bbox[1]
    draw.text((x, y), text, font=font, fill=color)
    return img

def format_tooltip(status):
    def v(key): return status.get(key, "?")
    lines = [
        f"Prikon: {v('Total power output')} W",
        f"Ucinnost: {v('Estimated efficiency')} %",
        f"+12V: {v('+12V output power')} W",
        f"+5V: {v('+5V output power')} W",
        f"+3.3V: {v('+3.3V output power')} W",
        f"VRM: {v('VRM temperature')} C",
        f"Fan: {v('Fan speed')} rpm",
    ]
    return "\n".join(lines)[:127]

def updater(icon):
    device = None
    last_icon = None
    last_title = None
    while True:
        try:
            if device is None:
                devices = list(liquidctl.find_liquidctl_devices(match="RM1000i"))
                if not devices:
                    raise RuntimeError("zařízení nenalezeno")
                device = devices[0]
                device.connect()
            status = get_stats(device)
            if not status:
                raise RuntimeError("prázdný status")
            watts = status.get("Total power output")
            label = f"{int(watts)}" if watts is not None else "??"
            color = (255, 80, 80) if watts and watts > 800 else (255, 255, 255)
            last_icon = make_icon(label, color)
            last_title = format_tooltip(status)
            icon.icon = last_icon
            icon.title = last_title
        except Exception:
            try:
                device.disconnect()
            except Exception:
                pass
            device = None
            if last_icon is not None:
                icon.icon = last_icon
                icon.title = last_title
        time.sleep(INTERVAL)

def main():
    icon = pystray.Icon(
        name="rm1000i",
        icon=make_icon("..."),
        title="RM1000i: načítám...",
        menu=pystray.Menu(
            pystray.MenuItem("Ukončit", lambda icon, item: icon.stop())
        )
    )
    t = threading.Thread(target=updater, args=(icon,), daemon=True)
    t.start()
    icon.run()

if __name__ == "__main__":
    main()
