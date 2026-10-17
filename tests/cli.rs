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

#[test]
fn argument_combinations_are_rejected() {
    for args in [
        vec!["--once", "--once"],
        vec!["--help", "--once"],
        vec![""],
        vec!["\u{feff}--once"],
        vec!["--once\r\n"],
        vec!["žluťoučký"],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}
#[test]
#[cfg(unix)]
fn invalid_utf8_argument_is_rejected_without_panicking() {
    use std::os::unix::ffi::OsStringExt;
    let output = Command::new(env!("CARGO_BIN_EXE_rm1000i-tray"))
        .arg(std::ffi::OsString::from_vec(vec![255]))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked"));
}

#[test]
fn large_argument_is_rejected() {
    let large = "x".repeat(16000);
    assert_eq!(run(&[&large]).status.code(), Some(2));
}
