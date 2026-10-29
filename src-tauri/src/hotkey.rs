use crate::capture::{self, Chord, Content, Key, Platform};
use crate::settings::{Hotkeys, Settings};
use crate::session::{self, Session};
use crate::{bubble, chat, llm, AppState};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut};

const RESERVED: [&str; 8] = ["Ctrl+Alt+Delete", "Ctrl+Shift+Escape", "Ctrl+Escape", "Alt+Tab", "Alt+F4", "Super+L", "Super+D", "Super+Tab"];

static CAPTURING: AtomicBool = AtomicBool::new(false);

/// SEL-1.2. Only hotkeys that changed are checked, so an old invalid value never blocks other edits.
pub fn validate(new: &Hotkeys, old: &Hotkeys) -> Result<(), String> {
    let fields = [
        ("Ask", Some(&new.ask), Some(&old.ask)),
        ("Snip", Some(&new.snip), Some(&old.snip)),
        ("Define", Some(&new.define), Some(&old.define)),
        ("Push-to-talk", new.push_to_talk.as_ref(), old.push_to_talk.as_ref()),
    ];
    let parsed: Vec<(&str, Shortcut)> = fields.iter().filter_map(|(label, v, _)| Some((*label, Shortcut::from_str(v.as_ref()?).ok()?))).collect();
    for (label, value, before) in fields {
        let Some(value) = value.filter(|v| Some(*v) != before) else { continue };
        let shortcut = Shortcut::from_str(value).map_err(|_| format!("{label}: \"{value}\" isn't a valid hotkey."))?;
        if !shortcut.mods.intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER) {
            return Err(format!("{label}: include Ctrl, Alt or Win. Shift alone would fire while you type capitals."));
        }
        if RESERVED.iter().any(|r| Shortcut::from_str(r).is_ok_and(|r| r == shortcut)) {
            return Err(format!("{label}: {value} is reserved by the system."));
        }
        if let Some((other, _)) = parsed.iter().find(|(l, s)| *l != label && *s == shortcut) {
            return Err(format!("{label}: {value} is already used for {other}."));
        }
    }
    Ok(())
}

/// Registers the ask, snip and define hotkeys unless paused (APP-1.3). If a new one can't be registered, its old one is put back
/// (SEL-1.3). Snip and Define come after Ask, so a clash never costs Ask, and a clash on a key this change didn't touch (another
/// app took it) is only logged, so it never blocks changing the others (HOV-1.1). Push-to-talk is registered by spec 09, so we
/// don't hold a key we don't use.
pub fn apply(app: &AppHandle, old: Option<&Settings>, new: &Settings) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    let _ = shortcuts.unregister_all();
    // On Wayland the desktop shortcut runs `cursora --ask` instead (LNX-2.2); through XWayland a hotkey would only fire over X11 apps.
    if new.paused || session::current() == Session::Wayland {
        return Ok(());
    }
    if let Err(e) = shortcuts.register(new.hotkeys.ask.as_str()) {
        if let Some(old) = old.filter(|o| !o.paused) {
            let _ = shortcuts.register(old.hotkeys.ask.as_str());
        }
        log::warn!("hotkey: {e}");
        return Err(format!("Couldn't use {} as the Ask hotkey. Another app may already use it.", new.hotkeys.ask));
    }
    let before = old.filter(|o| !o.paused).map(|o| &o.hotkeys);
    let mut error = None;
    for (label, key, was) in [("Snip", &new.hotkeys.snip, before.map(|h| &h.snip)), ("Define", &new.hotkeys.define, before.map(|h| &h.define))] {
        if let Err(e) = shortcuts.register(key.as_str()) {
            log::warn!("hotkey: {label}: {e}");
            if was == Some(key) {
                continue; // unchanged: said at startup already
            }
            if let Some(was) = was {
                let _ = shortcuts.register(was.as_str());
            }
            error.get_or_insert(format!("Couldn't use {key} as the {label} hotkey. Another app may already use it."));
        }
    }
    error.map_or(Ok(()), Err)
}

