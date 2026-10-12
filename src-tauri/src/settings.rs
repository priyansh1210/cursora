use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, io, path::Path};

/// Later specs add fields with defaults; `#[serde(default)]` keeps older files loading (APP-4.3).
/// Add an explicit migration step only when a field is renamed or changes type.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: u32,
    pub setup_done: bool,
    pub paused: bool,
    pub start_with_system: bool,
    pub theme: Theme,
    pub hotkeys: Hotkeys,
    pub personality_on: bool,
    pub local_only: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            setup_done: false,
            paused: false,
            start_with_system: true,
            theme: Theme::System,
            hotkeys: Hotkeys::default(),
            personality_on: true,
            local_only: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Hotkeys {
    pub ask: String,
    pub snip: String,
    pub push_to_talk: Option<String>,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self { ask: "Ctrl+G".into(), snip: "Ctrl+Shift+G".into(), push_to_talk: None }
    }
}

/// Missing file → defaults. Unreadable or invalid file → moved to `settings.bak-<ts>.json`,
/// defaults returned with a warning for the user (APP-4.2).
pub fn load(path: &Path) -> (Settings, Option<String>) {
    let err = match fs::read_to_string(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return (Settings::default(), None),
        Err(e) => e.to_string(),
        Ok(text) => match serde_json::from_str(&text) {
            Ok(s) => return (s, None),
            Err(e) => e.to_string(),
        },
    };
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let backup = path.with_file_name(format!("settings.bak-{ts}.json"));
    let warning = match fs::rename(path, &backup) {
        Ok(()) => format!("Cursora couldn't read its settings ({err}). The old file was saved as {} and defaults are used.", backup.display()),
        Err(_) => format!("Cursora couldn't read its settings ({err}). Defaults are used."),
    };
    (Settings::default(), Some(warning))
}

/// Writes a temp file then renames it over the old one, so a crash never leaves a half-written file.
pub fn save(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(settings)?)?;
    fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cursora-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir.join("settings.json")
    }

    #[test]
    fn app_4_1_saved_settings_load_back() {
        let path = temp_file("roundtrip");
        let s = Settings {
            paused: true,
            theme: Theme::Dark,
            hotkeys: Hotkeys { ask: "Alt+Q".into(), push_to_talk: Some("Ctrl+Alt+Space".into()), ..Hotkeys::default() },
            ..Settings::default()
        };
        save(&path, &s).unwrap();
        assert_eq!(load(&path), (s, None));
    }

    #[test]
    fn app_4_2_missing_file_gives_defaults() {
        assert_eq!(load(&temp_file("missing")), (Settings::default(), None));
    }

    #[test]
    fn app_4_2_invalid_file_is_backed_up_and_defaults_used() {
        let path = temp_file("invalid");
        fs::write(&path, "{ not json").unwrap();
        let (s, warning) = load(&path);
        assert_eq!(s, Settings::default());
        assert!(warning.is_some());
        assert!(!path.exists());
        let backups: Vec<PathBuf> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with("settings.bak-"))
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read_to_string(&backups[0]).unwrap(), "{ not json");
    }

    #[test]
    fn app_4_3_older_file_keeps_user_values_and_fills_defaults() {
        let path = temp_file("older");
        fs::write(&path, r#"{"schemaVersion":1,"paused":true,"hotkeys":{"ask":"Alt+Q"}}"#).unwrap();
        let (s, warning) = load(&path);
        assert_eq!(warning, None);
        assert!(s.paused);
        assert_eq!(s.hotkeys.ask, "Alt+Q");
        assert_eq!(s.hotkeys.snip, "Ctrl+Shift+G");
        assert!(s.start_with_system);
    }

    #[test]
    fn app_4_4_settings_have_no_secret_fields() {
        fn names(v: &serde_json::Value, out: &mut Vec<String>) {
            if let serde_json::Value::Object(map) = v {
                for (k, v) in map {
                    out.push(k.clone());
                    names(v, out);
                }
            }
        }
        let mut all = Vec::new();
        names(&serde_json::to_value(Settings::default()).unwrap(), &mut all);
        for n in all {
            let lower = n.to_lowercase();
            let secret = lower == "key" || n.ends_with("Key") || ["token", "secret", "password"].iter().any(|w| lower.contains(w));
            assert!(!secret, "settings field {n:?} looks like a secret; secrets belong in the OS keychain");
        }
    }
}
