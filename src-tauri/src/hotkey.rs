use crate::capture::{self, Content, Platform};
use crate::settings::{Hotkeys, Settings};
use crate::{bubble, chat, llm, AppState};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut};

const RESERVED: [&str; 8] = ["Ctrl+Alt+Delete", "Ctrl+Shift+Escape", "Ctrl+Escape", "Alt+Tab", "Alt+F4", "Super+L", "Super+D", "Super+Tab"];

static CAPTURING: AtomicBool = AtomicBool::new(false);

/// SEL-1.2. Only hotkeys that changed are checked, so an old invalid value never blocks other edits.
pub fn validate(new: &Hotkeys, old: &Hotkeys) -> Result<(), String> {
    let fields = [
        ("Ask", Some(&new.ask), Some(&old.ask)),
        ("Snip", Some(&new.snip), Some(&old.snip)),
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

/// Registers the ask hotkey unless paused (APP-1.3). If the new one can't be registered, the old one is put back (SEL-1.3).
/// Snip (spec 10) and push-to-talk (spec 09) are registered by their own specs, so we don't hold keys we don't use.
pub fn apply(app: &AppHandle, old: Option<&Settings>, new: &Settings) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    let _ = shortcuts.unregister_all();
    if new.paused {
        return Ok(());
    }
    if let Err(e) = shortcuts.register(new.hotkeys.ask.as_str()) {
        if let Some(old) = old.filter(|o| !o.paused) {
            let _ = shortcuts.register(old.hotkeys.ask.as_str());
        }
        log::warn!("hotkey: {e}");
        return Err(format!("Couldn't use {} as the Ask hotkey. Another app may already use it.", new.hotkeys.ask));
    }
    Ok(())
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
    // Read at the keypress, before the capture can take any time (BUB-1.1).
    let cursor = app.cursor_position().ok().map(|p| (p.x.round() as i32, p.y.round() as i32));
    let app = app.clone();
    let s = app.state::<AppState>().settings.lock().unwrap().clone();
    std::thread::spawn(move || {
        let started = Instant::now();
        // With no model set up, nothing is captured: the bubble only says so (LLM-9.1).
        let model = llm::availability(&s.models, s.local_only).ok();
        let vision = model.is_some_and(|(p, m)| llm::vision(p, m));
        let health = (model.is_some() || llm::echo()).then(|| {
            let app = app.clone();
            std::thread::spawn(move || llm::health(&app)) // alongside the capture
        });
        let platform = capture::OsPlatform::new();
        let result = match health {
            Some(_) => capture::capture(&platform, &s.capture, vision),
            None => capture::Capture { app: platform.foreground(), method: capture::Method::None, content: Content::None, notice: None },
        };
        CAPTURING.store(false, Ordering::SeqCst);
        // Metadata only, never the text (APP-7.2).
        let size = match &result.content {
            Content::Text { text: t, truncated } => format!("{} chars{}", t.chars().count(), if *truncated { ", truncated" } else { "" }),
            Content::Files(f) => format!("{} items", f.items.len() + f.omitted),
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
        Hotkeys { ask: ask.into(), snip: snip.into(), push_to_talk: ptt.map(Into::into) }
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
}
