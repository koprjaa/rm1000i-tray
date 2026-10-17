// Project: rm1000i-tray
// File: src/windows.rs
// Description: Windows event loop and background USB polling.
// Author: Jan Alexandr Kopřiva
// License: MIT
use rm1000i_tray::{
    format_tooltip, label_and_color, make_icon, psu::Psu, Monitor, Status, INTERVAL, STARTUP_DELAY,
    WHITE,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};
use winit::{
    application::ApplicationHandler,
    event_loop::{ActiveEventLoop, EventLoop},
};
struct App {
    tray: Option<TrayIcon>,
    quit: tray_icon::menu::MenuId,
    menu: Option<Menu>,
    error: Option<String>,
}
fn icon(text: &str, color: [u8; 3]) -> Result<Icon, tray_icon::BadIcon> {
    // Hand Windows an icon already at tray size. Left to scale 256 px itself it
    // drops pixels and the digits come out jagged; pystray avoided this by
    // passing a multi-size .ico that Pillow downscaled with Lanczos.
    let size = tray_icon_size();
    let img = image::imageops::resize(
        &make_icon(text, color),
        size,
        size,
        image::imageops::FilterType::Lanczos3,
    );
    Icon::from_rgba(img.into_raw(), size, size)
}
fn tray_icon_size() -> u32 {
    use windows_sys::Win32::UI::{
        HiDpi::{GetDpiForSystem, GetSystemMetricsForDpi},
        WindowsAndMessaging::SM_CXSMICON,
    };
    // SAFETY: both calls only read system settings.
    let px = unsafe { GetSystemMetricsForDpi(SM_CXSMICON, GetDpiForSystem()) };
    if px > 0 {
        px as u32
    } else {
        16
    }
}
impl ApplicationHandler<Status> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.tray.is_none() {
            let result = icon("...", WHITE)
                .map_err(|e| e.to_string())
                .and_then(|icon| {
                    let menu = self
                        .menu
                        .take()
                        .ok_or_else(|| "Tray menu already consumed".to_string())?;
                    TrayIconBuilder::new()
                        .with_icon(icon)
                        .with_tooltip("RM1000i: loading...")
                        .with_menu(Box::new(menu))
                        .build()
                        .map_err(|e| e.to_string())
                });
            match result {
                Ok(tray) => self.tray = Some(tray),
                Err(error) => {
                    self.error = Some(error);
                    event_loop.exit();
                }
            }
        }
    }
    fn user_event(&mut self, _: &ActiveEventLoop, status: Status) {
        if let Some(tray) = &self.tray {
            let (text, color) = label_and_color(&status);
            if let Ok(icon) = icon(&text, color) {
                let _ = tray.set_icon(Some(icon));
            }
            let _ = tray.set_tooltip(Some(format_tooltip(&status)));
        }
    }
    fn window_event(
        &mut self,
        _: &ActiveEventLoop,
        _: winit::window::WindowId,
        _: winit::event::WindowEvent,
    ) {
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.quit {
                event_loop.exit();
            }
        }
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
            std::time::Instant::now() + Duration::from_millis(100),
        ));
    }
}
pub fn run() -> Result<(), String> {
    thread::sleep(Duration::from_secs(STARTUP_DELAY));
    let event_loop = EventLoop::<Status>::with_user_event()
        .build()
        .map_err(|e| e.to_string())?;
    let menu = Menu::new();
    let quit = MenuItem::new("Quit", true, None);
    menu.append(&quit).map_err(|e| e.to_string())?;
    let proxy = event_loop.create_proxy();
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    thread::spawn(move || {
        let mut monitor = Monitor::<Psu> {
            device: None,
            last: None,
        };
        while !worker_stop.load(Ordering::Relaxed) {
            if monitor.poll(Psu::open) {
                if let Some(status) = monitor.last.clone() {
                    if proxy.send_event(status).is_err() {
                        break;
                    }
                }
            }
            thread::sleep(Duration::from_secs(INTERVAL));
        }
    });
    let mut app = App {
        tray: None,
        quit: quit.id().clone(),
        menu: Some(menu),
        error: None,
    };
    let result = event_loop.run_app(&mut app).map_err(|e| e.to_string());
    stop.store(true, Ordering::Relaxed);
    rm1000i_tray::tray_result(result, app.error)
}
