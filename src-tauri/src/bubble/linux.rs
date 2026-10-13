use crate::x11::X11;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tauri::WebviewWindow;

/// X11 window managers leave a skip-taskbar window out of Alt+Tab too.
pub fn hide_from_alt_tab(_: &WebviewWindow) {}

pub fn foreground() -> isize {
    X11::connect().and_then(|x| x.active_window()).map_or(0, |w| w as isize)
}

/// The bubble's X window (also under XWayland); 0 on native Wayland.
pub fn handle(window: &WebviewWindow) -> isize {
    match window.window_handle().map(|h| h.as_raw()) {
        Ok(RawWindowHandle::Xlib(h)) => h.window as isize,
        _ => 0,
    }
}

/// Focus through the window manager, as a pager (issue 18); also gives focus back to the app on close (BUB-2.3).
pub fn activate(window: isize) -> bool {
    if window == 0 {
        return false;
    }
    X11::connect().and_then(|x| x.activate(window as u32)).map_err(|e| log::warn!("X11 activate: {e}")).is_ok()
}

pub fn focus(window: &WebviewWindow) {
    let _ = window.set_focus();
    activate(handle(window));
}
