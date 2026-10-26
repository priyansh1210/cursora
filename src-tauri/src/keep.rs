//! Pin and Save for answers (spec 17). Rust copies the answer from the conversation, so the page only says which one.
use crate::AppState;
use serde::Serialize;
use std::collections::HashMap;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_opener::OpenerExt;

/// A finished answer and where it came from (`chat::kept`).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Kept {
    pub question: String,
    pub answer: String,
    pub app: String,
    pub model: Option<String>,
}

/// The open notes, each a copy made when it was pinned (SAV-1.3), by window label. Nothing is stored, so they're gone after a restart (SAV-1.2).
#[derive(Default)]
pub struct Pins {
    notes: Mutex<HashMap<String, Kept>>,
    next: AtomicU32,
}

const NOTE: (f64, f64) = (360.0, 260.0);

/// SAV-1.1: a small window with a title bar (moving, resizing and Close come with it), on top, not taking the typing focus.
/// Async: creating a window inside a synchronous command deadlocks on Windows (wry#583).
// shortcut: no limit on open notes, though each one is its own webview with its own memory. Add a limit if users pin dozens.
#[tauri::command(async)]
pub fn pin_answer(window: WebviewWindow, app: AppHandle, asked: i64) -> Result<(), String> {
    let kept = crate::chat::kept(&app, window.label(), asked)?;
    let pins = app.state::<Pins>();
    let label = format!("pin-{}", pins.next.fetch_add(1, Ordering::SeqCst));
    let title: String = kept.question.lines().next().unwrap_or_default().chars().take(60).collect();
    let open = {
        let mut notes = pins.notes.lock().unwrap();
        notes.insert(label.clone(), kept);
        notes.len() - 1
    };
    let mut builder = WebviewWindowBuilder::new(&app, &label, WebviewUrl::default())
        .additional_browser_args(crate::BROWSER_ARGS)
        .title(if title.trim().is_empty() { "Cursora note" } else { title.as_str() })
        .inner_size(NOTE.0, NOTE.1)
        .min_inner_size(240.0, 140.0)
        .always_on_top(true)
        .focused(false)
        .on_navigation(|url| crate::bubble::is_app_url(url));
    if let Some((x, y)) = beside(&window, open) {
        builder = builder.position(x, y);
    }
    let note = builder.build().map_err(|e| {
        app.state::<Pins>().notes.lock().unwrap().remove(&label);
        e.to_string()
    })?;
    let app = app.clone();
    note.on_window_event(move |e| {
        if let WindowEvent::Destroyed = e {
            app.state::<Pins>().notes.lock().unwrap().remove(&label);
        }
    });
    Ok(())
}

/// To the right of the calling window, or to its left when the screen has no room there; 24 px lower for each note already open.
// shortcut: in the calling window's scale; a note opening on a monitor with another scale may sit a little off. Fine until users mix scales.
fn beside(window: &WebviewWindow, open: usize) -> Option<(f64, f64)> {
    let scale = window.scale_factor().ok()?;
    let pos = window.outer_position().ok()?.to_logical::<f64>(scale);
    let size = window.outer_size().ok()?.to_logical::<f64>(scale);
    let monitor = window.current_monitor().ok()??;
    let (left, width) = (monitor.position().to_logical::<f64>(scale).x, monitor.size().to_logical::<f64>(scale).width);
    let right = pos.x + size.width + 12.0;
    let x = if right + NOTE.0 <= left + width { right } else { (pos.x - NOTE.0 - 12.0).max(left) };
    Some((x, pos.y + 24.0 * (open % 5) as f64))
}

/// The note's own answer, for `Pin.tsx`.
#[tauri::command]
pub fn pinned(window: WebviewWindow, app: AppHandle) -> Option<Kept> {
    app.state::<Pins>().notes.lock().unwrap().get(window.label()).cloned()
}

/// SAV-2.1: writes the answer into the saved-answers folder and returns the file's path. Never looks at "Save history" (SAV-2.6).
#[tauri::command]
pub fn save_answer(window: WebviewWindow, app: AppHandle, asked: i64) -> Result<PathBuf, String> {
    let kept = crate::chat::kept(&app, window.label(), asked)?;
    let now = crate::skills::local_time();
    write(&folder(&app)?, &file_name(&now[..10], &kept.question), &markdown(&now, &kept))
}

/// "Show in folder": the saved file, selected in the file manager. Any existing `.md` file, so it still works after the folder
/// setting changes; revealing a file never opens it.
#[tauri::command]
pub fn show_saved(app: AppHandle, path: PathBuf) -> Result<(), String> {
    if !(path.is_file() && path.extension().is_some_and(|e| e == "md")) {
        return Err("That file isn't there any more.".into());
    }
    let opener = app.opener();
    // A Linux desktop without the file manager's portal can't select the file, so its folder opens instead.
    let folder = path.parent().unwrap_or(&path).to_string_lossy().into_owned();
    opener.reveal_item_in_dir(&path).or_else(|_| opener.open_path(folder, None::<&str>)).map_err(|e| format!("Couldn't show the file: {e}"))
}

/// SAV-2.4: the chosen folder, or Cursora in Documents.
pub fn folder(app: &AppHandle) -> Result<PathBuf, String> {
    let chosen = app.state::<AppState>().settings.lock().unwrap().saved_answers_folder.clone();
    chosen.map_or_else(|| default_folder(app.path().document_dir().ok(), app.path().home_dir().ok()), Ok)
}

/// Documents/Cursora, or ~/Documents/Cursora when the system names no Documents folder.
pub fn default_folder(documents: Option<PathBuf>, home: Option<PathBuf>) -> Result<PathBuf, String> {
    let documents = documents.or_else(|| home.map(|h| h.join("Documents")));
    documents.map(|d| d.join("Cursora")).ok_or_else(|| "Cursora can't find your Documents folder: choose a folder in Settings → General.".into())
}

