use super::{Clip, CopyKey, Platform, SourceApp};
use crate::session::{self, Session};
use crate::x11::{self, Item, R, X11};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::res::{ClientIdMask, ClientIdSpec, ConnectionExt as _};
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, KEY_PRESS_EVENT, KEY_RELEASE_EVENT};
use x11rb::protocol::xtest::ConnectionExt as _;
use x11rb::NONE;

/// How long a selection's owner gets to answer one read.
const READ_MS: u64 = 100;
/// For the whole clipboard snapshot, every target included.
const SNAPSHOT_MS: u64 = 300;
/// `wl-paste` is killed after this (LNX-3.4).
const WL_PASTE_MS: u64 = 300;
const XK_CONTROL_L: u32 = 0xffe3;
const XK_CONTROL_R: u32 = 0xffe4;
const XK_SHIFT_L: u32 = 0xffe1;
const XK_INSERT: u32 = 0xff63;
const XK_C: u32 = 0x63;

/// X11: everything through the X server. Wayland: only `wl-paste`, no keys and no clipboard (spec 08).
pub struct LinuxPlatform {
    x11: Option<X11>,
    wl_paste: bool,
}

impl LinuxPlatform {
    pub fn new() -> Self {
        match session::current() {
            Session::X11 => LinuxPlatform { x11: X11::connect().map_err(|e| log::warn!("X11: {e}")).ok(), wl_paste: false },
            Session::Wayland => LinuxPlatform { x11: None, wl_paste: on_path("wl-paste") },
            _ => LinuxPlatform { x11: None, wl_paste: false },
        }
    }
}

impl Platform for LinuxPlatform {
    type Snapshot = Vec<Item>;

    fn foreground(&self) -> SourceApp {
        let Some(x) = &self.x11 else { return SourceApp::default() }; // Wayland can't tell (LNX-4.3)
        source_app(x).unwrap_or_else(|e| {
            log::warn!("X11 foreground: {e}");
            SourceApp::default()
        })
    }

    /// PRIMARY: what is selected right now, without any key (LNX-3.1, 3.4).
    fn accessible_selection(&self, max_chars: usize) -> Option<String> {
        match &self.x11 {
            Some(x) => x.convert(AtomEnum::PRIMARY.into(), x.atoms.UTF8_STRING, deadline(READ_MS)).ok()?.map(|i| String::from_utf8_lossy(&i.data).into_owned()),
            None if self.wl_paste => wl_paste(max_chars),
            None => None,
        }
    }

