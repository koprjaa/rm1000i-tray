// Project: rm1000i-tray
// File: src/main.rs
// Description: Tray entry point and one-shot hardware diagnostics.
// Author: Jan Alexandr Kopřiva
// License: AGPL-3.0-or-later
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#[cfg(windows)]
mod windows;
use rm1000i_tray::{number, psu::Psu};
fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    #[cfg(all(windows, not(debug_assertions)))]
    if !args.is_empty() {
        // Attach CLI output to the invoking terminal without creating a new window.
        unsafe {
            windows_sys::Win32::System::Console::AttachConsole(
                windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
            );
        }
    }
    if args == ["--help"] || args == ["-h"] {
        println!("Usage: rm1000i-tray [--once]\n  --once  Read PSU values once without a tray");
        return std::process::ExitCode::SUCCESS;
    }
    if args == ["--once"] {
        let result = Psu::open()
            .and_then(|d| d.ok_or_else(|| "RM1000i not found".to_string()))
            .and_then(|d| d.readings());
        return match result {
            Ok(values) => {
                for (key, value, unit) in values {
                    println!("{key}: {} {unit}", number(value));
                }
                std::process::ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::ExitCode::FAILURE
            }
        };
    }
    if !args.is_empty() {
        eprintln!("Unknown argument. Usage: rm1000i-tray [--once]");
        return std::process::ExitCode::from(2);
    }
    #[cfg(windows)]
    {
        match windows::run() {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{e}");
                std::process::ExitCode::FAILURE
            }
        }
    }
    #[cfg(not(windows))]
    {
        eprintln!("Tray mode requires Windows; use --once for diagnostics.");
        std::process::ExitCode::FAILURE
    }
}
