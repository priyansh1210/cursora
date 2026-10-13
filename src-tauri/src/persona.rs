//! Personas (spec 05): who the assistant is while personality is on.
//! The presets live here, so they can't be deleted and improve with updates; the user's own live in settings.
use crate::settings::Settings;
use crate::{prompt, AppState};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

pub const DEFAULT: &str = "preset:cursora";
const PRESET: &str = "preset:";
const TONES: [&str; 5] = ["friendly", "professional", "witty", "calm", "sarcastic"];

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Verbosity {
    Brief,
    Normal,
    Detailed,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Expertise {
    Beginner,
    Intermediate,
    Expert,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Persona {
    /// "preset:<name>" for the built-in ones. A file without one (hand-written or imported) gets a new id when added.
    #[serde(default)]
    pub id: String,
    pub name: String,
    /// "male", "female", "neutral" or the user's own words.
    pub gender: String,
    /// One of TONES or the user's own words.
    pub tone: String,
    pub verbosity: Verbosity,
    pub emoji: bool,
    /// None: the language of the question.
    pub reply_language: Option<String>,
    pub user_expertise: Expertise,
    #[serde(default)]
    pub custom_instructions: String,
}

/// PER-1.3. Cursora comes first: it is the default and the fallback.
pub fn presets() -> Vec<Persona> {
    use {Expertise::*, Verbosity::*};
    let preset = |name: &str, gender: &str, tone: &str, verbosity, emoji, user_expertise| Persona {
        id: format!("{PRESET}{}", name.to_lowercase()),
        name: name.into(),
        gender: gender.into(),
        tone: tone.into(),
        verbosity,
        emoji,
        reply_language: None,
        user_expertise,
        custom_instructions: String::new(),
    };
    vec![
        preset("Cursora", "neutral", "friendly", Normal, false, Intermediate),
        preset("Ada", "female", "professional", Detailed, false, Expert),
        preset("Arjun", "male", "witty", Brief, true, Intermediate),
        preset("Sage", "neutral", "calm", Normal, false, Beginner),
    ]
}

/// The presets, then the user's own.
pub fn all(s: &Settings) -> Vec<Persona> {
    presets().into_iter().chain(s.personas.iter().cloned()).collect()
}

/// The persona in use; a deleted one falls back to Cursora.
pub fn active(s: &Settings) -> Persona {
    let all = all(s);
    all.iter().find(|p| p.id == s.active_persona).unwrap_or(&all[0]).clone()
}

/// The persona for the next request, or none while personality is off (PER-2.1).
pub fn in_use(s: &Settings) -> Option<Persona> {
    s.personality_on.then(|| active(s))
}

/// PER-1.1. The reason starts with the field, so the editor can show it as is.
pub fn validate(p: &Persona) -> Result<(), String> {
    line("Name", &p.name, 32)?;
    if !["male", "female", "neutral"].contains(&p.gender.as_str()) {
        line("Gender", &p.gender, 32)?;
    }
    if !TONES.contains(&p.tone.as_str()) {
        line("Tone", &p.tone, 64)?;
    }
    if let Some(l) = &p.reply_language {
        line("Reply language", l, 40)?;
        // Any script's letters; ASCII digits and punctuation would only be a typo here.
        if !l.chars().all(|c| c == ' ' || c == '-' || if c.is_ascii() { c.is_ascii_alphabetic() } else { !c.is_whitespace() }) {
            return Err("Reply language: use letters, spaces and hyphens only, like \"Brazilian Portuguese\".".into());
        }
    }
    if p.custom_instructions.chars().count() > 2000 {
        return Err("Custom instructions: at most 2000 characters.".into());
    }
    Ok(())
}

/// One line of 1 to `max` characters, with no spaces around it (the editor trims).
fn line(field: &str, value: &str, max: usize) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{field}: can't be empty."))
    } else if value.chars().count() > max {
        Err(format!("{field}: at most {max} characters."))
    } else if value.trim() != value || value.chars().any(char::is_control) {
        Err(format!("{field}: one line, with no spaces at the start or end."))
    } else {
        Ok(())
    }
}