/// OFF-1.3: the hotkey as keys that can be pressed again, for a letter, digit, F1 to F24 or Space with its modifiers.
pub fn chord(shortcut: &Shortcut) -> Option<Chord> {
    let name = shortcut.key.to_string(); // "KeyG", "Digit5", "F9", "Space"
    let key = if name == "Space" {
        Key::Space
    } else if let Some(l) = name.strip_prefix("Key").filter(|l| l.len() == 1) {
        Key::Letter(l.chars().next()?.to_ascii_lowercase())
    } else if let Some(d) = name.strip_prefix("Digit") {
        Key::Digit(d.parse().ok().filter(|d| *d <= 9)?)
    } else {
        Key::F(name.strip_prefix('F')?.parse().ok().filter(|n| (1..=24).contains(n))?)
    };
    let has = |m| shortcut.mods.contains(m);
    Some(Chord { ctrl: has(Modifiers::CONTROL), alt: has(Modifiers::ALT), shift: has(Modifiers::SHIFT), win: has(Modifiers::SUPER), key })
}

/// OFF-1.2: steps aside for one press. The hotkey is released, its keys are pressed again for the app, and after the app
/// has had them (injected keys arrive after the call returns) the hotkey is taken back. Its own thread, so the wait never
/// holds up the main one.
fn pass_through(app: &AppHandle, shortcut: Shortcut, exe: &str) {
    let Some(chord) = chord(&shortcut) else {
        return log::info!("hotkey: off in {exe}, but {shortcut:?} can't be pressed again (OFF-1.3)");
    };
    log::info!("hotkey: off in {exe}, passed on");
    let app = app.clone();
    std::thread::spawn(move || {
        let shortcuts = app.global_shortcut();
        if let Err(e) = shortcuts.unregister(shortcut) {
            return log::warn!("hotkey: couldn't step aside: {e}");
        }
        // X11: global-hotkey queues the ungrab without sending it, so the keys pressed again met the old grab and were
        // lost (issue 67). Its next request with a reply sends the queue first: any registration makes one, even one that
        // fails because the keyboard has no F24, and one that succeeds is undone at once.
        // shortcut: works around global-hotkey 0.8; drop it once the crate flushes after ungrabbing.
        #[cfg(target_os = "linux")]
        {
            let flush = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::SUPER), tauri_plugin_global_shortcut::Code::F24);
            if shortcuts.register(flush).is_ok() {
                let _ = shortcuts.unregister(flush);
            }
        }
        capture::OsPlatform::new().send_chord(&chord);
        std::thread::sleep(Duration::from_millis(100));
        if let Err(e) = shortcuts.register(shortcut) {
            log::warn!("hotkey: couldn't take it back: {e}");
        }
    });
}

/// A registered hotkey was pressed: in an app on the off list it passes through (OFF-1.1); otherwise the snip key starts
/// a snip (VIS-2.1), the define key defines the word under the pointer (HOV-1.2), and the other is Ask.
pub fn on_press(app: &AppHandle, shortcut: &Shortcut) {
    let hotkeys = app.state::<AppState>().settings.lock().unwrap().hotkeys.clone();
    if !hotkeys.off_in.is_empty() {
        let front = capture::OsPlatform::new().foreground();
        if capture::blocked(&hotkeys.off_in, &front) {
            return pass_through(app, *shortcut, &front.exe);
        }
    }
    let is = |key: &str| Shortcut::from_str(key).is_ok_and(|s| s == *shortcut);
    if is(&hotkeys.snip) {
        crate::snip::start(app, Default::default());
    } else if is(&hotkeys.define) {
        crate::snip::define(app);
    } else {
        on_ask(app);
    }
}

