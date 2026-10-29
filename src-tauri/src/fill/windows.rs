//! Spec 27 on Windows: UI Automation reads the form and focuses its boxes, `SendInput` types, and a Win32 window draws the frame.
use super::{Field, Target};
use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::time::{Duration, Instant};
use windows::core::{w, BOOL, BSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{CombineRgn, CreateRectRgn, CreateSolidBrush, DeleteObject, SetWindowRgn, RGN_DIFF};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_I4};
use windows::Win32::UI::Accessibility::{
    CUIAutomation8, IUIAutomation2, IUIAutomationCacheRequest, IUIAutomationCondition, IUIAutomationElement, IUIAutomationValuePattern, TreeScope_Descendants, UIA_CONTROLTYPE_ID,
    UIA_BoundingRectanglePropertyId, UIA_ControlTypePropertyId, UIA_DocumentControlTypeId, UIA_EditControlTypeId, UIA_HelpTextPropertyId, UIA_IsEnabledPropertyId,
    UIA_IsKeyboardFocusablePropertyId, UIA_IsPasswordPropertyId, UIA_NamePropertyId, UIA_ValueIsReadOnlyPropertyId, UIA_ValuePatternId, UIA_ValueValuePropertyId,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_CONTROL, VK_ESCAPE, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetForegroundWindow, PeekMessageW, RegisterClassW, SetLayeredWindowAttributes, SetWindowPos,
    SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HHOOK, HWND_TOPMOST, KBDLLHOOKSTRUCT, LWA_ALPHA, MSG, PM_REMOVE, SWP_NOACTIVATE, SWP_SHOWWINDOW, WH_KEYBOARD_LL, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

/// The time between two letters, and between two boxes: fast enough not to wait for, slow enough to watch (FIL-3.1).
const LETTER_MS: u64 = 12;
const BOX_MS: u64 = 150;
/// Cursora's green (#15803d), as 0x00BBGGRR.
const GREEN: COLORREF = COLORREF(0x003D_8015);
/// The frame: its gap to the box and its thickness, in pixels.
const GAP: i32 = 3;
const THICK: i32 = 3;

/// COM for this thread, released when dropped, only if this call started it.
struct Com(bool);

impl Com {
    fn init() -> Self {
        Com(unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok())
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

pub fn read(window: isize) -> Result<Vec<Field>, String> {
    let _com = Com::init();
    let uia = automation()?;
    Ok(boxes(&uia, window)?.into_iter().map(|(f, _)| f).collect())
}

/// Returns how many boxes were filled, the labels of those that couldn't be focused, and why it stopped early, if it did.
pub fn type_into(window: isize, targets: &[Target], before: impl FnOnce()) -> Result<(usize, Vec<String>, Option<&'static str>), String> {
    let _com = Com::init();
    let uia = automation()?;
    let found = boxes(&uia, window)?;
    // The page may have changed since the plan: only a box at the same place, with the same label and still empty, is typed into.
    let todo: Vec<(&Target, &IUIAutomationElement)> = targets.iter().filter_map(|t| found.get(t.index).filter(|(f, _)| !f.filled && f.name() == t.field).map(|(_, el)| (t, el))).collect();
    if todo.is_empty() {
        return Err("The form changed since Cursora read it, so nothing was filled. Ask /fill again.".into());
    }
    if !modifiers_up() {
        return Err("Ctrl, Shift, Alt or Windows was still held down, so nothing was filled.".into());
    }
    before();
    let _esc = EscHook::new();
    let frame = Frame::new();
    let (mut done, mut missed) = (0, Vec::new());
    for (t, el) in todo {
        if let Some(why) = stop(window) {
            return Ok((done, missed, Some(why)));
        }
        if !focus(&uia, el) {
            missed.push(t.field.clone());
            continue;
        }
        if let (Some(f), Ok(r)) = (&frame, unsafe { el.CurrentBoundingRectangle() }) {
            f.around(r);
        }
        for unit in t.value.encode_utf16() {
            if let Some(why) = stop(window) {
                return Ok((done, missed, Some(why)));
            }
            letter(unit);
            wait(LETTER_MS);
        }
        done += 1;
        wait(BOX_MS);
    }
    log::info!("fill: {done} boxes filled, {} not focused", missed.len()); // counts only (APP-7.2)
    Ok((done, missed, None))
}

fn automation() -> Result<IUIAutomation2, String> {
    unsafe {
        let uia: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER).map_err(|e| format!("UI Automation isn't available ({}).", e.message()))?;
        // A big page takes a while; the capture's 60 ms is for one element.
        let _ = uia.SetConnectionTimeout(2_000);
        let _ = uia.SetTransactionTimeout(10_000);
        Ok(uia)
    }
}

/// The boxes. Chrome builds its accessibility tree once a client asks, so a read that finds none is tried again.
fn boxes(uia: &IUIAutomation2, window: isize) -> Result<Vec<(Field, IUIAutomationElement)>, String> {
    let found = unsafe { find(uia, window) }.map_err(|e| format!("Cursora couldn't read the form ({}).", e.message()))?;
    if !found.is_empty() {
        return Ok(found);
    }
    std::thread::sleep(Duration::from_millis(500));
    unsafe { find(uia, window) }.map_err(|e| format!("Cursora couldn't read the form ({}).", e.message()))
}

/// The enabled, focusable, writable Edit controls: in the window's documents where it has any (a browser's page, not its
/// address bar), else in the whole window (a dialog box).
// shortcut: Edit controls only; a box shown as a combo box (a web page's role=combobox) isn't read. Read editable combo boxes too if forms need them.
unsafe fn find(uia: &IUIAutomation2, window: isize) -> windows::core::Result<Vec<(Field, IUIAutomationElement)>> {
    unsafe {
        let root = uia.ElementFromHandle(HWND(window as *mut _))?;
        let cache = uia.CreateCacheRequest()?;
        for p in [
            UIA_NamePropertyId, UIA_HelpTextPropertyId, UIA_IsPasswordPropertyId, UIA_IsEnabledPropertyId, UIA_IsKeyboardFocusablePropertyId, UIA_BoundingRectanglePropertyId,
            UIA_ValueValuePropertyId, UIA_ValueIsReadOnlyPropertyId,
        ] {
            cache.AddProperty(p)?;
        }
        cache.AddPattern(UIA_ValuePatternId)?;
        let edits = control(uia, UIA_EditControlTypeId)?;
        let docs = root.FindAll(TreeScope_Descendants, &control(uia, UIA_DocumentControlTypeId)?)?;
        let left = root.CurrentBoundingRectangle()?.left;
        let mut out = Vec::new();
        for i in 0..docs.Length()? {
            collect(&docs.GetElement(i)?, &edits, &cache, left, &mut out)?;
        }
        if out.is_empty() {
            collect(&root, &edits, &cache, left, &mut out)?;
        }
        Ok(out.into_iter().map(|(f, _, el)| (f, el)).collect())
    }
}

/// ControlType == `kind`, as a property condition (VT_I4).
unsafe fn control(uia: &IUIAutomation2, kind: UIA_CONTROLTYPE_ID) -> windows::core::Result<IUIAutomationCondition> {
    let value = VARIANT { Anonymous: VARIANT_0 { Anonymous: ManuallyDrop::new(VARIANT_0_0 { vt: VT_I4, Anonymous: VARIANT_0_0_0 { lVal: kind.0 }, ..Default::default() }) } };
    unsafe { uia.CreatePropertyCondition(UIA_ControlTypePropertyId, &value) }
}

/// Adds `root`'s boxes. A page inside a page (an iframe) is a document inside a document, so a box already found, by its
/// label and place on screen, is skipped. `left`: the window's left edge.
unsafe fn collect(root: &IUIAutomationElement, edits: &IUIAutomationCondition, cache: &IUIAutomationCacheRequest, left: i32, out: &mut Vec<(Field, RECT, IUIAutomationElement)>) -> windows::core::Result<()> {
    let on = |b: windows::core::Result<BOOL>| b.is_ok_and(|b| b.as_bool());
    let text = |s: windows::core::Result<BSTR>| s.map(|s| s.to_string()).unwrap_or_default();
    unsafe {
        let found = root.FindAllBuildCache(TreeScope_Descendants, edits, cache)?;
        for i in 0..found.Length()? {
            let el = found.GetElement(i)?;
            let Ok(value) = el.GetCachedPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) else { continue };
            if !on(el.CachedIsEnabled()) || !on(el.CachedIsKeyboardFocusable()) || on(value.CachedIsReadOnly()) {
                continue;
            }
            let rect = el.CachedBoundingRectangle().unwrap_or_default();
            // A box the user can't see gets none of their details: a page can hide one (a pixel, or far to the left) to
            // collect more than it shows. A box below the fold still counts.
            if !visible(rect, left) {
                continue;
            }
            let field = Field { label: text(el.CachedName()), hint: text(el.CachedHelpText()), filled: !text(value.CachedValue()).trim().is_empty(), password: on(el.CachedIsPassword()) };
            if !out.iter().any(|(f, r, _)| *r == rect && f.label == field.label) {
                out.push((field, rect, el));
            }
        }
    }
    Ok(())
}

/// Big enough to see, and not wholly to the left of the window (FIL-2.2).
fn visible(r: RECT, left: i32) -> bool {
    r.right - r.left >= 4 && r.bottom - r.top >= 4 && r.right > left
}

/// Focuses the box and waits up to 500 ms for UI Automation to say so; a box that never says so isn't typed into.
fn focus(uia: &IUIAutomation2, el: &IUIAutomationElement) -> bool {
    if unsafe { el.SetFocus() }.is_err() {
        return false;
    }
    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        let focused = unsafe { uia.GetFocusedElement().and_then(|f| uia.CompareElements(&f, el)) };
        if focused.is_ok_and(|same| same.as_bool()) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        wait(20);
    }
}

