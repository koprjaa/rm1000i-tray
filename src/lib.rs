// Project: rm1000i-tray
// File: src/lib.rs
// Description: Portable formatting, rasterization and reconnect logic.
// Author: Jan Alexandr Kopřiva
// License: AGPL-3.0-or-later
pub mod protocol;
pub mod psu;
use ab_glyph::{point, Font, FontArc, PxScale, ScaleFont};
use image::{Rgba, RgbaImage};
use std::{collections::BTreeMap, sync::LazyLock};
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Numeric {
    Integer(i64),
    Float(f64),
}
impl Numeric {
    pub fn as_f64(self) -> f64 {
        match self {
            Self::Integer(v) => v as f64,
            Self::Float(v) => v,
        }
    }
}
impl From<f64> for Numeric {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}
pub type Status = BTreeMap<String, Numeric>;
pub type Reading = (String, Numeric, &'static str);
pub fn stats_from_readings(readings: Vec<Reading>) -> Status {
    readings
        .into_iter()
        .map(|(key, value, _)| (key, value))
        .collect()
}
pub const HIGH_WATTAGE_THRESHOLD: f64 = 800.0;
pub const WHITE: [u8; 3] = [255, 255, 255];
pub const RED: [u8; 3] = [255, 80, 80];
pub const ICON_SIZE: u32 = 256;
pub const TOOLTIP_LIMIT: usize = 127;
pub const INTERVAL: u64 = 1;
pub const STARTUP_DELAY: u64 = 10;
pub const FIELDS: [(&str, &str, &str); 7] = [
    ("Power", "Total power output", "W"),
    ("Efficiency", "Estimated efficiency", "%"),
    ("+12V", "+12V output power", "W"),
    ("+5V", "+5V output power", "W"),
    ("+3.3V", "+3.3V output power", "W"),
    ("VRM", "VRM temperature", "C"),
    ("Fan", "Fan speed", "rpm"),
];
pub fn number(value: impl Into<Numeric>) -> String {
    let value = match value.into() {
        Numeric::Integer(value) => return value.to_string(),
        Numeric::Float(value) => value,
    };
    if value.is_nan() {
        return "nan".into();
    }
    // Debug formatting preserves .0 and the sign of zero. Python includes an
    // explicit exponent sign and pads single-digit exponents to two digits.
    let text = format!("{value:?}");
    if let Some((mantissa, exponent)) = text.split_once('e') {
        let exponent: i32 = exponent.parse().expect("formatted float exponent");
        format!("{mantissa}e{exponent:+03}")
    } else {
        text
    }
}
pub fn format_tooltip(status: &Status) -> String {
    FIELDS
        .iter()
        .map(|(label, key, unit)| {
            format!(
                "{label}: {} {unit}",
                status
                    .get(*key)
                    .map(|v| number(*v))
                    .unwrap_or_else(|| "?".into())
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(TOOLTIP_LIMIT)
        .collect()
}
pub fn label_and_color(status: &Status) -> (String, [u8; 3]) {
    match status
        .get("Total power output")
        .copied()
        .filter(|v| v.as_f64().is_finite())
    {
        Some(w) => (
            if let Numeric::Integer(value) = w {
                value.to_string()
            } else if w.as_f64().trunc() == 0.0 {
                "0".into()
            } else {
                format!("{:.0}", w.as_f64().trunc())
            },
            if w.as_f64() > HIGH_WATTAGE_THRESHOLD {
                RED
            } else {
                WHITE
            },
        ),
        None => ("??".into(), WHITE),
    }
}
pub fn make_icon(text: &str, color: [u8; 3]) -> RgbaImage {
    // Loaded once: the icon is redrawn every second on the UI thread.
    static FONT: LazyLock<FontArc> = LazyLock::new(|| {
        std::fs::read("C:/Windows/Fonts/arialbd.ttf")
            .ok()
            .and_then(|b| FontArc::try_from_vec(b).ok())
            .unwrap_or_else(|| {
                FontArc::try_from_slice(include_bytes!("../assets/fallback.ttf"))
                    .expect("bundled font")
            })
    });
    let font = FONT.clone();
    // ab_glyph uses pixel height; convert from Pillow's em-sized font setting.
    let size = if text.chars().count() <= 3 {
        155.0
    } else {
        120.0
    };
    let scale =
        PxScale::from(size * font.height_unscaled() / font.units_per_em().unwrap_or(2048.0));
    let scaled = font.as_scaled(scale);
    let mut cursor = 0.0;
    let mut previous = None;
    let mut glyphs = Vec::new();
    for ch in text.chars() {
        let id = font.glyph_id(ch);
        if let Some(prev) = previous {
            cursor += scaled.kern(prev, id);
        }
        let glyph = id.with_scale_and_position(scale, point(cursor, 0.0));
        cursor += scaled.h_advance(id);
        previous = Some(id);
        if let Some(outline) = font.outline_glyph(glyph) {
            glyphs.push(outline);
        }
    }
    let mut img = RgbaImage::new(ICON_SIZE, ICON_SIZE);
    if glyphs.is_empty() {
        return img;
    }
    let left = glyphs
        .iter()
        .map(|g| g.px_bounds().min.x)
        .fold(f32::INFINITY, f32::min);
    let right = glyphs
        .iter()
        .map(|g| g.px_bounds().max.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let top = glyphs
        .iter()
        .map(|g| g.px_bounds().min.y)
        .fold(f32::INFINITY, f32::min);
    let bottom = glyphs
        .iter()
        .map(|g| g.px_bounds().max.y)
        .fold(f32::NEG_INFINITY, f32::max);
    let dx = ((256.0 - right + left) / 2.0 - left).round() as i32;
    let dy = ((256.0 - bottom + top) / 2.0 - top).round() as i32;
    for g in glyphs {
        let b = g.px_bounds();
        g.draw(|x, y, a| {
            let x = x as i32 + b.min.x as i32 + dx;
            let y = y as i32 + b.min.y as i32 + dy;
            if (0..256).contains(&x) && (0..256).contains(&y) {
                img.put_pixel(
                    x as u32,
                    y as u32,
                    Rgba([color[0], color[1], color[2], (a * 255.0).round() as u8]),
                );
            }
        });
    }
    img
}
pub trait Device {
    fn status(&mut self) -> Result<Status, String>;
}
pub fn get_stats(device: &mut impl Device) -> Option<Status> {
    device.status().ok()
}
#[derive(Default)]
pub struct Monitor<D> {
    pub device: Option<D>,
    pub last: Option<Status>,
}
impl<D: Device> Monitor<D> {
    pub fn poll(&mut self, open: impl FnOnce() -> Result<Option<D>, String>) -> bool {
        if self.device.is_none() {
            self.device = open().ok().flatten();
        }
        if let Some(status) = self
            .device
            .as_mut()
            .and_then(get_stats)
            .filter(|s| !s.is_empty())
        {
            self.last = Some(status);
            true
        } else {
            self.device = None;
            false
        }
    }
}

/// Failed polls in a row after which the tray stops showing the last reading.
pub const STALE_AFTER: u32 = 5;

/// Counts consecutive failed polls; true exactly once, when the last reading
/// has become too old to keep showing as if it were live.
pub fn went_stale(failures: &mut u32, ok: bool) -> bool {
    if ok {
        *failures = 0;
        false
    } else {
        *failures = failures.saturating_add(1);
        *failures == STALE_AFTER
    }
}

pub fn tray_result(
    event_loop: Result<(), String>,
    creation_error: Option<String>,
) -> Result<(), String> {
    event_loop?;
    creation_error.map_or(Ok(()), Err)
}
