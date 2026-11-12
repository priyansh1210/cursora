//! Spec 27 on Windows: UI Automation reads the form and focuses its boxes, `SendInput` types, and a Win32 window draws the frame.
use super::{Field, Fit, Kind, Target};
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
    CUIAutomation8, ExpandCollapseState_Expanded, IUIAutomation2, IUIAutomationCacheRequest, IUIAutomationCondition, IUIAutomationElement, IUIAutomationExpandCollapsePattern,
    IUIAutomationInvokePattern, IUIAutomationRangeValuePattern, IUIAutomationSelectionItemPattern, IUIAutomationSelectionPattern, IUIAutomationTextPattern, IUIAutomationTogglePattern,
    IUIAutomationTreeWalker, IUIAutomationValuePattern, ToggleState_On, TreeScope_Children, TreeScope_Descendants, UIA_BoundingRectanglePropertyId, UIA_CheckBoxControlTypeId,
    UIA_ComboBoxControlTypeId, UIA_ControlTypePropertyId, UIA_DocumentControlTypeId, UIA_EditControlTypeId, UIA_ExpandCollapsePatternId, UIA_GroupControlTypeId, UIA_HelpTextPropertyId,
    UIA_InvokePatternId, UIA_IsEnabledPropertyId, UIA_IsKeyboardFocusablePropertyId, UIA_IsPasswordPropertyId, UIA_ListControlTypeId, UIA_ListItemControlTypeId, UIA_NamePropertyId,
    UIA_RadioButtonControlTypeId, UIA_RangeValuePatternId, UIA_SelectionItemIsSelectedPropertyId, UIA_SelectionItemPatternId, UIA_SelectionPatternId, UIA_SpinnerControlTypeId,
    UIA_TextControlTypeId, UIA_TextPatternId, UIA_TogglePatternId, UIA_ToggleToggleStatePropertyId, UIA_ValueIsReadOnlyPropertyId, UIA_ValuePatternId, UIA_ValueValuePropertyId,
    UIA_CONTROLTYPE_ID,
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

/// Returns how many boxes were filled, the labels of those that couldn't be, and why it stopped early, if it did.
pub fn type_into(window: isize, targets: &[Target], before: impl FnOnce()) -> Result<(usize, Vec<String>, Option<&'static str>), String> {
    let _com = Com::init();
    let uia = automation()?;
    let found = boxes(&uia, window)?;
    // The page may have changed since the plan: only a box at the same place, with the same label and still empty, is filled.
    let todo: Vec<(&Target, &Found)> = targets.iter().filter_map(|t| found.get(t.index).filter(|(f, _)| !f.filled && f.name() == t.field).map(|b| (t, b))).collect();
    if todo.is_empty() {
        return Err("The form changed since Cursora read it, so nothing was filled. Ask /fill again.".into());
    }
    if !modifiers_up() {
        return Err("Ctrl, Shift, Alt or Windows was still held down, so nothing was filled.".into());
    }
    before();
    let _esc = EscHook::new();
    let frame = Frame::new();
    let show = |el: &IUIAutomationElement| {
        if let (Some(f), Ok(r)) = (&frame, unsafe { el.CurrentBoundingRectangle() }) {
            f.around(r);
        }
    };
    let (mut done, mut missed) = (0, Vec::new());
    for (t, (f, els)) in todo {
        if let Some(why) = stop(window) {
            return Ok((done, missed, Some(why)));
        }
        let filled = match super::fit(f, &t.value) {
            None => false,
            // A date box's digits go into its first part, which moves on by itself (FIL-3.5).
            Some(Fit::Type(text)) => {
                if focus(&uia, els.last().unwrap()) {
                    show(&els[0]);
                    if let Some(why) = type_text(window, &text) {
                        return Ok((done, missed, Some(why)));
                    }
                    true
                } else {
                    false
                }
            }
            Some(Fit::Find(value)) => {
                show(&els[0]);
                unsafe { pick_from_list(&uia, window, &els[0], &value) }.unwrap_or(false)
            }
            Some(Fit::Pick(options)) if f.kind == Kind::List => {
                show(&els[0]);
                unsafe { pick_from_list(&uia, window, &els[0], &options[0]) }.unwrap_or(false)
            }
            Some(Fit::Pick(options)) => options.iter().all(|o| {
                els.iter().find(|el| unsafe { el.CurrentName() }.is_ok_and(|n| n.to_string() == *o)).is_some_and(|el| {
                    show(el);
                    let done = unsafe { press(el, f.kind == Kind::Checks) }.unwrap_or(false);
                    wait(BOX_MS);
                    done
                })
            }),
        };
        if filled {
            done += 1;
        } else {
            missed.push(t.field.clone());
        }
        wait(BOX_MS);
    }
    log::info!("fill: {done} boxes filled, {} not", missed.len()); // counts only (APP-7.2)
    Ok((done, missed, None))
}

