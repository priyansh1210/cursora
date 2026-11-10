use tauri::WebviewWindow;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_F24};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowLongPtrW, SetForegroundWindow, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TOOLWINDOW};

/// APP-9.1: WebView2's browser process ended (it crashed, or an update ended it), and every window with it, the bubble
/// included. The bubble lives as long as Cursora, so it hears about it and restarts Cursora. Not in the first minute,
/// so a WebView2 that can't run doesn't make a restart loop.
pub fn on_browser_exit(window: &WebviewWindow, app: tauri::AppHandle) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{COREWEBVIEW2_PROCESS_FAILED_KIND, COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED};
    let started = std::time::Instant::now();
    let _ = window.with_webview(move |webview| unsafe {
        let Ok(core) = webview.controller().CoreWebView2() else { return };
        let handler = webview2_com::ProcessFailedEventHandler::create(Box::new(move |_, args| {
            let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND(-1); // BROWSER_PROCESS_EXITED is 0, the default
            if let Some(args) = args {
                args.ProcessFailedKind(&mut kind)?;
            }
            if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED {
                if started.elapsed() > std::time::Duration::from_secs(60) {
                    crate::restart_for_webview(&app, None);
                } else {
                    log::error!("WebView2's browser process exited in Cursora's first minute; not restarting");
                }
            }
            Ok(())
        }));
        let mut token = 0;
        if let Err(e) = core.add_ProcessFailed(&handler, &mut token) {
            log::warn!("can't watch WebView2's browser process: {e}");
        }
    });
}

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