/// SAV-2.3: "<date> <first words>", without what Windows or Linux refuse, no dot or space at the end, at most 80 characters.
/// The date in front keeps it from ever being a Windows device name such as CON.
pub fn file_name(date: &str, question: &str) -> String {
    let safe: String = question.chars().map(|c| if c.is_control() || r#"\/:*?"<>|"#.contains(c) { ' ' } else { c }).collect();
    let name: String = format!("{date} {}", safe.split_whitespace().collect::<Vec<_>>().join(" ")).chars().take(80).collect();
    name.trim_end_matches(['.', ' ']).to_string()
}

/// SAV-2.2: front matter Obsidian reads as properties (JSON strings are valid YAML), the question quoted, then the answer.
/// Never the selection or files: they may be private, and History keeps them.
pub fn markdown(saved: &str, k: &Kept) -> String {
    let value = |s: &str| serde_json::to_string(s).unwrap_or_default();
    let question: Vec<String> = k.question.lines().map(|l| format!("> {l}")).collect();
    format!(
        "---\nsaved: {}\napp: {}\nmodel: {}\n---\n\n{}\n\n{}\n",
        value(saved),
        value(&k.app),
        value(k.model.as_deref().unwrap_or_default()),
        question.join("\n"),
        k.answer.trim_end()
    )
}

/// SAV-2.3: `<name>.md`, or `<name> (2).md` and so on when taken. `create_new` leaves no gap between the check and the write.
pub fn write(dir: &Path, name: &str, text: &str) -> Result<PathBuf, String> {
    let fail = |e: std::io::Error| format!("Couldn't save the answer in {}: {e}", dir.display());
    std::fs::create_dir_all(dir).map_err(fail)?;
    for n in 1u32.. {
        let path = dir.join(if n == 1 { format!("{name}.md") } else { format!("{name} ({n}).md") });
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut f) => return f.write_all(text.as_bytes()).map(|_| path).map_err(fail),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(fail(e)),
        }
    }
    unreachable!("4 billion answers with one name")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cursora-keep-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn kept() -> Kept {
        Kept { question: "What is due\non Friday?".into(), answer: "The **report**.\n\n- step one\n".into(), app: "Notepad".into(), model: Some("qwen \"small\"".into()) }
    }

    #[test]
    fn sav_2_2_the_file_has_the_date_app_model_question_and_answer_and_no_selection() {
        let md = markdown("2026-10-26 21:14", &kept());
        assert_eq!(md, "---\nsaved: \"2026-10-26 21:14\"\napp: \"Notepad\"\nmodel: \"qwen \\\"small\\\"\"\n---\n\n> What is due\n> on Friday?\n\nThe **report**.\n\n- step one\n");
        let no_model = markdown("2026-10-26 21:14", &Kept { model: None, ..kept() });
        assert!(no_model.contains("\nmodel: \"\"\n"));
    }

    #[test]
    fn sav_2_3_names_are_safe_short_and_never_overwrite() {
        assert_eq!(file_name("2026-10-26", "What's due: \"A/B\" <x>|y*?\\z\n\tnext"), "2026-10-26 What's due A B x y z next");
        assert_eq!(file_name("2026-10-26", "/explain"), "2026-10-26 explain");
        assert_eq!(file_name("2026-10-26", "Is it done... "), "2026-10-26 Is it done");
        assert_eq!(file_name("2026-10-26", ""), "2026-10-26");
        let hindi = file_name("2026-10-26", &"मीटिंग कब है ".repeat(20));
        assert_eq!(hindi.chars().count(), 80, "cut by characters, never inside one");
        let d = temp("names");
        let first = write(&d, "2026-10-26 a", "one").unwrap();
        let second = write(&d, "2026-10-26 a", "two").unwrap();
        assert_eq!((first.file_name().unwrap(), second.file_name().unwrap()), ("2026-10-26 a.md".as_ref(), "2026-10-26 a (2).md".as_ref()));
        assert_eq!(std::fs::read_to_string(&first).unwrap(), "one", "the first file is untouched");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn sav_2_4_the_folder_is_documents_cursora_made_on_the_first_save_with_a_plain_error() {
        assert_eq!(default_folder(Some("D:/Docs".into()), Some("C:/Users/a".into())), Ok(PathBuf::from("D:/Docs/Cursora")));
        assert_eq!(default_folder(None, Some("/home/a".into())), Ok(PathBuf::from("/home/a/Documents/Cursora")));
        assert!(default_folder(None, None).unwrap_err().contains("Settings → General"));
        let d = temp("folder").join("Cursora");
        assert!(write(&d, "x", "y").unwrap().starts_with(&d), "made on the first save");
        let file = temp("file");
        std::fs::write(&file, "not a folder").unwrap();
        let e = write(&file.join("Cursora"), "x", "y").unwrap_err();
        assert!(e.starts_with(&format!("Couldn't save the answer in {}", file.join("Cursora").display())), "{e}");
        let _ = (std::fs::remove_dir_all(d.parent().unwrap()), std::fs::remove_file(&file));
    }

    #[test]
    fn sav_2_6_a_save_writes_the_one_file_and_nothing_else() {
        // Nothing in the save path reads the history settings, so "Save history" off changes nothing here.
        let d = temp("only");
        write(&d, "2026-10-26 q", &markdown("2026-10-26 21:14", &kept())).unwrap();
        let files: Vec<_> = std::fs::read_dir(&d).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(files, ["2026-10-26 q.md"]);
        let _ = std::fs::remove_dir_all(&d);
    }
}
