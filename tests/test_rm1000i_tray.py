#
# Project: rm1000i-tray
# File:    test_rm1000i_tray.py
#
# Description:
# Tests for the tooltip text, the icon color threshold, and a device that reports nothing.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# Created: 2026-04-03
#
# License: MIT
#

"""Tests for the tray widget logic.

No PSU and no tray backend are needed. liquidctl and pystray are imported inside
the functions that talk to them, so the module imports on any machine.
"""

from types import SimpleNamespace

import pytest

from rm1000i_tray import (
    HIGH_WATTAGE_THRESHOLD,
    ICON_SIZE,
    RED,
    TOOLTIP_LIMIT,
    WHITE,
    close_device,
    format_tooltip,
    get_stats,
    label_and_color,
    make_icon,
    poll_once,
)

FULL_STATUS = {
    "Total power output": 342.5,
    "Estimated efficiency": 91.0,
    "+12V output power": 300.0,
    "+5V output power": 25.5,
    "+3.3V output power": 12.0,
    "VRM temperature": 46.0,
    "Fan speed": 880,
}


class FakeIcon:
    def __init__(self):
        self.icon = None
        self.title = None


class FakeDevice:
    """Stands in for a liquidctl device handle."""

    def __init__(self, status=None, raises=None):
        self._status = status or []
        self._raises = raises
        self.disconnected = False

    def get_status(self):
        if self._raises:
            raise self._raises
        return self._status

    def disconnect(self):
        self.disconnected = True


# --- get_stats --------------------------------------------------------------


def test_get_stats_drops_the_unit_from_each_triple():
    device = FakeDevice([("Total power output", 342.5, "W"), ("Fan speed", 880, "rpm")])
    assert get_stats(device) == {"Total power output": 342.5, "Fan speed": 880}


def test_get_stats_returns_none_when_the_device_errors():
    assert get_stats(FakeDevice(raises=OSError("usb gone"))) is None


def test_get_stats_on_a_device_that_reports_nothing():
    assert get_stats(FakeDevice([])) == {}


# --- format_tooltip ---------------------------------------------------------


def test_tooltip_lists_every_metric():
    tooltip = format_tooltip(FULL_STATUS)
    lines = tooltip.split("\n")
    assert len(lines) == 7
    assert lines[0] == "Power: 342.5 W"
    assert lines[-1] == "Fan: 880 rpm"


def test_tooltip_fits_inside_the_windows_limit():
    assert len(format_tooltip(FULL_STATUS)) <= TOOLTIP_LIMIT


def test_a_missing_reading_shows_as_a_question_mark():
    assert "Fan: ? rpm" in format_tooltip({})


def test_tooltip_is_truncated_rather_than_overflowing():
    absurd = dict.fromkeys(FULL_STATUS, "9" * 100)
    assert len(format_tooltip(absurd)) == TOOLTIP_LIMIT


# --- label_and_color --------------------------------------------------------


@pytest.mark.parametrize(
    ("watts", "expected_label"),
    [(0, "0"), (342.5, "342"), (999.9, "999")],
)
def test_label_is_the_truncated_wattage(watts, expected_label):
    label, _ = label_and_color({"Total power output": watts})
    assert label == expected_label


def test_the_icon_turns_red_above_the_threshold():
    _, color = label_and_color({"Total power output": HIGH_WATTAGE_THRESHOLD + 1})
    assert color == RED


def test_the_icon_stays_white_at_the_threshold():
    _, color = label_and_color({"Total power output": HIGH_WATTAGE_THRESHOLD})
    assert color == WHITE


def test_a_zero_reading_is_shown_and_not_treated_as_missing():
    """0 W is falsy, so a truthiness check would render it as unknown."""
    label, color = label_and_color({"Total power output": 0})
    assert label == "0"
    assert color == WHITE


def test_a_missing_wattage_shows_two_question_marks():
    assert label_and_color({}) == ("??", WHITE)


# --- make_icon --------------------------------------------------------------


def test_icon_is_a_square_rgba_image():
    img = make_icon("342")
    assert img.size == (ICON_SIZE, ICON_SIZE)
    assert img.mode == "RGBA"


def test_icon_draws_something_visible():
    blank = make_icon(" ").getchannel("A").getextrema()
    drawn = make_icon("342").getchannel("A").getextrema()
    assert drawn[1] > 0
    assert drawn != blank


# --- poll_once --------------------------------------------------------------


def test_a_good_read_updates_the_icon_and_keeps_the_device():
    icon, device = FakeIcon(), FakeDevice([(k, v, "") for k, v in FULL_STATUS.items()])
    assert poll_once(icon, device) is device
    assert icon.title.startswith("Power: 342.5 W")
    assert icon.icon is not None


def test_a_failed_read_drops_the_device_and_keeps_the_last_icon():
    icon = FakeIcon()
    good = FakeDevice([(k, v, "") for k, v in FULL_STATUS.items()])
    poll_once(icon, good)
    previous_icon, previous_title = icon.icon, icon.title

    broken = FakeDevice(raises=OSError("usb gone"))
    assert poll_once(icon, broken) is None
    assert broken.disconnected is True
    assert icon.icon is previous_icon
    assert icon.title == previous_title


def test_an_empty_status_is_treated_as_a_failure():
    icon, device = FakeIcon(), FakeDevice([])
    assert poll_once(icon, device) is None


def test_a_missing_device_leaves_the_icon_alone(monkeypatch):
    """No PSU attached must not blank the tray or raise out of the loop."""
    import rm1000i_tray

    monkeypatch.setattr(rm1000i_tray, "open_device", lambda: None)
    icon = FakeIcon()
    assert poll_once(icon, None) is None
    assert icon.icon is None


def test_a_driver_failure_while_connecting_is_contained(monkeypatch):
    import rm1000i_tray

    def boom():
        raise OSError

    monkeypatch.setattr(rm1000i_tray, "open_device", boom)
    icon = FakeIcon()
    assert poll_once(icon, None) is None
    assert icon.icon is None


# --- close_device -----------------------------------------------------------


def test_close_device_tolerates_none():
    close_device(None)


def test_close_device_swallows_a_broken_handle():
    broken = SimpleNamespace(disconnect=lambda: (_ for _ in ()).throw(OSError("gone")))
    close_device(broken)
