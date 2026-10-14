use super::{Clip, CopyKey, Platform, SourceApp};
use clipboard_win::{raw, Clipboard};
use std::path::Path;
use std::time::{Duration, Instant};
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, OpenProcessToken, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation2, IUIAutomationTextPattern, UIA_TextPatternId};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_C, VK_CONTROL, VK_INSERT,
    VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};

/// UI Automation gives up on unresponsive apps after this, so the copy fallback still fits the budget (SEL-2.5).
const UIA_TIMEOUT_MS: u32 = 60;
/// CF_BITMAP, CF_PALETTE, CF_DSPBITMAP: Windows re-creates these from CF_DIB, so leaving them out loses nothing.
const SYNTHESIZED: [u32; 3] = [2, 9, 0x82];

/// Handle-based formats (metafiles, owner-display, private and GDI objects) that can't be saved as bytes.
fn unrestorable(format: u32) -> bool {
    matches!(format, 3 | 14 | 0x80 | 0x83 | 0x8E | 0x200..=0x3FF)
}

/// Lives on one capture thread; owns that thread's COM initialization.
pub struct WinPlatform;

impl WinPlatform {
    pub fn new() -> Self {
        // An error only means COM was already initialized on this thread, which is fine.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        WinPlatform
    }
}

impl Drop for WinPlatform {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

impl Platform for WinPlatform {
    type Snapshot = Vec<(u32, Vec<u8>)>;

    fn foreground(&self) -> SourceApp {
        unsafe {
            let hwnd = GetForegroundWindow();
            let mut buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut buf).max(0) as usize;
            let title = String::from_utf16_lossy(&buf[..len]);
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 {
                return SourceApp::default(); // no window in front: nothing to read, and not an admin app
            }
            let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                // Can't even query it: protected or higher-integrity, so treat as unreadable.
                return SourceApp { title, elevated: true, window: hwnd.0 as isize, ..SourceApp::default() };
            };
            let exe_path = image_path(process);
            let elevated = is_elevated(process) && !is_elevated(GetCurrentProcess());
            let _ = CloseHandle(process);
            let path = Path::new(&exe_path);
            let exe = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            // shortcut: exe stem, not the product name; read FileDescription if names look wrong in the bubble.
            let name = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            SourceApp { exe, name, title, elevated, window: hwnd.0 as isize }
        }
    }

    fn accessible_selection(&self, max_chars: usize) -> Option<String> {
        unsafe {
            let uia: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER).ok()?;
            let _ = uia.SetConnectionTimeout(UIA_TIMEOUT_MS);
            let _ = uia.SetTransactionTimeout(UIA_TIMEOUT_MS);
            let pattern: IUIAutomationTextPattern = uia.GetFocusedElement().ok()?.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
            let ranges = pattern.GetSelection().ok()?;
            let mut text = String::new();
            for i in 0..ranges.Length().ok()? {
                let limit = (max_chars + 1).min(i32::MAX as usize) as i32; // one extra char reveals truncation
                text.push_str(&ranges.GetElement(i).ok()?.GetText(limit).ok()?.to_string());
            }
            (!text.is_empty()).then_some(text)
        }
    }

    fn extra_modifiers_released(&self, wait: Duration) -> bool {
        let deadline = Instant::now() + wait;
        loop {
            let held = [VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN].iter().any(|vk| unsafe { GetAsyncKeyState(vk.0 as i32) } < 0);
            if !held {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn clipboard_seq(&self) -> u32 {
        raw::seq_num().map_or(0, |n| n.get())
    }

    /// None when the clipboard is busy or holds something we couldn't put back exactly.
    fn clipboard_snapshot(&self) -> Option<Self::Snapshot> {
        let _open = Clipboard::new_attempts(10).ok()?;
        let formats: Vec<u32> = raw::EnumFormats::new().collect();
        if formats.iter().any(|&f| unrestorable(f)) {
            return None;
        }
        formats
            .into_iter()
            .filter(|f| !SYNTHESIZED.contains(f))
            .map(|f| {
                let mut data = Vec::new();
                raw::get_vec(f, &mut data).ok().map(|_| (f, data))
            })
            .collect()
    }

    fn clipboard_restore(&self, snapshot: Self::Snapshot) {
        let Ok(_open) = Clipboard::new_attempts(10) else {
            return log::warn!("clipboard busy; could not restore it");
        };
        let _ = raw::empty();
        for (format, data) in &snapshot {
            let _ = raw::set_without_clear(*format, data);
        }
    }

    fn clipboard_read(&self) -> Clip {
        let Ok(_open) = Clipboard::new_attempts(10) else { return Clip::Nothing };
        let mut files = Vec::new();
        if raw::get_file_list_path(&mut files).is_ok() && !files.is_empty() {
            return Clip::Files(files);
        }
        let mut utf8 = Vec::new();
        if raw::get_string(&mut utf8).is_ok() {
            return Clip::Text(String::from_utf8_lossy(&utf8).into_owned());
        }
        Clip::Nothing
    }

    fn send_copy(&self, key: CopyKey) {
        let (mods, main): (&[VIRTUAL_KEY], VIRTUAL_KEY) = match key {
            CopyKey::CtrlC => (&[VK_CONTROL], VK_C),
            CopyKey::CtrlInsert => (&[VK_CONTROL], VK_INSERT),
            CopyKey::CtrlShiftC => (&[VK_CONTROL, VK_SHIFT], VK_C),
            CopyKey::Never => return,
        };
        let mut inputs: Vec<INPUT> = mods.iter().map(|&vk| key_input(vk, false)).collect();
        inputs.push(key_input(main, false));
        inputs.push(key_input(main, true));
        inputs.extend(mods.iter().rev().map(|&vk| key_input(vk, true)));
        unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
    }
}

fn key_input(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    let mut flags = if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) };
    if vk == VK_INSERT {
        flags |= KEYEVENTF_EXTENDEDKEY; // otherwise it is Numpad 0
    }
    INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, dwFlags: flags, ..Default::default() } } }
}

unsafe fn image_path(process: HANDLE) -> String {
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    match unsafe { QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len) } {
        Ok(()) => String::from_utf16_lossy(&buf[..len as usize]),
        Err(_) => String::new(),
    }
}

/// A token we may not open belongs to a higher-integrity process, which is exactly the elevated case.
pub(crate) unsafe fn is_elevated(process: HANDLE) -> bool {
    let mut token = HANDLE::default();
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) }.is_err() {
        return true;
    }
    let mut elevation = TOKEN_ELEVATION::default();
    let mut len = 0u32;
    let ok = unsafe { GetTokenInformation(token, TokenElevation, Some(&mut elevation as *mut _ as *mut _), size_of::<TOKEN_ELEVATION>() as u32, &mut len) }.is_ok();
    let _ = unsafe { CloseHandle(token) };
    ok && elevation.TokenIsElevated != 0
}
