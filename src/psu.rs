// Project: rm1000i-tray
// File: src/psu.rs
// Description: Bounded HID transactions for the RM1000i, without initialization writes.
// Author: Jan Alexandr Kopřiva
// License: AGPL-3.0-or-later
use crate::{
    protocol::{estimated_input, linear11, packet},
    stats_from_readings, Device, Numeric, Reading, Status,
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
    pub fn readings(&self) -> Result<Vec<Reading>, String> {
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
        let request = packet(address, opcode, data)?;
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
            if count == 0 {
                return Err("Timed out HID response".into());
            }
            if count == 64 && response[..2] == [address, opcode] {
                return Ok(response);
            }
        }
    }
    fn page(&self, rail: u8) -> Result<(), String> {
        self.exec(2, 0, &[rail]).map(|_| ())
    }
    fn value(&self, opcode: u8) -> Result<Numeric, String> {
        let r = self.exec(3, opcode, &[])?;
        let value = linear11([r[2], r[3]]);
        // Python's integer exponentiation returns an int for nonnegative exponents.
        Ok(if (r[3] as i8) >> 3 >= 0 {
            Numeric::Integer(value as i64)
        } else {
            Numeric::Float(value)
        })
    }
    pub fn readings(&self) -> Result<Vec<Reading>, String> {
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
        let input = estimated_input(voltage.as_f64(), watts.as_f64());
        if input <= 0.0 {
            return Err("Invalid estimated input power".into());
        }
        values.extend([
            ("Total power output".into(), watts, "W"),
            ("Estimated input power".into(), input.into(), "W"),
            (
                "Estimated efficiency".into(),
                (watts.as_f64() / input * 100.0).round_ties_even().into(),
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
#[path = "../tests/support/psu.rs"]
mod tests;
