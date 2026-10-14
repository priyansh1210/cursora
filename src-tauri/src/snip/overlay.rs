//! The snip overlay (VIS-2.1): one window over every monitor, showing the frozen grab.
use super::PENDING;
use tauri::{AppHandle, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder, WindowEvent};

pub const LABEL: &str = "snip";

/// Created for each snip, placed exactly over the grab (in screen pixels), on top and focused.
pub fn show(app: &AppHandle, (x, y, width, height): (i32, i32, u32, u32)) -> tauri::Result<()> {
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::default())
        .additional_browser_args(crate::BROWSER_ARGS)
        .title("Cursora snip")
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(cfg!(not(windows))) // GTK holds a fixed-size window at its natural size (see the bubble)
        .visible(false)
        .on_navigation(|url| crate::bubble::is_app_url(url))
        .build()?;
    window.set_position(PhysicalPosition::new(x, y))?;
    window.set_size(PhysicalSize::new(width, height))?;
    // Closed some other way (Alt+F4): the grab goes with it, and the snip is over. After snip_done the grab is already gone.
    window.on_window_event(|e| {
        if let WindowEvent::Destroyed = e {
            if PENDING.lock().unwrap().take().is_some() {
                super::done();
            }
        }
    });
    window.show()?;
    crate::bubble::focus(&window);
    Ok(())
}
