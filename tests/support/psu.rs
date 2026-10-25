// Project: rm1000i-tray
// File: tests/support/psu.rs
// Description: Mock HID transport regression tests, included by the library.
// Author: Jan Alexandr Kopřiva
// License: AGPL-3.0-or-later
use super::*;
use std::cell::RefCell;
#[derive(Default)]
struct Wire {
    sent: RefCell<Vec<Vec<u8>>>,
    failure: u8,
}
impl Io for Wire {
    fn write(&self, data: &[u8]) -> Result<usize, String> {
        if self.failure == 1 {
            return Err("unplugged during write".into());
        }
        self.sent.borrow_mut().push(data.to_vec());
        Ok(if self.failure == 2 { 0 } else { data.len() })
    }
    fn read_timeout(&self, data: &mut [u8], timeout: i32) -> Result<usize, String> {
        if self.failure == 3 {
            return Err("unplugged during read".into());
        }
        if timeout == 1 {
            return Ok(if self.failure == 8 { 64 } else { 0 });
        }
        if self.failure == 4 {
            return Ok(0);
        }
        if self.failure == 5 {
            return Ok(1);
        }
        let sent = self.sent.borrow();
        let p = sent.last().unwrap();
        data[0] = p[1];
        data[1] = p[2];
        let value: u16 = if p[2] == 0x88 { 230 } else { 500 };
        data[2..4].copy_from_slice(&value.to_le_bytes());
        if self.failure == 6 {
            data[1] = 0xfe;
        }
        if self.failure == 7 {
            data[0] = 0xff;
        }
        Ok(64)
    }
}
#[test]
fn status_only_sends_whitelisted_commands() {
    let wire = Wire::default();
    let values = Reader(&wire).readings().unwrap();
    assert_eq!(values.len(), 16);
    assert!(values.contains(&("Total power output".into(), Numeric::Integer(500), "W")));
    assert!(values.contains(&("Estimated input power".into(), 550.0.into(), "W")));
    assert!(values.contains(&("Estimated efficiency".into(), 91.0.into(), "%")));
    let expected: Vec<_> = include_str!("../fixtures/readings.tsv")
        .lines()
        .map(|line| {
            let parts: Vec<_> = line.split('\t').collect();
            (
                parts[0].to_string(),
                parts[1].to_string(),
                parts[2].to_string(),
            )
        })
        .collect();
    let actual: Vec<_> = values
        .iter()
        .map(|(k, v, u)| (k.clone(), crate::number(*v), u.to_string()))
        .collect();
    assert_eq!(actual, expected);
    let sent = wire.sent.borrow();
    for p in sent.iter() {
        let payload = if p[1] == 2 { &p[3..4] } else { &p[3..3] };
        assert!(crate::protocol::allowed(p[1], p[2], payload));
        assert!(p[if p[1] == 2 { 4 } else { 3 }..].iter().all(|v| *v == 0));
    }
    let pages: Vec<_> = sent.iter().filter(|p| p[1] == 2).map(|p| p[3]).collect();
    assert_eq!(pages, [0, 0, 1, 2, 0]);
    assert_eq!(sent.len(), 19);
}
#[test]
fn usb_errors_short_packets_and_uninitialized_replies_are_errors() {
    for failure in 1..=8 {
        let wire = Wire {
            failure,
            ..Wire::default()
        };
        assert!(Reader(&wire).readings().is_err(), "failure {failure}");
    }
}
struct Replies {
    frames: RefCell<std::collections::VecDeque<Vec<u8>>>,
    reads: std::cell::Cell<usize>,
}
impl Io for Replies {
    fn write(&self, data: &[u8]) -> Result<usize, String> {
        Ok(data.len())
    }
    fn read_timeout(&self, data: &mut [u8], _: i32) -> Result<usize, String> {
        self.reads.set(self.reads.get() + 1);
        let frame = self.frames.borrow_mut().pop_front().unwrap_or_default();
        data[..frame.len()].copy_from_slice(&frame);
        Ok(frame.len())
    }
}
#[test]
fn unrelated_and_short_reports_can_precede_valid_reply() {
    let mut valid = vec![0; 64];
    valid[..4].copy_from_slice(&[3, 0x88, 230, 0]);
    let mut unrelated = valid.clone();
    unrelated[1] = 0x90;
    let wire = Replies {
        frames: RefCell::new([unrelated, vec![3], valid].into()),
        reads: std::cell::Cell::new(0),
    };
    assert_eq!(Reader(&wire).value(0x88).unwrap(), Numeric::Integer(230));
    assert_eq!(wire.reads.get(), 3);
}
#[test]
fn timeout_does_not_busy_spin() {
    let wire = Replies {
        frames: RefCell::new(Default::default()),
        reads: std::cell::Cell::new(0),
    };
    assert!(Reader(&wire).value(0x88).is_err());
    assert_eq!(wire.reads.get(), 1);
}

#[test]
fn python_linear11_numeric_types() {
    for line in include_str!("../fixtures/linear11.tsv").lines() {
        let (raw, expected) = line.split_once('\t').unwrap();
        let bytes = raw.parse::<u16>().unwrap().to_le_bytes();
        let mut frame = vec![0; 64];
        frame[..4].copy_from_slice(&[3, 0x88, bytes[0], bytes[1]]);
        let wire = Replies {
            frames: RefCell::new([frame].into()),
            reads: std::cell::Cell::new(0),
        };
        let value = Reader(&wire).value(0x88).unwrap();
        assert_eq!(crate::number(value), expected, "raw {raw}");
    }
}