/// FIL-3.3: Esc, or another window in front, stops the typing before the next letter.
fn stop(window: isize) -> Option<&'static str> {
    if ESC.load(Ordering::SeqCst) {
        return Some("you pressed Esc");
    }
    (unsafe { GetForegroundWindow() }.0 as isize != window).then_some("another window came to the front")
}

/// Set by the hook when Esc is pressed while Cursora types.
static ESC: AtomicBool = AtomicBool::new(false);

/// While Cursora types, Esc goes to Cursora, not to the app: in a dialog box or a web page's pop-up, Esc would close the
/// form. A hook also catches a press of any length, which looking at the key between two letters doesn't. It needs this
/// thread to handle messages, which `wait` does.
struct EscHook(Option<HHOOK>);

impl EscHook {
    fn new() -> Self {
        ESC.store(false, Ordering::SeqCst);
        let instance = unsafe { GetModuleHandleW(None) }.ok();
        EscHook(unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(esc_hook), instance.map(Into::into), 0) }.ok())
    }
}

impl Drop for EscHook {
    fn drop(&mut self) {
        if let Some(hook) = self.0 {
            let _ = unsafe { UnhookWindowsHookEx(hook) };
        }
    }
}

unsafe extern "system" fn esc_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // The letters typed are VK_PACKET, so an Esc is always the user's (or a test's).
    if code >= 0 && unsafe { (*(lparam.0 as *const KBDLLHOOKSTRUCT)).vkCode } == u32::from(VK_ESCAPE.0) {
        ESC.store(true, Ordering::SeqCst);
        return LRESULT(1);
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// A held Ctrl would turn the letters into shortcuts; the user may still be letting go of a key.
fn modifiers_up() -> bool {
    let deadline = Instant::now() + Duration::from_millis(300);
    loop {
        if ![VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN].iter().any(|vk| unsafe { GetAsyncKeyState(vk.0 as i32) } < 0) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// One UTF-16 unit, as the character itself rather than a key, so any keyboard layout types it as it is.
fn letter(unit: u16) {
    let input = |flags| INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(0), wScan: unit, dwFlags: flags, ..Default::default() } } };
    unsafe { SendInput(&[input(KEYEVENTF_UNICODE), input(KEYEVENTF_UNICODE | KEYEVENTF_KEYUP)], size_of::<INPUT>() as i32) };
}