/// Types the text letter by letter; returns why it stopped, if it did (FIL-3.3).
fn type_text(window: isize, text: &str) -> Option<&'static str> {
    for unit in text.encode_utf16() {
        if let Some(why) = stop(window) {
            return Some(why);
        }
        letter(unit);
        wait(LETTER_MS);
    }
    None
}

/// Selects a radio button, or ticks a check box (never unticks one), and waits for the page to say so (FIL-3.5).
unsafe fn press(el: &IUIAutomationElement, check: bool) -> windows::core::Result<bool> {
    unsafe {
        if check {
            let toggle = el.GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId)?;
            if toggle.CurrentToggleState()? != ToggleState_On {
                toggle.Toggle()?;
            }
            Ok(until(|| toggle.CurrentToggleState().is_ok_and(|s| s == ToggleState_On)))
        } else {
            let item = el.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(UIA_SelectionItemPatternId)?;
            item.Select()?;
            Ok(until(|| item.CurrentIsSelected().is_ok_and(|s| s.as_bool())))
        }
    }
}

/// Opens the list, picks the option `value` names (FIL-2.10) and closes it (FIL-3.5). A page's `select` opens with
/// ExpandCollapse; a list without it (Google Forms') opens when its chosen item is clicked. Its options are under it,
/// else they are the ones that appeared in the window when it opened.
// shortcut: options shown where the window's tree doesn't have them (a canvas, another window) aren't found; read the popup's window too if a real form needs it.
unsafe fn pick_from_list(uia: &IUIAutomation2, window: isize, list: &IUIAutomationElement, value: &str) -> windows::core::Result<bool> {
    unsafe {
        let items = control(uia, UIA_ListItemControlTypeId)?;
        let root = uia.ElementFromHandle(HWND(window as *mut _))?;
        let shown = |under: &IUIAutomationElement| -> windows::core::Result<Vec<(String, RECT, IUIAutomationElement)>> {
            let all = under.FindAll(TreeScope_Descendants, &items)?;
            let mut out = vec![];
            for i in 0..all.Length()? {
                let el = all.GetElement(i)?;
                let rect = el.CurrentBoundingRectangle().unwrap_or_default();
                if visible(rect, i32::MIN) {
                    out.push((el.CurrentName().map(|n| n.to_string()).unwrap_or_default(), rect, el));
                }
            }
            Ok(out)
        };
        let before = shown(&root)?;
        let expand = list.GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(UIA_ExpandCollapsePatternId).ok();
        match &expand {
            Some(e) => e.Expand()?,
            None => list.FindFirst(TreeScope_Descendants, &items)?.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)?.Invoke()?,
        }
        wait(400);
        let mut options: Vec<_> = shown(list)?.into_iter().filter(|(n, _, _)| !super::placeholder(n)).collect();
        if options.is_empty() {
            options = shown(&root)?.into_iter().filter(|(n, r, _)| !super::placeholder(n) && !before.iter().any(|(m, q, _)| m == n && q == r)).collect();
        }
        let names: Vec<String> = options.iter().map(|(n, _, _)| n.clone()).collect();
        let picked = match super::choose(&names, value) {
            Some(i) => {
                let item = options[i].2.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(UIA_SelectionItemPatternId)?;
                item.Select()?;
                let name = &names[i];
                // A list that closes hides the option picked: its Value, the item it now shows, or the option gone (closed by
                // the pick) says it was picked.
                let el = &options[i].2;
                until(|| {
                    item.CurrentIsSelected().is_ok_and(|s| s.as_bool())
                        || el.CurrentBoundingRectangle().map_or(true, |r| !visible(r, i32::MIN))
                        || list.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId).and_then(|v| v.CurrentValue()).is_ok_and(|v| v.to_string() == *name)
                        || shown(list).is_ok_and(|now| now.iter().any(|(n, _, el)| n == name && el.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(UIA_SelectionItemPatternId).and_then(|s| s.CurrentIsSelected()).is_ok_and(|s| s.as_bool())))
                })
            }
            None => false,
        };
        if let Some(e) = &expand {
            if e.CurrentExpandCollapseState().is_ok_and(|s| s == ExpandCollapseState_Expanded) {
                let _ = e.Collapse();
            }
        }
        Ok(picked)
    }
}