/// Closes an open bubble (SEL-1.4). Otherwise captures off the main thread and opens the bubble.
/// A second press while a capture runs is ignored, since two captures would fight over the clipboard.
pub fn on_ask(app: &AppHandle) {
    if bubble::is_open(app) {
        return bubble::close(app);
    }
    if CAPTURING.swap(true, Ordering::SeqCst) {
        return;
    }
    // Read at the keypress, before the capture can take any time (BUB-1.1). Under XWayland it is stale
    // while a Wayland app has focus, so on Wayland the bubble is centered instead (LNX-4.2).
    let cursor = app.cursor_position().ok().filter(|_| session::current() != Session::Wayland).map(|p| (p.x.round() as i32, p.y.round() as i32));
    let app = app.clone();
    let s = app.state::<AppState>().settings.lock().unwrap().clone();
    std::thread::spawn(move || {
        let started = Instant::now();
        // With no model set up, nothing is captured: the bubble only says so (LLM-9.1).
        let model = llm::availability(&s.models, s.local_only).ok();
        let vision = llm::active(&app).is_ok_and(|m| m.info().vision); // Echo too, in debug builds
        let health = (model.is_some() || llm::echo()).then(|| {
            let app = app.clone();
            std::thread::spawn(move || llm::health(&app)) // alongside the capture
        });
        let platform = capture::OsPlatform::new();
        let mut result = match health {
            Some(_) => capture::capture(&platform, &s.capture, vision || crate::snip::ocr::available()), // "Snip an area instead" (OCR-1.1)
            None => capture::Capture { app: platform.foreground(), method: capture::Method::None, content: Content::None, notice: None },
        };
        CAPTURING.store(false, Ordering::SeqCst);
        // shortcut: decoding up to 4 images comes before the bubble shows; move it after the bubble if selections of large photos feel slow.
        if let (true, Content::Files(f)) = (vision, &mut result.content) {
            capture::files::attach_images(f, s.snip.max_side); // VIS-2.7
        }
        // Metadata only, never the text (APP-7.2).
        let size = match &result.content {
            Content::Text { text: t, truncated } => format!("{} chars{}", t.chars().count(), if *truncated { ", truncated" } else { "" }),
            Content::Files(f) => format!("{} items", f.items.len() + f.omitted),
            Content::Snip(s) => format!("snip {}×{}", s.width, s.height),
            Content::None => "nothing".into(),
        };
        log::info!("capture from {}: {:?}, {size}, {:?}, {} ms", result.app.exe, result.method, result.notice, started.elapsed().as_millis());
        let (previous, source) = (result.app.window, result.app.clone());
        let mut conv = chat::start(&app, result);
        // A check that finished during the capture decides before the bubble shows; a slower one swaps the input afterwards.
        let late = match health {
            Some(h) if h.is_finished() => {
                if let Err(reason) = h.join().unwrap_or(Ok(())) {
                    conv = chat::set_unavailable(&app, conv.id, reason).unwrap_or(conv);
                }
                None
            }
            h => h,
        };
        let id = conv.id;
        bubble::open(&app, cursor, previous, conv);
        crate::tray::seen(&app, &source); // after the bubble, so a menu rebuild never delays it
        if let Some(Err(reason)) = late.map(|h| h.join().unwrap_or(Ok(()))) {
            if chat::set_unavailable(&app, id, reason).is_some() {
                let _ = app.emit_to(bubble::LABEL, "chat-load", ());
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(ask: &str, snip: &str, ptt: Option<&str>) -> Hotkeys {
        Hotkeys { ask: ask.into(), snip: snip.into(), push_to_talk: ptt.map(Into::into), ..Hotkeys::default() }
    }

    #[test]
    fn hov_1_1_the_define_hotkey_is_checked_like_the_others() {
        let old = Hotkeys::default();
        assert_eq!(old.define, "Ctrl+Alt+G");
        assert_eq!(validate(&Hotkeys { define: "Ctrl+Alt+D".into(), ..old.clone() }, &old), Ok(()));
        assert!(validate(&Hotkeys { define: "Shift+D".into(), ..old.clone() }, &old).unwrap_err().starts_with("Define: include Ctrl, Alt or Win"));
        assert!(validate(&Hotkeys { define: "Ctrl+G".into(), ..old.clone() }, &old).unwrap_err().contains("already used for Ask"));
        assert!(validate(&Hotkeys { ask: "Ctrl+Alt+G".into(), ..old.clone() }, &old).unwrap_err().contains("already used for Define"));
    }

    #[test]
    fn sel_1_2_hotkeys_are_validated() {
        let old = Hotkeys::default();
        assert_eq!(validate(&keys("Alt+Q", "Ctrl+Shift+G", None), &old), Ok(()));
        assert!(validate(&keys("Shift+G", "Ctrl+Shift+G", None), &old).unwrap_err().contains("Ctrl, Alt or Win"));
        assert!(validate(&keys("G", "Ctrl+Shift+G", None), &old).is_err());
        assert!(validate(&keys("Alt+F4", "Ctrl+Shift+G", None), &old).unwrap_err().contains("reserved"));
        assert!(validate(&keys("Ctrl+Shift+G", "Ctrl+Shift+G", None), &old).unwrap_err().contains("already used for Snip"));
        assert!(validate(&keys("Ctrl+G+H", "Ctrl+Shift+G", None), &old).unwrap_err().contains("isn't a valid hotkey"));
    }

    #[test]
    fn sel_1_2_only_changed_hotkeys_are_checked() {
        let old = keys("Ctrl+G", "Ctrl+Shift+G", Some("U")); // an invalid value saved earlier
        assert_eq!(validate(&keys("Alt+Q", "Ctrl+Shift+G", Some("U")), &old), Ok(()));
        assert!(validate(&keys("Ctrl+G", "Ctrl+Shift+G", Some("V")), &old).is_err());
    }

    #[test]
    fn off_1_3_only_letters_digits_f_keys_and_space_are_pressed_again() {
        let c = |s: &str| chord(&Shortcut::from_str(s).unwrap());
        let ctrl = |key| Some(Chord { ctrl: true, alt: false, shift: false, win: false, key });
        assert_eq!(c("Ctrl+G"), ctrl(Key::Letter('g')));
        assert_eq!(c("Ctrl+Shift+F9"), Some(Chord { shift: true, ..ctrl(Key::F(9)).unwrap() }));
        assert_eq!(c("Ctrl+Alt+Shift+F24"), Some(Chord { alt: true, shift: true, ..ctrl(Key::F(24)).unwrap() }));
        assert_eq!(c("Ctrl+3"), ctrl(Key::Digit(3)));
        assert_eq!(c("Ctrl+Space"), ctrl(Key::Space));
        assert_eq!(c("Super+Q"), Some(Chord { ctrl: false, win: true, ..ctrl(Key::Letter('q')).unwrap() }));
        assert_eq!(c("Ctrl+ArrowUp"), None);
        assert_eq!(c("Ctrl+Enter"), None);
    }

    #[test]
    fn off_1_1_an_app_on_the_off_list_is_matched_like_the_blocklist() {
        use crate::capture::{Rule, SourceApp};
        let s: crate::settings::Settings = serde_json::from_str(r#"{"hotkeys":{"ask":"Ctrl+G"}}"#).unwrap();
        assert!(s.hotkeys.off_in.is_empty(), "an older settings file has an empty list");
        let off = vec![Rule::Process("Code.exe".into()), Rule::Title("Jupyter".into())];
        let app = |exe: &str, title: &str| SourceApp { exe: exe.into(), title: title.into(), ..Default::default() };
        assert!(capture::blocked(&off, &app("code.exe", "main.rs - cursora")));
        assert!(capture::blocked(&off, &app("chrome.exe", "Untitled - Jupyter Notebook")));
        assert!(!capture::blocked(&off, &app("notepad.exe", "notes.txt - Notepad")));
    }
}