    /// While a key grab is active, X sends every key to the grabbing app, so a copy sent while the hotkey's
    /// own key is down would go to Cursora. So every key but Ctrl must be up (SEL-3.2).
    fn extra_modifiers_released(&self, wait: Duration) -> bool {
        let Some(x) = &self.x11 else { return false }; // Wayland: never simulate keys
        let ctrl: Vec<u8> = [XK_CONTROL_L, XK_CONTROL_R].iter().filter_map(|&k| x.keycode(k).ok().flatten()).collect();
        let deadline = Instant::now() + wait;
        loop {
            let Some(keys) = x.conn.query_keymap().ok().and_then(|c| c.reply().ok()) else { return false };
            let down = (8..=255u8).any(|code| keys.keys[code as usize / 8] & (1 << (code % 8)) != 0 && !ctrl.contains(&code));
            if !down {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn clipboard_seq(&self) -> u32 {
        self.x11.as_ref().map_or(0, X11::owner_changes)
    }

    /// Every target, byte for byte. None if the owner doesn't answer in time or an item is too big to hand
    /// back in one request, so the clipboard is then left alone (SEL-3.1).
    fn clipboard_snapshot(&self) -> Option<Vec<Item>> {
        let x = self.x11.as_ref()?;
        let (clipboard, a) = (x.atoms.CLIPBOARD, &x.atoms);
        if x.conn.get_selection_owner(clipboard).ok()?.reply().ok()?.owner == NONE {
            return Some(Vec::new());
        }
        let deadline = deadline(SNAPSHOT_MS);
        let targets = x.convert(clipboard, a.TARGETS, deadline).ok()??;
        let skip = [a.TARGETS, a.MULTIPLE, a.TIMESTAMP, a.SAVE_TARGETS, a.DELETE];
        // shortcut: no INCR when handing the clipboard back, so an item bigger than one request (16 MB with BIG-REQUESTS) stops the copy; upgrade if users copy huge images and then ask about files.
        let max = x.conn.maximum_request_bytes() - 64;
        let mut items = Vec::new();
        let offered = targets.data.chunks_exact(4).map(|a| u32::from_ne_bytes([a[0], a[1], a[2], a[3]]));
        for target in offered.filter(|t| targets.format == 32 && !skip.contains(t)) {
            match x.convert(clipboard, target, deadline).ok()? {
                Some(item) if item.data.len() > max => return None,
                Some(item) => items.push(item),
                None => {} // the owner offers it but won't give it, so no app could paste it either
            }
        }
        Some(items)
    }

    fn clipboard_restore(&self, snapshot: Vec<Item>) {
        if let Some(x) = &self.x11 {
            x11::serve(x.atoms.CLIPBOARD, snapshot);
        }
    }

    /// Files first: a file manager offers their names as text too (LNX-3.2).
    fn clipboard_read(&self) -> Clip {
        let Some(x) = &self.x11 else { return Clip::Nothing };
        let (clipboard, deadline) = (x.atoms.CLIPBOARD, deadline(READ_MS));
        for target in [x.atoms.URI_LIST, x.atoms.GNOME_FILES] {
            if let Ok(Some(item)) = x.convert(clipboard, target, deadline) {
                let paths = uri_paths(&String::from_utf8_lossy(&item.data));
                if !paths.is_empty() {
                    return Clip::Files(paths);
                }
            }
        }
        match x.convert(clipboard, x.atoms.UTF8_STRING, deadline) {
            Ok(Some(item)) => Clip::Text(String::from_utf8_lossy(&item.data).into_owned()),
            _ => Clip::Nothing,
        }
    }

    fn send_copy(&self, key: CopyKey) {
        let Some(x) = &self.x11 else { return };
        let keysyms: &[u32] = match key {
            CopyKey::CtrlC => &[XK_CONTROL_L, XK_C],
            CopyKey::CtrlInsert => &[XK_CONTROL_L, XK_INSERT],
            CopyKey::CtrlShiftC => &[XK_CONTROL_L, XK_SHIFT_L, XK_C],
            CopyKey::Never => return,
        };
        let Some(codes) = keysyms.iter().map(|&k| x.keycode(k).ok().flatten()).collect::<Option<Vec<u8>>>() else {
            return log::warn!("X11: the keyboard has no key for {key:?}");
        };
        let presses = codes.iter().map(|&c| (KEY_PRESS_EVENT, c));
        let releases = codes.iter().rev().map(|&c| (KEY_RELEASE_EVENT, c));
        let sent = presses.chain(releases).try_for_each(|(kind, code)| x.conn.xtest_fake_input(kind, code, x11rb::CURRENT_TIME, x.root, 0, 0, 0).map(drop));
        if let Err(e) = sent.and_then(|_| x.conn.flush()) {
            log::warn!("X11 copy key: {e}");
        }
    }

    fn primary(&self) -> bool {
        true
    }

    /// PRIMARY's owner, by process: its window is a hidden one with no title (LNX-3.5).
    // shortcut: a title rule can't match it, and a clipboard manager that takes PRIMARY over hides the app it came from; upgrade by remembering the window in front when the owner changes, if users block web pages by title.
    fn selection_owner(&self) -> Option<SourceApp> {
        let x = self.x11.as_ref()?;
        let owner = x.conn.get_selection_owner(AtomEnum::PRIMARY.into()).ok()?.reply().ok()?.owner;
        if owner == NONE {
            return None;
        }
        let exe = process_name(pid(x, owner).ok()??)?;
        Some(SourceApp { name: exe.clone(), exe, ..SourceApp::default() })
    }

    fn selection_readable(&self) -> bool {
        self.x11.is_some() || self.wl_paste
    }
}

fn deadline(ms: u64) -> Instant {
    Instant::now() + Duration::from_millis(ms)
}

/// `_NET_ACTIVE_WINDOW`, then the process behind it, and its title (LNX-4.3).
fn source_app(x: &X11) -> R<SourceApp> {
    let window = x.active_window()?;
    if window == NONE {
        return Ok(SourceApp::default());
    }
    let pid = pid(x, window)?;
    let text = |name: u32, type_: AtomEnum| -> R<Option<String>> { Ok(x.property(window, name, type_)?.map(|p| String::from_utf8_lossy(&p.value).into_owned())) };
    let title = match text(x.atoms._NET_WM_NAME, AtomEnum::ANY)? {
        Some(t) => t,
        None => text(AtomEnum::WM_NAME.into(), AtomEnum::ANY)?.unwrap_or_default(),
    };
    let class = x.property(window, AtomEnum::WM_CLASS, AtomEnum::STRING)?.map(|p| p.value).unwrap_or_default();
    Ok(app(pid.and_then(process_name), &class, title, window))
}

/// The pid behind a window. The X server knows it for every local client; `_NET_WM_PID` is the app's own
/// claim, and inside a Flatpak sandbox it is a pid of the sandbox's own numbering.
fn pid(x: &X11, window: u32) -> R<Option<u32>> {
    let spec = ClientIdSpec { client: window, mask: ClientIdMask::LOCAL_CLIENT_PID };
    let server_pid = x.conn.res_query_client_ids(&[spec]).ok().and_then(|c| c.reply().ok()).and_then(|r| r.ids.first()?.value.first().copied());
    match server_pid {
        Some(pid) => Ok(Some(pid)),
        None => x.value32(window, x.atoms._NET_WM_PID, AtomEnum::CARDINAL),
    }
}

/// The process name, else the first part of `WM_CLASS` (a window from a remote or very old client).
fn app(process: Option<String>, wm_class: &[u8], title: String, window: u32) -> SourceApp {
    let exe = process.unwrap_or_else(|| String::from_utf8_lossy(wm_class.split(|&b| b == 0).next().unwrap_or_default()).into_owned());
    SourceApp { name: exe.clone(), exe, title, elevated: false, window: window as isize }
}

/// The file name of the process's program, or its short name where that can't be read (another user's process).
fn process_name(pid: u32) -> Option<String> {
    let exe = std::fs::read_link(format!("/proc/{pid}/exe")).ok().and_then(|p| Some(p.file_name()?.to_string_lossy().trim_end_matches(" (deleted)").to_owned()));
    exe.or_else(|| Some(std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?.trim_end().to_owned()))
}

/// `file://` URIs from `text/uri-list` or `x-special/gnome-copied-files`, as paths. Comments, the copy/cut line,
/// other schemes and files on other hosts are skipped.
pub fn uri_paths(list: &str) -> Vec<PathBuf> {
    list.lines().filter_map(|line| url::Url::parse(line.trim()).ok()?.to_file_path().ok()).collect()
}

fn on_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()))
}