/// Waits up to 500 ms for `done`, as a page updates a moment after a click.
fn until(done: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        if done() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        wait(20);
    }
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

/// A box as read, with its elements: the box (and, for a date box, its first part), or a question's buttons.
type Found = (Field, Vec<IUIAutomationElement>);

/// The boxes. Chrome builds its accessibility tree once a client asks, so a read that finds none is tried again.
fn boxes(uia: &IUIAutomation2, window: isize) -> Result<Vec<Found>, String> {
    let found = unsafe { find(uia, window) }.map_err(|e| format!("Cursora couldn't read the form ({}).", e.message()))?;
    if !found.is_empty() {
        return Ok(found);
    }
    std::thread::sleep(Duration::from_millis(500));
    unsafe { find(uia, window) }.map_err(|e| format!("Cursora couldn't read the form ({}).", e.message()))
}

/// The boxes of any kind (FIL-2.7), in the window's documents where it has any (a browser's page, not its address bar),
/// else in the whole window (a dialog box).
unsafe fn find(uia: &IUIAutomation2, window: isize) -> windows::core::Result<Vec<Found>> {
    unsafe {
        let root = uia.ElementFromHandle(HWND(window as *mut _))?;
        let cache = uia.CreateCacheRequest()?;
        for p in [
            UIA_NamePropertyId, UIA_HelpTextPropertyId, UIA_IsPasswordPropertyId, UIA_IsEnabledPropertyId, UIA_IsKeyboardFocusablePropertyId, UIA_BoundingRectanglePropertyId,
            UIA_ControlTypePropertyId, UIA_ValueValuePropertyId, UIA_ValueIsReadOnlyPropertyId, UIA_ToggleToggleStatePropertyId, UIA_SelectionItemIsSelectedPropertyId,
        ] {
            cache.AddProperty(p)?;
        }
        for p in [UIA_ValuePatternId, UIA_TogglePatternId, UIA_SelectionItemPatternId, UIA_SelectionPatternId, UIA_TextPatternId] {
            cache.AddPattern(p)?;
        }
        let mut kinds = control(uia, UIA_EditControlTypeId)?;
        for kind in [UIA_ComboBoxControlTypeId, UIA_ListControlTypeId, UIA_RadioButtonControlTypeId, UIA_CheckBoxControlTypeId] {
            kinds = uia.CreateOrCondition(&kinds, &control(uia, kind)?)?;
        }
        let docs = root.FindAll(TreeScope_Descendants, &control(uia, UIA_DocumentControlTypeId)?)?;
        let left = root.CurrentBoundingRectangle()?.left;
        let mut out = Vec::new();
        for i in 0..docs.Length()? {
            collect(uia, &docs.GetElement(i)?, &kinds, &cache, left, &mut out)?;
        }
        if out.is_empty() {
            collect(uia, &root, &kinds, &cache, left, &mut out)?;
        }
        Ok(out.into_iter().map(|b| (b.field, b.els)).collect())
    }
}

/// ControlType == `kind`, as a property condition (VT_I4).
unsafe fn control(uia: &IUIAutomation2, kind: UIA_CONTROLTYPE_ID) -> windows::core::Result<IUIAutomationCondition> {
    let value = VARIANT { Anonymous: VARIANT_0 { Anonymous: ManuallyDrop::new(VARIANT_0_0 { vt: VT_I4, Anonymous: VARIANT_0_0_0 { lVal: kind.0 }, ..Default::default() }) } };
    unsafe { uia.CreatePropertyCondition(UIA_ControlTypePropertyId, &value) }
}

/// A box being read: where it is, and a question's group, to add the next button to.
struct Read {
    field: Field,
    els: Vec<IUIAutomationElement>,
    rect: RECT,
    group: Option<IUIAutomationElement>,
}

