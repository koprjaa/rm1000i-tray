// Project: rm1000i-tray
// File: tests/logic.rs
// Description: Equivalents of the original widget tests and protocol safety checks.
// Author: Jan Alexandr Kopřiva
// License: MIT
use rm1000i_tray::protocol::*;
use rm1000i_tray::*;
use std::{cell::Cell, rc::Rc};
fn full() -> Status {
    [
        ("Total power output", 342.5),
        ("Estimated efficiency", 91.0),
        ("+12V output power", 300.0),
        ("+5V output power", 25.5),
        ("+3.3V output power", 12.0),
        ("VRM temperature", 46.0),
        ("Fan speed", 880.0),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v))
    .collect()
}
struct Fake {
    result: Result<Status, String>,
    closed: Rc<Cell<bool>>,
}
impl Device for Fake {
    fn status(&mut self) -> Result<Status, String> {
        self.result.clone()
    }
}
impl Drop for Fake {
    fn drop(&mut self) {
        self.closed.set(true);
    }
}
fn fake(result: Result<Status, String>) -> Fake {
    Fake {
        result,
        closed: Rc::new(Cell::new(false)),
    }
}
#[test]
fn get_stats_drops_units() {
    let readings = vec![
        ("Total power output".into(), 342.5, "W"),
        ("Fan speed".into(), 880.0, "rpm"),
    ];
    let expected = [
        ("Total power output".into(), 342.5),
        ("Fan speed".into(), 880.0),
    ]
    .into();
    assert_eq!(
        get_stats(&mut fake(Ok(stats_from_readings(readings)))),
        Some(expected)
    );
}
#[test]
fn get_stats_errors() {
    assert_eq!(get_stats(&mut fake(Err("usb gone".into()))), None);
}
#[test]
fn get_stats_empty() {
    assert_eq!(get_stats(&mut fake(Ok(Status::new()))), Some(Status::new()));
}
#[test]
fn tooltip_metrics() {
    let t = format_tooltip(&full());
    assert_eq!(t.lines().count(), 7);
    assert!(t.starts_with("Power: 342.5 W\n"));
    assert!(t.ends_with("Fan: 880.0 rpm"));
}
#[test]
fn tooltip_limit() {
    assert!(format_tooltip(&full()).len() <= TOOLTIP_LIMIT);
}
#[test]
fn tooltip_missing() {
    assert!(format_tooltip(&Status::new()).contains("Fan: ? rpm"));
}
#[test]
fn tooltip_truncated() {
    let s = full().into_keys().map(|k| (k, 1e100)).collect();
    assert_eq!(format_tooltip(&s).len(), TOOLTIP_LIMIT);
}
#[test]
fn labels_truncate() {
    for (w, label) in [
        (0.0, "0"),
        (342.5, "342"),
        (999.9, "999"),
        (-0.9, "0"),
        (-342.5, "-342"),
    ] {
        assert_eq!(
            label_and_color(&[("Total power output".into(), w)].into()),
            (label.into(), if w > 800.0 { RED } else { WHITE })
        );
    }
}
#[test]
fn red_above_threshold() {
    assert_eq!(
        label_and_color(&[("Total power output".into(), 801.0)].into()).1,
        RED
    );
}
#[test]
fn white_at_threshold() {
    assert_eq!(
        label_and_color(&[("Total power output".into(), 800.0)].into()).1,
        WHITE
    );
}
#[test]
fn zero_not_missing() {
    assert_eq!(
        label_and_color(&[("Total power output".into(), 0.0)].into()),
        ("0".into(), WHITE)
    );
}
#[test]
fn missing_wattage() {
    assert_eq!(label_and_color(&Status::new()), ("??".into(), WHITE));
}
#[test]
fn icon_square_rgba() {
    let img = make_icon("342", WHITE);
    assert_eq!(img.dimensions(), (256, 256));
    assert_eq!(img.as_raw().len(), 256 * 256 * 4);
}
#[test]
fn icon_visible() {
    assert!(make_icon("342", WHITE).pixels().any(|p| p[3] > 0));
    assert!(make_icon(" ", WHITE).pixels().all(|p| p[3] == 0));
}
#[test]
fn good_poll_updates_and_keeps_device() {
    let mut m = Monitor {
        device: Some(fake(Ok(full()))),
        last: None,
    };
    assert!(m.poll(|| panic!("already connected")));
    assert!(m.device.is_some());
    assert_eq!(m.last, Some(full()));
}
#[test]
fn failed_poll_drops_and_preserves() {
    let broken = fake(Err("gone".into()));
    let closed = broken.closed.clone();
    let mut m = Monitor {
        device: Some(broken),
        last: Some(full()),
    };
    assert!(!m.poll(|| panic!("already connected")));
    assert!(m.device.is_none());
    assert!(closed.get());
    assert_eq!(m.last, Some(full()));
}
#[test]
fn empty_poll_fails() {
    let mut m = Monitor {
        device: Some(fake(Ok(Status::new()))),
        last: None,
    };
    assert!(!m.poll(|| Ok(None)));
    assert!(m.device.is_none());
}
#[test]
fn missing_device_preserves() {
    let mut m = Monitor::<Fake> {
        device: None,
        last: None,
    };
    assert!(!m.poll(|| Ok(None)));
    assert!(m.last.is_none());
}
#[test]
fn connecting_error_contained() {
    let mut m = Monitor::<Fake> {
        device: None,
        last: Some(full()),
    };
    assert!(!m.poll(|| Err("driver failed".into())));
    assert_eq!(m.last, Some(full()));
}
#[test]
fn close_none() {
    drop(None::<Fake>);
}
#[test]
fn close_broken_handle() {
    let d = fake(Err("gone".into()));
    let closed = d.closed.clone();
    drop(d);
    assert!(closed.get());
}
#[test]
fn reconnect_after_failure() {
    let mut m = Monitor::<Fake> {
        device: None,
        last: Some(full()),
    };
    assert!(!m.poll(|| Err("gone".into())));
    assert!(m.poll(|| Ok(Some(fake(Ok(full()))))));
}
#[test]
fn linear11_signed_and_fractional() {
    assert_eq!(linear11([0x67, 0xe3]), 54.4375);
    assert_eq!(linear11([0xff, 0x07]), -1.0);
    assert_eq!(linear11([0, 0]), 0.0);
    assert_eq!(linear11([1, 8]), 2.0);
    assert_eq!(linear11([0, 0x80]), 0.0);
}
#[test]
fn whitelist_exhaustive() {
    for address in 0..=255 {
        for opcode in 0..=255 {
            assert_eq!(
                allowed(address, opcode, &[]),
                address == 3 && READ_OPCODES.contains(&opcode)
            );
            for rail in 0..=3 {
                assert_eq!(
                    allowed(address, opcode, &[rail]),
                    address == 2 && opcode == 0 && rail < 3
                );
            }
        }
    }
    assert!(!allowed(2, 0, &[0, 1]));
    for op in [0x3b, 0xd8, 0xf0, 0xfe] {
        assert!(!allowed(2, op, &[0]));
        assert!(!allowed(3, op, &[]));
    }
}
#[test]
#[should_panic(expected = "whitelist")]
fn forbidden_packet_asserts() {
    packet(2, 0xf0, &[0]);
}
#[test]
fn packet_layout() {
    let p = packet(3, 0x88, &[]);
    assert_eq!(p.len(), 65);
    assert_eq!(&p[..4], &[0, 3, 0x88, 0]);
    assert!(p[4..].iter().all(|b| *b == 0));
    assert_eq!(&packet(2, 0, &[2])[..4], &[0, 2, 0, 2]);
}
#[test]
fn efficiency_matches_reference() {
    assert_eq!(estimated_input(115.0, 500.0), 553.0);
    assert_eq!(estimated_input(230.0, 500.0), 550.0);
    assert_eq!(estimated_input(172.5, 500.0), 551.0);
}