/// The Wayland PRIMARY selection, through `wl-paste`. Killed after `WL_PASTE_MS`; at most `max_chars` × 4 bytes are read.
fn wl_paste(max_chars: usize) -> Option<String> {
    let mut child = Command::new("wl-paste")
        .args(["--primary", "--no-newline", "--type", "text"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| log::warn!("wl-paste: {e}"))
        .ok()?;
    let stdout = child.stdout.take()?;
    let limit = (max_chars as u64).saturating_mul(4);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.take(limit).read_to_end(&mut bytes);
        let _ = tx.send(bytes);
    });
    let bytes = rx.recv_timeout(Duration::from_millis(WL_PASTE_MS)).ok();
    let _ = child.kill(); // also ends a wl-paste still waiting to write past the limit
    let _ = child.wait();
    bytes.map(|b| String::from_utf8_lossy(&b).into_owned()).filter(|t| !t.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use x11rb::protocol::xproto::{CreateWindowAux, PropMode, WindowClass};
    use x11rb::wrapper::ConnectionExt as _;

    /// These tests take over selections and press keys, so they run only on an X server of their own.
    fn test_display() -> LinuxPlatform {
        assert!(
            std::env::var_os("CURSORA_XVFB").is_some(),
            "the X11 tests take over the clipboard and press keys: run them on their own X server (scripts/wsl-test.sh, or CURSORA_XVFB=1 xvfb-run -a cargo test)"
        );
        LinuxPlatform { x11: Some(X11::connect().expect("an X server")), wl_paste: false }
    }

    fn eventually(what: &str, mut ok: impl FnMut() -> bool) {
        let end = deadline(2000);
        while !ok() {
            assert!(Instant::now() < end, "timed out: {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Text offered as `target`, typed as the target itself, as GTK does for UTF8_STRING and text/uri-list.
    fn text(target: u32, text: &str) -> Item {
        Item { target, type_: target, format: 8, data: text.as_bytes().to_vec() }
    }

    #[test]
    fn lnx_3_1_primary_is_read_from_a_real_x_server() {
        let p = test_display();
        let x = p.x11.as_ref().unwrap();
        x11::serve(AtomEnum::PRIMARY.into(), vec![text(x.atoms.UTF8_STRING, "héllo wörld")]);
        eventually("PRIMARY", || p.accessible_selection(100).as_deref() == Some("héllo wörld"));
    }

    #[test]
    fn lnx_3_5_the_primary_owner_on_a_real_x_server() {
        let p = test_display();
        let x = p.x11.as_ref().unwrap();
        // The same text as LNX-3.1's test, which may run at the same time on this X server.
        x11::serve(AtomEnum::PRIMARY.into(), vec![text(x.atoms.UTF8_STRING, "héllo wörld")]);
        let me = std::env::current_exe().unwrap().file_name().unwrap().to_string_lossy().into_owned();
        eventually("the owner", || p.selection_owner().is_some_and(|o| o.exe == me));
    }

    #[test]
    fn lnx_3_2_files_on_the_clipboard_and_a_byte_exact_restore_on_a_real_x_server() {
        let p = test_display();
        let x = p.x11.as_ref().unwrap();
        let before = p.clipboard_seq();
        let copied = vec![text(x.atoms.URI_LIST, "# from a test\r\nfile:///tmp/a%20b.txt\r\nfile:///tmp/c\r\n"), text(x.atoms.UTF8_STRING, "a b.txt\nc")];
        x11::serve(x.atoms.CLIPBOARD, copied.clone());
        eventually("the files", || p.clipboard_read() == Clip::Files(vec!["/tmp/a b.txt".into(), "/tmp/c".into()]));
        assert_ne!(p.clipboard_seq(), before, "a new owner moves the sequence number");
        let snapshot = p.clipboard_snapshot().expect("a snapshot");
        assert_eq!(snapshot, copied);
        x11::serve(x.atoms.CLIPBOARD, vec![text(x.atoms.UTF8_STRING, "something else")]); // another app copies
        eventually("the other copy", || p.clipboard_read() == Clip::Text("something else".into()));
        p.clipboard_restore(snapshot.clone());
        eventually("the restore", || p.clipboard_snapshot().as_ref() == Some(&snapshot));
        p.clipboard_restore(Vec::new()); // an empty clipboard before: no owner again
        eventually("no owner", || p.clipboard_snapshot() == Some(Vec::new()));
    }

    #[test]
    fn lnx_3_2_uri_lists_become_paths() {
        let gnome = "copy\nfile:///home/u/My%20Docs/r%C3%A9sum%C3%A9.pdf\nfile:///home/u/dir";
        assert_eq!(uri_paths(gnome), [PathBuf::from("/home/u/My Docs/résumé.pdf"), PathBuf::from("/home/u/dir")]);
        let list = "# comment\r\nfile:///a\r\nhttps://example.com/x\r\nfile://otherhost/b\r\n\r\n";
        assert_eq!(uri_paths(list), [PathBuf::from("/a")]);
    }

    #[test]
    fn lnx_4_3_the_app_in_front_on_a_real_x_server() {
        let p = test_display();
        let x = p.x11.as_ref().unwrap();
        // A window of this test process, made the active one the way a window manager would.
        let window = x.conn.generate_id().unwrap();
        x.conn.create_window(0, window, x.root, 0, 0, 1, 1, 0, WindowClass::INPUT_ONLY, 0, &CreateWindowAux::new()).unwrap();
        x.conn.change_property8(PropMode::REPLACE, window, x.atoms._NET_WM_NAME, x.atoms.UTF8_STRING, "Report – draft".as_bytes()).unwrap();
        x.conn.change_property8(PropMode::REPLACE, window, AtomEnum::WM_CLASS, AtomEnum::STRING, b"tester\0Tester\0").unwrap();
        x.conn.change_property32(PropMode::REPLACE, x.root, x.atoms._NET_ACTIVE_WINDOW, AtomEnum::WINDOW, &[window]).unwrap();
        x.conn.flush().unwrap();
        let me = std::env::current_exe().unwrap().file_name().unwrap().to_string_lossy().into_owned();
        let app = p.foreground();
        assert_eq!((app.exe.as_str(), app.name.as_str(), app.title.as_str(), app.window), (me.as_str(), me.as_str(), "Report – draft", window as isize));
        // Without a process, the WM_CLASS name.
        assert_eq!(super::app(None, b"tester\0Tester\0", String::new(), 1).exe, "tester");
        // Wayland: nothing to ask.
        assert_eq!(LinuxPlatform { x11: None, wl_paste: true }.foreground(), SourceApp::default());
    }

    #[test]
    fn sel_3_2_on_x11_every_key_but_ctrl_must_be_up() {
        let p = test_display();
        let x = p.x11.as_ref().unwrap();
        let press = |keysym, down| {
            let code = x.keycode(keysym).unwrap().unwrap();
            x.conn.xtest_fake_input(if down { KEY_PRESS_EVENT } else { KEY_RELEASE_EVENT }, code, 0, x.root, 0, 0, 0).unwrap();
            x.conn.flush().unwrap();
        };
        press(XK_CONTROL_L, true);
        assert!(p.extra_modifiers_released(Duration::from_millis(50)), "Ctrl alone may stay down");
        press(XK_C, true); // the hotkey's own key, still held
        assert!(!p.extra_modifiers_released(Duration::from_millis(50)));
        press(XK_C, false);
        press(XK_CONTROL_L, false);
        assert!(p.extra_modifiers_released(Duration::from_millis(50)));
    }
}