/// Adds `root`'s boxes (FIL-2.7). A page inside a page (an iframe) is a document inside a document, so a box already
/// found, by its label and place on screen, is skipped. `left`: the window's left edge.
#[allow(non_upper_case_globals)] // the windows crate's control type ids, matched as constants
unsafe fn collect(uia: &IUIAutomation2, root: &IUIAutomationElement, kinds: &IUIAutomationCondition, cache: &IUIAutomationCacheRequest, left: i32, out: &mut Vec<Read>) -> windows::core::Result<()> {
    let on = |b: windows::core::Result<BOOL>| b.is_ok_and(|b| b.as_bool());
    let text = |s: windows::core::Result<BSTR>| s.map(|s| s.to_string()).unwrap_or_default();
    unsafe {
        let walker = uia.ControlViewWalker()?;
        let found = root.FindAllBuildCache(TreeScope_Descendants, kinds, cache)?;
        for i in 0..found.Length()? {
            let el = found.GetElement(i)?;
            let rect = el.CachedBoundingRectangle().unwrap_or_default();
            // A box the user can't see gets none of their details: a page can hide one (a pixel, or far to the left) to
            // collect more than it shows. A box below the fold still counts.
            if !on(el.CachedIsEnabled()) || !visible(rect, left) {
                continue;
            }
            let name = text(el.CachedName());
            let value = el.GetCachedPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId).ok();
            let writable = value.as_ref().is_some_and(|v| !on(v.CachedIsReadOnly())) && on(el.CachedIsKeyboardFocusable());
            let current = value.as_ref().map(|v| text(v.CachedValue())).unwrap_or_default();
            let field = Field { label: name.clone(), hint: text(el.CachedHelpText()), password: on(el.CachedIsPassword()), ..Field::default() };
            let ty = el.CachedControlType()?;
            let read = match ty {
                UIA_EditControlTypeId if writable => {
                    let (kind, first, dated) = date_parts(uia, &el)?;
                    Read { field: Field { kind, filled: dated || !current.trim().is_empty(), ..field }, els: [Some(el)].into_iter().chain([first]).flatten().collect(), rect, group: None }
                }
                UIA_ComboBoxControlTypeId => {
                    // A Win32 combo box's Edit is read as itself.
                    if el.FindFirst(TreeScope_Children, &control(uia, UIA_EditControlTypeId)?).is_ok() {
                        continue;
                    }
                    let typing = writable && el.GetCachedPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId).is_ok();
                    let (kind, filled) = if typing { (Kind::Text, !current.trim().is_empty()) } else { (Kind::List, !super::placeholder(&current)) };
                    Read { field: Field { kind, filled, ..field }, els: vec![el], rect, group: None }
                }
                UIA_ListControlTypeId => {
                    // A list of check boxes (Google Forms') has no Selection pattern: its boxes are read themselves.
                    if el.GetCachedPatternAs::<IUIAutomationSelectionPattern>(UIA_SelectionPatternId).is_err() {
                        continue;
                    }
                    let items = el.FindAllBuildCache(TreeScope_Descendants, &control(uia, UIA_ListItemControlTypeId)?, cache)?;
                    let (mut options, mut filled) = (vec![], false);
                    for j in 0..items.Length()? {
                        let item = items.GetElement(j)?;
                        let n = text(item.CachedName());
                        let chosen = item.GetCachedPatternAs::<IUIAutomationSelectionItemPattern>(UIA_SelectionItemPatternId).is_ok_and(|s| on(s.CachedIsSelected()));
                        filled |= chosen && !super::placeholder(&n);
                        if visible(item.CachedBoundingRectangle().unwrap_or_default(), left) {
                            options.push(n);
                        }
                    }
                    Read { field: Field { kind: Kind::List, filled, options, ..field }, els: vec![el], rect, group: None }
                }
                UIA_RadioButtonControlTypeId | UIA_CheckBoxControlTypeId => {
                    let (kind, chosen) = if ty == UIA_CheckBoxControlTypeId {
                        (Kind::Checks, el.GetCachedPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId).is_ok_and(|t| t.CachedToggleState().is_ok_and(|s| s == ToggleState_On)))
                    } else {
                        (Kind::Choice, el.GetCachedPatternAs::<IUIAutomationSelectionItemPattern>(UIA_SelectionItemPatternId).is_ok_and(|s| on(s.CachedIsSelected())))
                    };
                    let (group, question) = group_of(&walker, &el);
                    // Buttons next to each other with the same group are one question.
                    if let Some(last) = out.last_mut().filter(|q| q.field.kind == kind && same(uia, q.group.as_ref(), group.as_ref())) {
                        last.field.options.push(name);
                        last.field.filled |= chosen;
                        last.els.push(el);
                        continue;
                    }
                    // Without a named group, the question is the text just before its first button, else the button's own name
                    // (a check box on its own, "I agree to the terms").
                    let before = || walker.GetPreviousSiblingElement(&el).ok().filter(|p| p.CurrentControlType().is_ok_and(|t| t == UIA_TextControlTypeId)).map(|p| text(p.CurrentName()));
                    let label = question.or_else(before).unwrap_or_else(|| name.clone());
                    Read { field: Field { label: label.trim().to_string(), hint: String::new(), kind, filled: chosen, options: vec![name], ..field }, els: vec![el], rect, group }
                }
                _ => continue,
            };
            if !out.iter().any(|b| b.rect == read.rect && b.field.label == read.field.label) {
                out.push(read);
            }
        }
    }
    Ok(())
}

