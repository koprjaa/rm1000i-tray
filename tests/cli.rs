// Project: rm1000i-tray
// File: tests/cli.rs
// Description: CLI subprocess integration tests without requiring hardware.
// Author: Jan Alexandr Kopřiva
// License: MIT
use std::process::Command;
fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rm1000i-tray"))
        .args(args)
        .output()
        .expect("run CLI")
}
#[test]
fn help() {
    let o = run(&["--help"]);
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).contains("--once"));
}
#[test]
fn invalid_argument() {
    assert_eq!(run(&["--invalid"]).status.code(), Some(2));
}
#[test]
fn once() {
    let o = run(&["--once"]);
    if o.status.success() {
        let s = String::from_utf8_lossy(&o.stdout);
        for key in [
            "Total power output:",
            "Estimated efficiency:",
            "+12V output power:",
            "Fan speed:",
        ] {
            assert!(s.contains(key));
        }
    } else {
        assert_eq!(o.status.code(), Some(1));
        assert!(!o.stderr.is_empty());
        assert!(o.stdout.is_empty());
    }
}
#[test]
#[cfg(not(windows))]
fn tray_requires_windows() {
    let o = run(&[]);
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("Windows"));
}
