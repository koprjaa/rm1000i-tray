// Project: rm1000i-tray
// File: tests/audit_probe.rs
// Description: Compare serialized Python oracle values against the Rust implementation.
// Author: Jan Alexandr Kopřiva
// License: AGPL-3.0-or-later
use rm1000i_tray::{label_and_color, number};
#[test]
fn python_numeric_oracle() {
    for line in include_str!("fixtures/numbers.tsv").lines() {
        let parts: Vec<_> = line.split('\t').collect();
        let value: f64 = parts[0].parse().unwrap();
        assert_eq!(number(value), parts[1], "format {}", parts[0]);
        if parts[2] != "error" {
            assert_eq!(
                label_and_color(&[("Total power output".into(), value.into())].into()).0,
                parts[2]
            );
        }
    }
}

#[test]
fn python_linear11_oracle() {
    for line in include_str!("fixtures/linear11.tsv").lines() {
        let (raw, expected) = line.split_once('\t').unwrap();
        let raw: u16 = raw.parse().unwrap();
        let expected: f64 = expected.parse().unwrap();
        assert_eq!(
            rm1000i_tray::protocol::linear11(raw.to_le_bytes()),
            expected
        );
    }
}

#[test]
fn python_efficiency_oracle() {
    for line in include_str!("fixtures/estimates.tsv").lines() {
        let values: Vec<f64> = line.split('\t').map(|s| s.parse().unwrap()).collect();
        assert_eq!(
            rm1000i_tray::protocol::estimated_input(values[0], values[1]),
            values[2]
        );
    }
}

#[test]
fn nonfinite_wattage_is_contained() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            label_and_color(&[("Total power output".into(), value.into())].into()).0,
            "??"
        );
    }
}

#[test]
fn python_tooltip_oracle() {
    for line in include_str!("fixtures/tooltips.tsv").lines() {
        let (input, expected) = line.split_once('\t').unwrap();
        let value: f64 = input.parse().unwrap();
        let status = rm1000i_tray::FIELDS
            .iter()
            .map(|(_, key, _)| (key.to_string(), value.into()))
            .collect();
        assert_eq!(
            rm1000i_tray::format_tooltip(&status),
            expected.replace("\\n", "\n")
        );
    }
}