/// Sleeps, handling this thread's window messages meanwhile, so the frame gets drawn.
fn wait(ms: u64) {
    let deadline = Instant::now() + Duration::from_millis(ms);
    loop {
        let mut msg = MSG::default();
        while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return;
        }
        std::thread::sleep(left.min(Duration::from_millis(5)));
    }
}

/// A green frame around the box being filled: a topmost window shaped like a frame by its region, which never takes the
/// focus or a click.
struct Frame(HWND);

impl Frame {
    fn new() -> Option<Self> {
        static CLASS: Once = Once::new();
        unsafe {
            let instance = GetModuleHandleW(None).ok()?;
            CLASS.call_once(|| {
                let class = WNDCLASSW { lpfnWndProc: Some(frame_proc), hInstance: instance.into(), lpszClassName: w!("CursoraFillFrame"), hbrBackground: CreateSolidBrush(GREEN), ..Default::default() };
                RegisterClassW(&class);
            });
            let style = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
            let hwnd = CreateWindowExW(style, w!("CursoraFillFrame"), None, WS_POPUP, 0, 0, 0, 0, None, None, Some(instance.into()), None).ok()?;
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);
            Some(Frame(hwnd))
        }
    }

    /// Around `r`, in physical screen pixels like UI Automation's rectangles.
    fn around(&self, r: RECT) {
        if r.right <= r.left || r.bottom <= r.top {
            return; // off screen or not laid out: no frame rather than a wrong one
        }
        let edge = GAP + THICK;
        let (w, h) = (r.right - r.left + 2 * edge, r.bottom - r.top + 2 * edge);
        unsafe {
            let outer = CreateRectRgn(0, 0, w, h);
            let inner = CreateRectRgn(THICK, THICK, w - THICK, h - THICK);
            CombineRgn(Some(outer), Some(outer), Some(inner), RGN_DIFF);
            let _ = DeleteObject(inner.into());
            SetWindowRgn(self.0, Some(outer), true); // the window owns the region from now on
            let _ = SetWindowPos(self.0, Some(HWND_TOPMOST), r.left - edge, r.top - edge, w, h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        }
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        let _ = unsafe { DestroyWindow(self.0) };
    }
}

unsafe extern "system" fn frame_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fil_2_2_a_box_the_user_cant_see_is_left_out() {
        let r = |left, top, right, bottom| RECT { left, top, right, bottom };
        assert!(visible(r(100, 100, 400, 130), 0));
        assert!(visible(r(100, 2000, 400, 2030), 0), "below the fold: scrolled to when focused");
        assert!(visible(r(-50, 100, 250, 130), 0), "partly in the window");
        assert!(!visible(r(100, 100, 101, 101), 0), "a pixel");
        assert!(!visible(r(100, 100, 400, 100), 0), "no height");
        assert!(!visible(r(-9999, 100, -9699, 130), 0), "moved far to the left");
        assert!(!visible(r(500, 100, 800, 130), 900), "left of a window that starts at 900");
    }
}