/// Two groups are the same element.
fn same(uia: &IUIAutomation2, a: Option<&IUIAutomationElement>, b: Option<&IUIAutomationElement>) -> bool {
    matches!((a, b), (Some(a), Some(b)) if unsafe { uia.CompareElements(a, b) }.is_ok_and(|s| s.as_bool()))
}

/// A button's group: its nearest ancestor, up to 4 levels, that is a named Group or List (a fieldset, an ARIA radiogroup
/// or list), with that name; else its parent, without one.
unsafe fn group_of(walker: &IUIAutomationTreeWalker, el: &IUIAutomationElement) -> (Option<IUIAutomationElement>, Option<String>) {
    unsafe {
        let parent = walker.GetParentElement(el).ok();
        let mut at = parent.clone();
        for _ in 0..4 {
            let Some(node) = at else { break };
            let ty = node.CurrentControlType().unwrap_or_default();
            let name = node.CurrentName().map(|n| n.to_string()).unwrap_or_default();
            if (ty == UIA_GroupControlTypeId || ty == UIA_ListControlTypeId) && !name.trim().is_empty() {
                return (Some(node), Some(name));
            }
            if ty == UIA_DocumentControlTypeId {
                break;
            }
            at = walker.GetParentElement(&node).ok();
        }
        (parent, None)
    }
}

/// A date box (Chrome's and Edge's `input type=date`): parts whose maxima are 12, 31 and a year's, in the locale's order,
/// with the first part to type into, and whether it has a date (its Value is empty either way; an empty part is 0).
/// Other parts (a time box) can't be filled; no parts is a text box.
unsafe fn date_parts(uia: &IUIAutomation2, el: &IUIAutomationElement) -> windows::core::Result<(Kind, Option<IUIAutomationElement>, bool)> {
    unsafe {
        let parts = el.FindAll(TreeScope_Children, &control(uia, UIA_SpinnerControlTypeId)?)?;
        if parts.Length()? == 0 {
            return Ok((Kind::Text, None, false));
        }
        let (mut order, mut dated) = (String::new(), false);
        for i in 0..parts.Length()? {
            let range = parts.GetElement(i)?.GetCurrentPatternAs::<IUIAutomationRangeValuePattern>(UIA_RangeValuePatternId);
            let max = range.as_ref().map_or(Ok(0.0), |r| r.CurrentMaximum()).unwrap_or_default();
            dated |= range.and_then(|r| r.CurrentValue()).is_ok_and(|v| v > 0.0);
            order.push(match max as u32 {
                12 => 'm',
                31 => 'd',
                m if m > 31 => 'y',
                _ => '?',
            });
        }
        let mut sorted: Vec<char> = order.chars().collect();
        sorted.sort();
        Ok(if sorted == ['d', 'm', 'y'] { (Kind::Date(order), Some(parts.GetElement(0)?), dated) } else { (Kind::Other, None, dated) })
    }
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

    /// Reads and fills a test page in Chrome titled "Fill probe", with made-up values: bring it to the front, then
    /// `cargo test probe -- --ignored --nocapture`. Prints what it read and what it filled.
    #[test]
    #[ignore]
    fn probe() {
        let title: Vec<u16> = "Fill probe - Google Chrome\0".encode_utf16().collect();
        let window = unsafe { windows::Win32::UI::WindowsAndMessaging::FindWindowW(None, windows::core::PCWSTR(title.as_ptr())) }.expect("the Fill probe page in Chrome").0 as isize;
        let fields = read(window).unwrap();
        fields.iter().enumerate().for_each(|(i, f)| println!("{i}: {f:?}"));
        let value = |label: &str| match label {
            "First name" => "Priya",
            "City" => "Bengaluru",
            "Country" => "Nepal",
            "Gender" => "F",
            "Year of study" => "Second year",
            "Languages you know" => "English, Hindi",
            "Date of birth" => "15/01/2000",
            "State" => "Kerala",
            "Blood group" => "O+",
            "Department" => "ECE",
            "Hobbies" => "Music",
            _ => "",
        };
        let targets: Vec<Target> = fields.iter().enumerate().filter(|(_, f)| !value(&f.name()).is_empty()).map(|(index, f)| Target { index, field: f.name(), value: value(&f.name()).into() }).collect();
        println!("{:?}", type_into(window, &targets, || {}));
    }
}
