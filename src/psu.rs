// Project: rm1000i-tray
// File: src/psu.rs
// Description: Bounded HID transactions for the RM1000i, without initialization writes.
// Author: Jan Alexandr Kopřiva
// License: MIT
use crate::{
    protocol::{estimated_input, linear11, packet},
    stats_from_readings, Device, Status,
};
pub struct Psu(hidapi::HidDevice);
impl Psu {
    pub fn open() -> Result<Option<Self>, String> {
        let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
        let info = api
            .device_list()
            .find(|d| d.vendor_id() == 0x1b1c && d.product_id() == 0x1c0d);
        info.map(|d| d.open_device(&api).map(Self).map_err(|e| e.to_string()))
            .transpose()
    }
    pub fn readings(&self) -> Result<Vec<(String, f64, &'static str)>, String> {
        Reader(&self.0).readings()
    }
}
trait Io {
    fn write(&self, data: &[u8]) -> Result<usize, String>;
    fn read_timeout(&self, data: &mut [u8], timeout: i32) -> Result<usize, String>;
}
impl Io for hidapi::HidDevice {
    fn write(&self, data: &[u8]) -> Result<usize, String> {
        self.write(data).map_err(|e| e.to_string())
    }
    fn read_timeout(&self, data: &mut [u8], timeout: i32) -> Result<usize, String> {
        self.read_timeout(data, timeout).map_err(|e| e.to_string())
    }
}
struct Reader<'a, T>(&'a T);
impl<T: Io> Reader<'_, T> {
    fn exec(&self, address: u8, opcode: u8, data: &[u8]) -> Result<[u8; 64], String> {
        let request = packet(address, opcode, data);
        let written = self.0.write(&request).map_err(|e| e.to_string())?;
        if written != request.len() {
            return Err("Short HID write".into());
        }
        // Other tools (FanControl, iCUE) poll the same PSU, so replies to their
        // requests can arrive first. Skip them until ours shows up or time runs out.
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1000);
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                return Err(
                    "Invalid or timed out HID response; PSU may be uninitialized or busy".into(),
                );
            }
            let mut response = [0; 64];
            let count = self
                .0
                .read_timeout(&mut response, left.as_millis().max(1) as i32)
                .map_err(|e| e.to_string())?;
            if count == 64 && response[..2] == [address, opcode] {
                return Ok(response);
            }
        }
    }
    fn page(&self, rail: u8) -> Result<(), String> {
        self.exec(2, 0, &[rail]).map(|_| ())
    }
    fn value(&self, opcode: u8) -> Result<f64, String> {
        let r = self.exec(3, opcode, &[])?;
        Ok(linear11([r[2], r[3]]))
    }
    pub fn readings(&self) -> Result<Vec<(String, f64, &'static str)>, String> {
        let mut discard = [0; 64];
        // Bound draining even if another process continuously generates reports.
        for i in 0..65 {
            if self
                .0
                .read_timeout(&mut discard, 1)
                .map_err(|e| e.to_string())?
                == 0
            {
                break;
            }
            if i == 64 {
                return Err("HID queue did not drain".into());
            }
        }
        self.page(0)?;
        let voltage = self.value(0x88)?;
        let mut values = Vec::new();
        for (key, opcode, unit) in [
            ("VRM temperature", 0x8d, "°C"),
            ("Case temperature", 0x8e, "°C"),
            ("Fan speed", 0x90, "rpm"),
        ] {
            values.push((key.into(), self.value(opcode)?, unit));
        }
        values.push(("Input voltage".into(), voltage, "V"));
        for (rail, name) in ["+12V", "+5V", "+3.3V"].iter().enumerate() {
            self.page(rail as u8)?;
            for (suffix, opcode, unit) in [
                ("voltage", 0x8b, "V"),
                ("current", 0x8c, "A"),
                ("power", 0x96, "W"),
            ] {
                values.push((format!("{name} output {suffix}"), self.value(opcode)?, unit));
            }
        }
        let watts = self.value(0xee)?;
        let input = estimated_input(voltage, watts);
        if input <= 0.0 {
            return Err("Invalid estimated input power".into());
        }
        values.extend([
            ("Total power output".into(), watts, "W"),
            ("Estimated input power".into(), input, "W"),
            (
                "Estimated efficiency".into(),
                (watts / input * 100.0).round_ties_even(),
                "%",
            ),
        ]);
        self.page(0)?;
        Ok(values)
    }
}
impl Device for Psu {
    fn status(&mut self) -> Result<Status, String> {
        Ok(stats_from_readings(self.readings()?))
    }
}

#[cfg(test)]
mod tests {
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
        assert!(values.contains(&("Total power output".into(), 500.0, "W")));
        assert!(values.contains(&("Estimated input power".into(), 550.0, "W")));
        assert!(values.contains(&("Estimated efficiency".into(), 91.0, "%")));
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
}
