use tauri::WebviewWindow;

// shortcut: no window-handle focus control outside Windows; spec 08 adds the X11 and Wayland versions.
pub fn hide_from_alt_tab(_: &WebviewWindow) {}

pub fn foreground() -> isize {
    0
}

pub fn handle(_: &WebviewWindow) -> isize {
    0
}

pub fn activate(_: isize) -> bool {
    false
}

pub fn focus(window: &WebviewWindow) {
    let _ = window.set_focus();
}
