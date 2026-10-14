//! Selection capture (spec 02). Every decision lives here; the OS is reached only through `Platform`.
pub mod files;
#[cfg(not(windows))]
mod linux;
#[cfg(windows)]
mod windows;

pub use files::{FileContext, Kind};
#[cfg(not(windows))]
pub use linux::{wl_paste, LinuxPlatform as OsPlatform};
#[cfg(windows)]
pub use self::windows::WinPlatform as OsPlatform;

use crate::chat::Image;
use crate::session::Session;
use crate::settings::CaptureSettings;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CopyKey {
    CtrlC,
    CtrlInsert,
    CtrlShiftC,
    Never,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct SourceApp {
    pub exe: String,
    pub name: String,
    pub title: String,
    /// The target runs as admin and Cursora does not, so Windows blocks reading it (SEL-6.1).
    pub elevated: bool,
    /// The app's window handle, so closing the bubble can give it focus back (BUB-2.3). Never sent to the UI.
    #[serde(skip)]
    pub window: isize,
}

impl SourceApp {
    /// How the prompt may name the source: never the window title unless the user allows it (SEL-7.2).
    pub fn for_prompt(&self, share_title: bool) -> String {
        if share_title && !self.title.is_empty() { format!("{} ({})", self.name, self.title) } else { self.name.clone() }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Method {
    /// Read without keys: UI Automation on Windows, PRIMARY on Linux.
    Direct,
    Copy,
    None,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Content {
    Text { text: String, truncated: bool },
    Files(FileContext),
    /// A screen region (spec 10).
    Snip(Snip),
    None,
}

impl Content {
    /// The images the model gets with the context.
    pub fn images(&self) -> Vec<Image> {
        match self {
            Content::Snip(s) => vec![s.full.clone().unwrap_or_else(|| s.thumbnail.clone())],
            Content::Files(f) => f.images.clone(),
            _ => vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Snip {
    /// At most 256 px on the long side: what the page shows and history keeps (VIS-2.6).
    pub thumbnail: Image,
    /// The size of the image the model gets.
    pub width: u32,
    pub height: u32,
    /// That image. Never serialized, so neither the page nor history ever has it (VIS-2.6); a conversation
    /// reopened from history has only the thumbnail, which its follow-ups send.
    #[serde(skip)]
    pub full: Option<Image>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Notice {
    /// A blocklist rule matched, so nothing was read (PRI-2.1).
    Blocked,
    Elevated,
    NoSelection {
        #[serde(rename = "offerSnip")]
        offer_snip: bool,
    },
    /// This session can't read any selection: Wayland without wl-paste (LNX-3.4).
    Unreadable,
    /// A snip needs a model that can see (VIS-1.2); `offer` is another configured one that can, for this snip only.
    NeedsVision { offer: Option<VisionOffer> },
    /// The screen couldn't be grabbed or the region prepared (spec 10).
    SnipFailed { reason: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VisionOffer {
    pub model: crate::llm::ModelRef,
    /// "llava-1.6 (LM Studio)"
    pub label: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Capture {
    pub app: SourceApp,
    pub method: Method,
    pub content: Content,
    pub notice: Option<Notice>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Clip {
    Text(String),
    Files(Vec<PathBuf>),
    Nothing,
}

pub trait Platform {
    type Snapshot;
    fn foreground(&self) -> SourceApp;
    fn accessible_selection(&self, max_chars: usize) -> Option<String>;
    /// True once Shift, Alt and Win are up; false if still held after `wait` (SEL-3.2).
    fn extra_modifiers_released(&self, wait: Duration) -> bool;
    fn clipboard_seq(&self) -> u32;
    /// None means the clipboard can't be saved exactly right now, so it must not be touched (SEL-3.1).
    fn clipboard_snapshot(&self) -> Option<Self::Snapshot>;
    fn clipboard_restore(&self, snapshot: Self::Snapshot);
    fn clipboard_read(&self) -> Clip;
    fn send_copy(&self, key: CopyKey);
    /// The selection is read from PRIMARY (Linux), so keys are only ever sent to file managers (LNX-3.1 to 3.3).
    fn primary(&self) -> bool {
        false
    }
    /// The app that owns the selection `accessible_selection` reads, where that can be another app than the
    /// one in front (X11's PRIMARY is shared by every app). None where it can't be told (LNX-3.5).
    fn selection_owner(&self) -> Option<SourceApp> {
        None
    }
    /// False when this session can't read any selection (LNX-3.4).
    fn selection_readable(&self) -> bool {
        true
    }
}

/// Linux file managers, by process name: their selection is files, copied with Ctrl+C (LNX-3.2).
pub const FILE_MANAGERS: [&str; 7] = ["nautilus", "dolphin", "thunar", "nemo", "caja", "pcmanfm", "pcmanfm-qt"];

/// `vision`: the active model reads images, so an empty capture can offer a snip (SEL-6.2).
pub fn capture<P: Platform>(p: &P, cfg: &CaptureSettings, vision: bool) -> Capture {
    let mut app = p.foreground(); // before any Cursora window can take focus (SEL-2.1)
    if blocked(&cfg.blocklist, &app) {
        app.title.clear(); // not shown, not saved
        return Capture { app, method: Method::None, content: Content::None, notice: Some(Notice::Blocked) };
    }
    if app.elevated {
        return Capture { app, method: Method::None, content: Content::None, notice: Some(Notice::Elevated) };
    }
    // PRIMARY in a file manager is text selected earlier somewhere else, not the files. PRIMARY owned by a
    // blocked app is that app's text, whichever app is in front (LNX-3.5).
    let skip_direct = p.primary() && (file_manager(&app.exe) || p.selection_owner().is_some_and(|owner| blocked(&cfg.blocklist, &owner)));
    let direct = if skip_direct { None } else { p.accessible_selection(cfg.max_chars).filter(|t| !t.trim().is_empty()) };
    let (method, content) = match direct {
        Some(text) => (Method::Direct, text_content(text, cfg.max_chars)),
        None => match copy_selection(p, copy_key(&app.exe, cfg, p.primary()), cfg.copy_timeout_ms) {
            Clip::Text(text) if !text.trim().is_empty() => (Method::Copy, text_content(text, cfg.max_chars)),
            Clip::Files(paths) if !paths.is_empty() => (Method::Copy, Content::Files(files::file_context(&paths, cfg))),
            _ => (Method::None, Content::None),
        },
    };
    let notice = matches!(content, Content::None).then(|| if p.selection_readable() { Notice::NoSelection { offer_snip: vision } } else { Notice::Unreadable });
    Capture { app, method, content, notice }
}

/// A blocklist rule (PRI-2.2): a process name, whole and ignoring case, or a keyword anywhere in the window title, ignoring case.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Rule {
    Process(String),
    Title(String),
}

pub fn blocked(rules: &[Rule], app: &SourceApp) -> bool {
    let (exe, title) = (app.exe.to_lowercase(), app.title.to_lowercase());
    rules.iter().any(|r| match r {
        Rule::Process(p) => !p.trim().is_empty() && p.trim().to_lowercase() == exe,
        Rule::Title(k) => !k.trim().is_empty() && title.contains(&k.trim().to_lowercase()),
    })
}

/// Whether this session can tell which app is in front, so the blocklist can be enforced (PRI-2.5, LNX-4.3).
pub fn identifies_apps(session: Session) -> bool {
    matches!(session, Session::Windows | Session::X11)
}

fn file_manager(exe: &str) -> bool {
    FILE_MANAGERS.iter().any(|f| f.eq_ignore_ascii_case(exe))
}

/// Per-app override first (SEL-4.4). Where the platform reads PRIMARY, file managers get Ctrl+C and every
/// other app no key at all, so no terminal can be interrupted (LNX-3.2, 3.3). Else the Ctrl+C-unsafe list, else Ctrl+C (SEL-4).
pub fn copy_key(exe: &str, cfg: &CaptureSettings, primary: bool) -> CopyKey {
    if let Some((_, key)) = cfg.copy_shortcuts.iter().find(|(app, _)| app.eq_ignore_ascii_case(exe)) {
        return *key;
    }
    if primary {
        return if file_manager(exe) { CopyKey::CtrlC } else { CopyKey::Never };
    }
    if cfg.unsafe_apps.iter().any(|app| app.eq_ignore_ascii_case(exe)) { CopyKey::CtrlInsert } else { CopyKey::CtrlC }
}

pub fn truncate(mut text: String, max_chars: usize) -> (String, bool) {
    match text.char_indices().nth(max_chars) {
        Some((cut, _)) => {
            text.truncate(cut);
            (text, true)
        }
        None => (text, false),
    }
}

fn text_content(text: String, max_chars: usize) -> Content {
    let (text, truncated) = truncate(text, max_chars);
    Content::Text { text, truncated }
}

/// Simulated copy. Success is judged only by a clipboard change, and the user's clipboard is always restored (SEL-3).
fn copy_selection<P: Platform>(p: &P, key: CopyKey, timeout_ms: u64) -> Clip {
    if key == CopyKey::Never || !p.extra_modifiers_released(Duration::from_millis(300)) {
        return Clip::Nothing;
    }
    let Some(snapshot) = p.clipboard_snapshot() else {
        return Clip::Nothing;
    };
    let before = p.clipboard_seq();
    p.send_copy(key);
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let clip = loop {
        if p.clipboard_seq() != before {
            break p.clipboard_read();
        }
        if Instant::now() >= deadline {
            break Clip::Nothing;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    p.clipboard_restore(snapshot);
    clip
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    /// Scripted OS. `on_copy` is what the target app puts on the clipboard when it receives the copy key.
    struct Fake {
        app: SourceApp,
        uia: Option<String>,
        primary: bool,
        owner: Option<SourceApp>,
        readable: bool,
        modifiers_held: bool,
        clipboard_busy: bool,
        on_copy: RefCell<Option<Clip>>,
        clipboard: RefCell<Clip>,
        seq: Cell<u32>,
        calls: RefCell<Vec<String>>,
    }

    impl Fake {
        fn new(exe: &str) -> Self {
            Fake {
                app: SourceApp { exe: exe.into(), name: exe.trim_end_matches(".exe").into(), title: "Secret doc".into(), elevated: false, window: 0 },
                uia: None,
                primary: false,
                owner: None,
                readable: true,
                modifiers_held: false,
                clipboard_busy: false,
                on_copy: RefCell::new(None),
                clipboard: RefCell::new(Clip::Text("what the user copied earlier".into())),
                seq: Cell::new(7),
                calls: RefCell::new(Vec::new()),
            }
        }
        fn log(&self, call: impl Into<String>) {
            self.calls.borrow_mut().push(call.into());
        }
        fn called(&self, prefix: &str) -> bool {
            self.calls.borrow().iter().any(|c| c.starts_with(prefix))
        }
    }

    impl Platform for Fake {
        type Snapshot = Clip;
        fn foreground(&self) -> SourceApp {
            self.log("foreground");
            self.app.clone()
        }
        fn accessible_selection(&self, _: usize) -> Option<String> {
            self.log("uia");
            self.uia.clone()
        }
        fn extra_modifiers_released(&self, _: Duration) -> bool {
            !self.modifiers_held
        }
        fn clipboard_seq(&self) -> u32 {
            self.seq.get()
        }
        fn clipboard_snapshot(&self) -> Option<Clip> {
            self.log("snapshot");
            (!self.clipboard_busy).then(|| self.clipboard.borrow().clone())
        }
        fn clipboard_restore(&self, snapshot: Clip) {
            self.log("restore");
            *self.clipboard.borrow_mut() = snapshot;
        }
        fn clipboard_read(&self) -> Clip {
            self.clipboard.borrow().clone()
        }
        fn send_copy(&self, key: CopyKey) {
            self.log(format!("send {key:?}"));
            if let Some(clip) = self.on_copy.borrow_mut().take() {
                *self.clipboard.borrow_mut() = clip;
                self.seq.set(self.seq.get() + 1);
            }
        }
        fn primary(&self) -> bool {
            self.primary
        }
        fn selection_owner(&self) -> Option<SourceApp> {
            self.owner.clone()
        }
        fn selection_readable(&self) -> bool {
            self.readable
        }
    }

    fn cfg() -> CaptureSettings {
        CaptureSettings { copy_timeout_ms: 20, ..CaptureSettings::default() }
    }

    fn text(c: &Capture) -> &str {
        match &c.content {
            Content::Text { text, .. } => text,
            other => panic!("expected text, got {other:?}"),
        }
    }

    #[test]
    fn sel_2_1_foreground_is_read_before_anything_else() {
        let fake = Fake::new("notepad.exe");
        capture(&fake, &cfg(), false);
        assert_eq!(fake.calls.borrow()[0], "foreground");
    }

    #[test]
    fn sel_2_2_accessibility_hit_never_touches_the_clipboard() {
        let fake = Fake { uia: Some("hello".into()), ..Fake::new("chrome.exe") };
        let c = capture(&fake, &cfg(), false);
        assert_eq!((c.method, text(&c)), (Method::Direct, "hello"));
        assert!(!fake.called("send") && !fake.called("snapshot"));
    }

    #[test]
    fn sel_2_2_accessibility_miss_falls_back_to_copy() {
        let fake = Fake::new("chrome.exe");
        *fake.on_copy.borrow_mut() = Some(Clip::Text("from copy".into()));
        let c = capture(&fake, &cfg(), false);
        assert_eq!((c.method, text(&c)), (Method::Copy, "from copy"));
    }

    #[test]
    fn sel_2_3_nothing_selected_gives_no_content() {
        let c = capture(&Fake::new("chrome.exe"), &cfg(), false);
        assert_eq!((c.method, &c.content), (Method::None, &Content::None));
    }

    #[test]
    fn sel_2_4_long_text_is_truncated_at_a_char_boundary() {
        assert_eq!(truncate("héllo".into(), 2), ("hé".into(), true));
        assert_eq!(truncate("hi".into(), 2), ("hi".into(), false));
        let fake = Fake { uia: Some("abcdef".into()), ..Fake::new("x.exe") };
        let c = capture(&fake, &CaptureSettings { max_chars: 3, ..cfg() }, false);
        assert_eq!(c.content, Content::Text { text: "abc".into(), truncated: true });
    }

    #[test]
    fn sel_3_1_clipboard_is_restored_after_success_and_after_timeout() {
        let success = Fake::new("x.exe");
        *success.on_copy.borrow_mut() = Some(Clip::Text("selected".into()));
        capture(&success, &cfg(), false);
        assert!(success.called("restore"));
        assert_eq!(*success.clipboard.borrow(), Clip::Text("what the user copied earlier".into()));

        let timeout = Fake::new("x.exe");
        capture(&timeout, &cfg(), false);
        assert!(timeout.called("restore"));
    }

    #[test]
    fn sel_3_1_clipboard_that_cannot_be_saved_is_never_touched() {
        let fake = Fake { clipboard_busy: true, ..Fake::new("x.exe") };
        *fake.on_copy.borrow_mut() = Some(Clip::Text("selected".into()));
        let c = capture(&fake, &cfg(), false);
        assert!(!fake.called("send") && !fake.called("restore"));
        assert_eq!(c.content, Content::None);
    }

    #[test]
    fn sel_3_2_held_extra_modifier_skips_the_simulated_copy() {
        let fake = Fake { modifiers_held: true, ..Fake::new("x.exe") };
        *fake.on_copy.borrow_mut() = Some(Clip::Text("selected".into()));
        let c = capture(&fake, &cfg(), false);
        assert!(!fake.called("send") && !fake.called("snapshot"));
        assert_eq!(c.content, Content::None);
    }

    #[test]
    fn sel_3_3_copy_success_is_detected_by_the_clipboard_changing() {
        let fake = Fake::new("x.exe");
        *fake.on_copy.borrow_mut() = Some(Clip::Text("selected".into()));
        assert_eq!(text(&capture(&fake, &cfg(), false)), "selected");
    }

    #[test]
    fn sel_3_4_unchanged_clipboard_is_never_used_as_the_selection() {
        let fake = Fake::new("x.exe"); // app ignores the copy key; old clipboard text must not leak in
        let c = capture(&fake, &cfg(), false);
        assert!(fake.called("send"));
        assert_eq!(c.content, Content::None);
    }

    #[test]
    fn sel_4_1_unsafe_app_is_never_sent_ctrl_c() {
        let fake = Fake::new("windowsterminal.exe");
        capture(&fake, &cfg(), false);
        assert!(!fake.called("send CtrlC"));
    }

    #[test]
    fn sel_4_2_unsafe_app_gets_ctrl_insert() {
        let fake = Fake::new("WindowsTerminal.exe");
        capture(&fake, &cfg(), false);
        assert!(fake.called("send CtrlInsert"));
        assert_eq!(copy_key("notepad.exe", &cfg(), false), CopyKey::CtrlC);
    }

    #[test]
    fn sel_4_3_default_unsafe_list_has_terminals_and_ides() {
        let apps = CaptureSettings::default().unsafe_apps;
        for exe in ["WindowsTerminal.exe", "conhost.exe", "OpenConsole.exe", "cmd.exe", "powershell.exe", "pwsh.exe", "Code.exe", "idea64.exe"] {
            assert!(apps.iter().any(|a| a == exe), "{exe} missing");
        }
    }

    #[test]
    fn sel_4_4_per_app_override_wins_case_insensitively() {
        let mut c = cfg();
        c.copy_shortcuts.insert("Notepad.exe".into(), CopyKey::Never);
        c.copy_shortcuts.insert("code.exe".into(), CopyKey::CtrlShiftC);
        assert_eq!(copy_key("notepad.exe", &c, false), CopyKey::Never);
        assert_eq!(copy_key("Code.exe", &c, false), CopyKey::CtrlShiftC);
        c.copy_shortcuts.insert("wine".into(), CopyKey::CtrlC);
        assert_eq!(copy_key("wine", &c, true), CopyKey::CtrlC, "also where PRIMARY is read, for apps that don't set it");
        let fake = Fake::new("notepad.exe");
        capture(&fake, &c, false);
        assert!(!fake.called("send"));
    }

    #[test]
    fn sel_6_1_elevated_app_gets_no_keys_and_a_notice() {
        let fake = Fake::new("regedit.exe");
        let fake = Fake { app: SourceApp { elevated: true, ..fake.app.clone() }, ..fake };
        let c = capture(&fake, &cfg(), false);
        assert_eq!(c.notice, Some(Notice::Elevated));
        assert!(!fake.called("uia") && !fake.called("send"));
    }

    #[test]
    fn sel_6_2_empty_capture_offers_snip_only_with_vision() {
        assert_eq!(capture(&Fake::new("x.exe"), &cfg(), true).notice, Some(Notice::NoSelection { offer_snip: true }));
        assert_eq!(capture(&Fake::new("x.exe"), &cfg(), false).notice, Some(Notice::NoSelection { offer_snip: false }));
        let found = Fake { uia: Some("text".into()), ..Fake::new("x.exe") };
        assert_eq!(capture(&found, &cfg(), true).notice, None);
    }

    #[test]
    fn pri_2_1_a_blocked_app_is_never_read_and_its_title_is_dropped() {
        let fake = Fake { uia: Some("secret".into()), ..Fake::new("KeePass.exe") };
        *fake.on_copy.borrow_mut() = Some(Clip::Text("secret".into()));
        let c = capture(&fake, &cfg(), true);
        assert_eq!(*fake.calls.borrow(), ["foreground"], "no accessibility read, no keys, no clipboard");
        assert_eq!((c.notice, c.content, c.app.exe.as_str(), c.app.title.as_str()), (Some(Notice::Blocked), Content::None, "KeePass.exe", ""));
        let title_rule = CaptureSettings { blocklist: vec![Rule::Title("secret DOC".into())], ..cfg() };
        assert_eq!(capture(&Fake::new("notepad.exe"), &title_rule, false).notice, Some(Notice::Blocked));
    }

    #[test]
    fn pri_2_2_rules_match_the_whole_process_name_or_a_title_keyword_ignoring_case() {
        let app = |exe: &str, title: &str| SourceApp { exe: exe.into(), title: title.into(), ..Default::default() };
        let process = [Rule::Process(" keepass.EXE ".into())];
        assert!(blocked(&process, &app("KeePass.exe", "")));
        assert!(!blocked(&process, &app("KeePass.exe.bak", "")) && !blocked(&process, &app("notepad.exe", "KeePass.exe")), "whole name, process only");
        let title = [Rule::Title("Net Banking".into())];
        assert!(blocked(&title, &app("chrome.exe", "HDFC NET BANKING - Google Chrome")));
        assert!(!blocked(&title, &app("Net Banking.exe", "Inbox")), "title only");
        assert!(!blocked(&[Rule::Process(" ".into()), Rule::Title("".into())], &app("", "anything")), "an empty rule matches nothing");
    }

    #[test]
    fn pri_2_3_password_managers_and_credential_prompts_are_blocked_by_default() {
        let rules = CaptureSettings::default().blocklist;
        for exe in ["KeePass.exe", "KeePassXC.exe", "1Password.exe", "Bitwarden.exe", "Enpass.exe", "CredentialUIBroker.exe", "consent.exe", "keepassxc", "bitwarden", "1password"] {
            assert!(rules.contains(&Rule::Process(exe.into())), "{exe} missing");
        }
    }

    #[test]
    fn sel_7_1_capture_records_source_app_and_method() {
        let fake = Fake { uia: Some("hi".into()), ..Fake::new("chrome.exe") };
        let c = capture(&fake, &cfg(), false);
        assert_eq!((c.app.exe.as_str(), c.app.title.as_str(), c.method), ("chrome.exe", "Secret doc", Method::Direct));
    }

    #[test]
    fn lnx_3_1_primary_is_read_first_and_no_key_is_sent() {
        let fake = Fake { uia: Some("from primary".into()), primary: true, ..Fake::new("gnome-text-editor") };
        let c = capture(&fake, &cfg(), false);
        assert_eq!((c.method, text(&c)), (Method::Direct, "from primary"));
        assert!(!fake.called("snapshot") && !fake.called("send"));
    }

    #[test]
    fn lnx_3_2_a_file_manager_skips_primary_copies_the_files_and_restores_the_clipboard() {
        let fake = Fake { uia: Some("text selected earlier in another app".into()), primary: true, ..Fake::new("nautilus") };
        *fake.on_copy.borrow_mut() = Some(Clip::Files(vec![PathBuf::from(env!("CARGO_MANIFEST_DIR"))]));
        let c = capture(&fake, &cfg(), false);
        assert!(!fake.called("uia"), "PRIMARY is skipped");
        assert!(fake.called("send CtrlC") && fake.called("restore"));
        assert!(matches!(&c.content, Content::Files(f) if f.items.len() == 1), "{:?}", c.content);
        assert_eq!(*fake.clipboard.borrow(), Clip::Text("what the user copied earlier".into()));
        for exe in FILE_MANAGERS {
            assert_eq!(copy_key(&exe.to_uppercase(), &cfg(), true), CopyKey::CtrlC, "{exe}");
        }
    }

    #[test]
    fn lnx_3_3_terminals_never_get_a_key() {
        let terminals = ["gnome-terminal", "gnome-terminal-server", "konsole", "xterm", "alacritty", "kitty", "wezterm", "wezterm-gui", "tilix", "terminator", "xfce4-terminal", "ptyxis"];
        for exe in terminals {
            assert_eq!(copy_key(exe, &cfg(), true), CopyKey::Never, "{exe}");
            for primary in [None, Some("make test".to_string())] {
                let fake = Fake { uia: primary, primary: true, ..Fake::new(exe) };
                *fake.on_copy.borrow_mut() = Some(Clip::Text("would have been ^C".into()));
                capture(&fake, &cfg(), false);
                assert!(!fake.called("send") && !fake.called("snapshot"), "{exe}");
            }
        }
    }

    #[test]
    fn lnx_3_4_a_session_that_cannot_read_selections_says_so() {
        let fake = Fake { primary: true, readable: false, ..Fake::new("") };
        let c = capture(&fake, &cfg(), true);
        assert_eq!((c.content, c.notice), (Content::None, Some(Notice::Unreadable)));
        let nothing_selected = Fake { primary: true, ..Fake::new("") };
        assert_eq!(capture(&nothing_selected, &cfg(), false).notice, Some(Notice::NoSelection { offer_snip: false }));
    }

    #[test]
    fn lnx_3_5_a_selection_owned_by_a_blocked_app_is_not_read() {
        let owned_by = |exe: &str| Some(SourceApp { exe: exe.into(), ..Default::default() });
        let fake = Fake { uia: Some("vault note".into()), primary: true, owner: owned_by("bitwarden"), ..Fake::new("gnome-text-editor") };
        let c = capture(&fake, &cfg(), false);
        assert!(!fake.called("uia") && !fake.called("send") && !fake.called("snapshot"));
        assert_eq!((c.content, c.notice, c.app.exe.as_str()), (Content::None, Some(Notice::NoSelection { offer_snip: false }), "gnome-text-editor"));
        let fake = Fake { uia: Some("notes".into()), primary: true, owner: owned_by("mousepad"), ..Fake::new("gnome-text-editor") };
        assert_eq!(text(&capture(&fake, &cfg(), false)), "notes");
    }

    #[test]
    fn lnx_4_3_only_windows_and_x11_identify_the_app_in_front() {
        assert!(identifies_apps(Session::Windows) && identifies_apps(Session::X11));
        assert!(!identifies_apps(Session::Wayland) && !identifies_apps(Session::Unknown));
    }

    #[test]
    fn sel_7_2_window_title_reaches_the_prompt_only_when_allowed() {
        let app = Fake::new("chrome.exe").app;
        assert_eq!(app.for_prompt(false), "chrome");
        assert_eq!(app.for_prompt(true), "chrome (Secret doc)");
    }
}