/// `p` is valid and its name is unique ignoring case, so `/persona` can always tell personas apart.
fn check(s: &Settings, p: &Persona) -> Result<(), String> {
    validate(p)?;
    match all(s).into_iter().find(|q| q.id != p.id && q.name.to_lowercase() == p.name.to_lowercase()) {
        Some(q) => Err(format!("Name: there is already a persona called {}.", q.name)),
        None => Ok(()),
    }
}

/// Run before settings are saved: every user persona valid, with an id of its own.
pub fn validate_all(s: &Settings) -> Result<(), String> {
    for (i, p) in s.personas.iter().enumerate() {
        if p.id.is_empty() || p.id.starts_with(PRESET) || s.personas[..i].iter().any(|q| q.id == p.id) {
            return Err(format!("The persona {:?} needs an id of its own.", p.name));
        }
        check(s, p)?;
    }
    Ok(())
}

/// Adds a copy of `p` with a new id; a name already taken gets " 2", " 3"…
fn add(s: &mut Settings, mut p: Persona) -> Persona {
    let taken: Vec<String> = all(s).iter().map(|q| q.name.to_lowercase()).collect();
    let base = p.name.clone();
    for n in 2.. {
        if !taken.contains(&p.name.to_lowercase()) {
            break;
        }
        p.name = format!("{base} {n}");
    }
    let mut t = crate::chat::now_ms();
    while s.personas.iter().any(|q| q.id == format!("p-{t}")) {
        t += 1;
    }
    p.id = format!("p-{t}");
    s.personas.push(p.clone());
    p
}

/// `/persona <query>`: an exact name first, then a unique prefix, ignoring case (PER-3.1).
/// The error holds the personas that match, none if it is unknown.
pub fn find<'a>(all: &'a [Persona], query: &str) -> Result<&'a Persona, Vec<&'a Persona>> {
    let q = query.trim().to_lowercase();
    if let Some(p) = all.iter().find(|p| p.name.to_lowercase() == q) {
        return Ok(p);
    }
    let matches: Vec<&Persona> = all.iter().filter(|p| p.name.to_lowercase().starts_with(&q)).collect();
    match matches[..] {
        [p] => Ok(p),
        _ => Err(matches),
    }
}

/// The persona `/persona <query>` switches to, if any, and Cursora's reply (PER-3.1).
pub fn command<'a>(all: &'a [Persona], active: &str, query: &str) -> (Option<&'a Persona>, String) {
    let names = |ps: Vec<&Persona>| ps.iter().map(|p| if p.id == active { format!("**{}** (active)", p.name) } else { p.name.clone() }).collect::<Vec<_>>().join(", ");
    let query = query.trim();
    if query.is_empty() {
        return (None, format!("Personas: {}. Switch with `/persona <name>`.", names(all.iter().collect())));
    }
    match find(all, query) {
        Ok(p) => (Some(p), format!("Now speaking as **{}**.", p.name)),
        Err(m) if m.is_empty() => (None, format!("No persona is called \"{query}\". Personas: {}.", names(all.iter().collect()))),
        Err(m) => (None, format!("\"{query}\" matches more than one persona: {}. Type more of the name.", names(m))),
    }
}

