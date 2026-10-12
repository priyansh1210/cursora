use tauri::WebviewWindow;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_F24};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowLongPtrW, SetForegroundWindow, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TOOLWINDOW};

/// `skip_taskbar` only removes the taskbar button; a tool window is also left out of Alt+Tab (BUB-2.1).
pub fn hide_from_alt_tab(window: &WebviewWindow) {
    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_TOOLWINDOW.0 as isize);
        }
    }
}

pub fn foreground() -> isize {
    unsafe { GetForegroundWindow().0 as isize }
}

pub fn handle(window: &WebviewWindow) -> isize {
    window.hwnd().map_or(0, |h| h.0 as isize)
}

/// Brings a window to the front. If Windows refuses, an F24 tap (a key no app uses) earns the right to.
/// Tauri's `set_focus` taps Alt instead, which the app in front takes as a menu-bar press (constitution §6).
pub fn activate(hwnd: isize) -> bool {
    if hwnd == 0 {
        return false;
    }
    let hwnd = HWND(hwnd as *mut _);
    unsafe {
        if SetForegroundWindow(hwnd).as_bool() {
            return true;
        }
        let key = |flags| INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_F24, dwFlags: flags, ..Default::default() } } };
        SendInput(&[key(KEYBD_EVENT_FLAGS(0)), key(KEYEVENTF_KEYUP)], size_of::<INPUT>() as i32);
        SetForegroundWindow(hwnd).as_bool()
    }
}

pub fn focus(window: &WebviewWindow) {
    activate(handle(window));
}
