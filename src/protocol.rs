// Project: rm1000i-tray
// File: src/protocol.rs
// Description: Strictly read-only Corsair HID packet construction.
// Author: Jan Alexandr Kopřiva
// License: MIT
pub const READ_OPCODES: &[u8] = &[0x88, 0x8b, 0x8c, 0x8d, 0x8e, 0x90, 0x96, 0xee];
pub fn allowed(address: u8, opcode: u8, data: &[u8]) -> bool {
    (address == 3 && READ_OPCODES.contains(&opcode) && data.is_empty())
        || (address == 2 && opcode == 0 && data.len() == 1 && data[0] <= 2)
}
pub fn packet(address: u8, opcode: u8, data: &[u8]) -> Result<[u8; 65], String> {
    if !allowed(address, opcode, data) {
        return Err("PSU command outside read-only whitelist".into());
    }
    let mut out = [0; 65];
    out[1] = address;
    out[2] = opcode;
    out[3..3 + data.len()].copy_from_slice(data);
    Ok(out)
}
pub fn linear11(bytes: [u8; 2]) -> f64 {
    let raw = u16::from_le_bytes(bytes);
    let exponent = (raw as i16) >> 11;
    let mantissa = ((raw << 5) as i16) >> 5;
    f64::from(mantissa) * 2f64.powi(i32::from(exponent))
}
pub fn estimated_input(voltage: f64, watts: f64) -> f64 {
    let low =
        0.00010018433053123574 * watts.powi(2) + 1.0272313660072225 * watts + 14.092187353321624;
    let high =
        8.600634771656125e-05 * watts.powi(2) + 1.0289245073649413 * watts + 13.701515390258626;
    (low + (high - low) * (voltage - 115.0) / 115.0).round_ties_even()
}