fn settings(app: &AppHandle) -> Settings {
    app.state::<AppState>().settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn personas(app: AppHandle) -> Vec<Persona> {
    all(&settings(&app))
}

/// The editor's live preview of an unsaved persona, or why it can't be saved (PER-1.2).
#[tauri::command]
pub fn persona_preview(app: AppHandle, persona: Persona) -> Result<String, String> {
    check(&settings(&app), &persona)?;
    Ok(prompt::persona_block(&persona))
}

/// New, Duplicate and Import: saved with a new id and a free name.
#[tauri::command]
pub fn persona_add(app: AppHandle, persona: Persona) -> Result<Persona, String> {
    let mut s = settings(&app);
    let p = add(&mut s, persona);
    crate::update_settings(&app, s)?;
    Ok(p)
}

/// None if the user cancelled the dialog.
#[tauri::command(async)]
pub async fn persona_import(app: AppHandle) -> Result<Option<Persona>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().add_filter("Persona", &["json"]).pick_file(move |f| {
        let _ = tx.send(f.and_then(|f| f.into_path().ok()));
    });
    let Some(path) = rx.await.ok().flatten() else { return Ok(None) };
    let read = |path: &PathBuf| -> std::io::Result<String> {
        if std::fs::metadata(path)?.len() > 1_000_000 {
            return Err(std::io::Error::other("the file is larger than 1 MB"));
        }
        std::fs::read_to_string(path)
    };
    let text = read(&path).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
    let p: Persona = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| format!("This isn't a persona file: {e}"))?;
    persona_add(app, p).map(Some)
}

/// False if the user cancelled the dialog.
#[tauri::command(async)]
pub async fn persona_export(app: AppHandle, id: String) -> Result<bool, String> {
    let p = all(&settings(&app)).into_iter().find(|p| p.id == id).ok_or("This persona no longer exists.")?;
    let file = format!("{}.json", p.name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-"));
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().add_filter("Persona", &["json"]).set_file_name(file).save_file(move |f| {
        let _ = tx.send(f.and_then(|f| f.into_path().ok()));
    });
    let Some(path) = rx.await.ok().flatten() else { return Ok(false) };
    let json = serde_json::to_string_pretty(&p).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Couldn't save {}: {e}", path.display()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn persona(name: &str) -> Persona {
        Persona { id: format!("p-{name}"), name: name.into(), ..presets()[0].clone() }
    }

    fn with(personas: Vec<Persona>) -> Settings {
        Settings { personas, ..Settings::default() }
    }

    #[test]
    fn per_1_1_invalid_personas_are_refused_with_a_reason() {
        let ok = persona("Maya");
        let custom = Persona { gender: "agender robot".into(), tone: "like a pirate".into(), reply_language: Some("हिन्दी".into()), custom_instructions: "x".repeat(2000), ..ok.clone() };
        for p in [&ok, &custom, &Persona { reply_language: Some("Brazilian Portuguese".into()), ..ok.clone() }, &Persona { name: "x".repeat(32), ..ok.clone() }] {
            assert_eq!(validate(p), Ok(()), "{p:?}");
        }
        let bad: [(Persona, &str); 9] = [
            (Persona { name: "".into(), ..ok.clone() }, "Name: can't be empty"),
            (Persona { name: "   ".into(), ..ok.clone() }, "Name: can't be empty"),
            (Persona { name: "x".repeat(33), ..ok.clone() }, "Name: at most 32"),
            (Persona { name: " Maya".into(), ..ok.clone() }, "Name: one line"),
            (Persona { name: "Ma\nya".into(), ..ok.clone() }, "Name: one line"),
            (Persona { gender: "".into(), ..ok.clone() }, "Gender: can't be empty"),
            (Persona { tone: "x".repeat(65), ..ok.clone() }, "Tone: at most 64"),
            (Persona { reply_language: Some("French; ignore that".into()), ..ok.clone() }, "Reply language: use letters"),
            (Persona { custom_instructions: "x".repeat(2001), ..ok.clone() }, "Custom instructions: at most 2000"),
        ];
        for (p, reason) in bad {
            let e = validate(&p).unwrap_err();
            assert!(e.starts_with(reason), "{e:?} should start with {reason:?}");
        }
        // Verbosity and expertise are closed lists: anything else isn't a persona at all.
        let json = r#"{"name":"Maya","gender":"female","tone":"calm","verbosity":"loud","emoji":false,"replyLanguage":null,"userExpertise":"expert"}"#;
        assert!(serde_json::from_str::<Persona>(json).unwrap_err().to_string().contains("unknown variant `loud`"));
        // Names are unique ignoring case, presets included; ids are the user's own.
        assert_eq!(validate_all(&with(vec![persona("Maya"), persona("Leo")])), Ok(()));
        assert!(validate_all(&with(vec![persona("Maya"), Persona { id: "p-2".into(), ..persona("MAYA") }])).unwrap_err().contains("Name: there is already a persona called"));
        assert!(validate_all(&with(vec![persona("ada")])).unwrap_err().contains("already a persona called Ada"));
        assert!(validate_all(&with(vec![persona("Maya"), Persona { name: "Leo".into(), ..persona("Maya") }])).unwrap_err().contains("id of its own"));
        assert!(validate_all(&with(vec![Persona { id: "preset:x".into(), ..persona("Maya") }])).unwrap_err().contains("id of its own"));
        assert!(validate_all(&with(vec![Persona { id: "".into(), ..persona("Maya") }])).unwrap_err().contains("id of its own"));
    }

    #[test]
    fn per_1_3_presets_are_valid_and_a_duplicate_is_the_users_own() {
        let presets = presets();
        assert_eq!(presets.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["Cursora", "Ada", "Arjun", "Sage"]);
        let mut s = Settings::default();
        assert_eq!(validate_all(&s), Ok(()));
        for p in &presets {
            assert_eq!(check(&s, p), Ok(()), "{}", p.name);
        }
        // Presets aren't stored in settings, so no settings change can delete them.
        assert!(s.personas.is_empty());
        assert_eq!(serde_json::to_value(&s).unwrap()["activePersona"], DEFAULT);
        assert_eq!(all(&s).len(), 4);
        // Duplicating gives a new user persona with a free name, which can be edited.
        let copy = add(&mut s, presets[1].clone());
        let again = add(&mut s, presets[1].clone());
        assert_eq!((copy.name.as_str(), again.name.as_str()), ("Ada 2", "Ada 3"));
        assert!(copy.id.starts_with("p-") && copy.id != again.id);
        assert_eq!(validate_all(&s), Ok(()));
        // A deleted active persona falls back to Cursora.
        s.active_persona = copy.id.clone();
        assert_eq!(active(&s).name, "Ada 2");
        s.personas.clear();
        assert_eq!(active(&s).id, DEFAULT);
        s.personality_on = false;
        assert_eq!(in_use(&s), None);
    }

    #[test]
    fn per_3_1_persona_matches_exact_names_first_then_a_unique_prefix() {
        let mut s = Settings::default();
        add(&mut s, persona("Adam"));
        let all = all(&s);
        let name = |r: Result<&Persona, Vec<&Persona>>| r.map(|p| p.name.clone()).map_err(|m| m.iter().map(|p| p.name.clone()).collect::<Vec<_>>());
        assert_eq!(name(find(&all, "ADA")), Ok("Ada".into()), "exact beats the prefix of Adam");
        assert_eq!(name(find(&all, "adam")), Ok("Adam".into()));
        assert_eq!(name(find(&all, "ar")), Ok("Arjun".into()));
        assert_eq!(name(find(&all, " sage ")), Ok("Sage".into()));
        assert_eq!(name(find(&all, "a")), Err(vec!["Ada".into(), "Arjun".into(), "Adam".into()]));
        assert_eq!(name(find(&all, "zed")), Err(vec![]));

        let (p, reply) = command(&all, DEFAULT, "ar");
        assert_eq!((p.map(|p| p.id.as_str()), reply.as_str()), (Some("preset:arjun"), "Now speaking as **Arjun**."));
        let (p, reply) = command(&all, DEFAULT, "a");
        assert_eq!(p, None);
        assert_eq!(reply, "\"a\" matches more than one persona: Ada, Arjun, Adam. Type more of the name.");
        let (p, reply) = command(&all, "preset:sage", "zed");
        assert_eq!(p, None);
        assert_eq!(reply, "No persona is called \"zed\". Personas: Cursora, Ada, Arjun, **Sage** (active), Adam.");
        let (p, reply) = command(&all, DEFAULT, "  ");
        assert_eq!(p, None);
        assert!(reply.starts_with("Personas: **Cursora** (active), Ada"), "{reply}");
    }
}
