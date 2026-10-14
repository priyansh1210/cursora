use crate::capture::{CopyKey, Rule};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    /// Windows: start as administrator through a Task Scheduler task (APP-3.3).
    pub run_as_admin: bool,
    pub theme: Theme,
    pub hotkeys: Hotkeys,
    pub personality_on: bool,
    /// A preset id ("preset:cursora") or one of `personas` (spec 05).
    pub active_persona: String,
    /// Only the user's own; the presets live in code.
    pub personas: Vec<crate::persona::Persona>,
    pub local_only: bool,
    pub capture: CaptureSettings,
    pub privacy: crate::privacy::PrivacySettings,
    pub history: HistorySettings,
    pub models: crate::llm::ModelSettings,
    pub snip: SnipSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            setup_done: false,
            paused: false,
            start_with_system: true,
            run_as_admin: false,
            theme: Theme::System,
            hotkeys: Hotkeys::default(),
            personality_on: true,
            active_persona: crate::persona::DEFAULT.into(),
            personas: vec![],
            local_only: false,
            capture: CaptureSettings::default(),
            privacy: crate::privacy::PrivacySettings::default(),
            history: HistorySettings::default(),
            models: crate::llm::ModelSettings::default(),
            snip: SnipSettings::default(),
        }
    }
}

/// Spec 02. The four limits have no UI; they are editable in settings.json.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct CaptureSettings {
    /// Apps where Ctrl+C may interrupt a process (SEL-4.3).
    pub unsafe_apps: Vec<String>,
    /// Per-app copy key, by exe name (SEL-4.4).
    pub copy_shortcuts: BTreeMap<String, CopyKey>,
    pub share_window_title: bool,
    pub max_chars: usize,
    pub copy_timeout_ms: u64,
    pub file_max_bytes: u64,
    pub files_total_bytes: u64,
    /// Apps Cursora never reads from (PRI-2).
    pub blocklist: Vec<Rule>,
}

impl Default for CaptureSettings {
    fn default() -> Self {
        let unsafe_apps = [
            "WindowsTerminal.exe", "conhost.exe", "OpenConsole.exe", "cmd.exe", "powershell.exe", "pwsh.exe",
            "mintty.exe", "alacritty.exe", "wezterm-gui.exe", "Code.exe", "Cursor.exe", "idea64.exe", "pycharm64.exe",
        ];
        // PRI-2.3: password managers and the Windows credential prompts, then their Linux process names.
        let blocked = [
            "KeePass.exe", "KeePassXC.exe", "1Password.exe", "Bitwarden.exe", "Enpass.exe", "CredentialUIBroker.exe", "consent.exe",
            "keepassxc", "bitwarden", "1password",
        ];
        Self {
            unsafe_apps: unsafe_apps.map(String::from).to_vec(),
            copy_shortcuts: BTreeMap::new(),
            share_window_title: false,
            max_chars: 50_000,
            copy_timeout_ms: 150,
            file_max_bytes: 100_000,
            files_total_bytes: 300_000,
            blocklist: blocked.map(|p| Rule::Process(p.into())).to_vec(),
        }
    }
}

/// Spec 03 (BUB-9).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct HistorySettings {
    pub save: bool,
    /// `None` keeps conversations forever.
    pub retention_days: Option<u32>,
}

impl Default for HistorySettings {
    fn default() -> Self {
        Self { save: true, retention_days: Some(90) }
    }
}

/// Spec 10. No UI; editable in settings.json, like the capture limits.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct SnipSettings {
    /// The long side of an image sent to the model, snips and image files alike (VIS-2.2, 2.7).
    pub max_side: u32,
}

impl Default for SnipSettings {
    fn default() -> Self {
        Self { max_side: 2048 }
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
        Ok(text) => match parse(&text) {
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

/// Notepad and PowerShell 5 may start the file with a UTF-8 byte order mark, which JSON doesn't allow.
pub fn parse(text: &str) -> serde_json::Result<Settings> {
    serde_json::from_str(text.trim_start_matches('\u{feff}'))
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
    fn app_4_2_a_file_saved_with_a_byte_order_mark_still_loads() {
        let path = temp_file("bom");
        fs::write(&path, "\u{feff}{\"paused\":true}").unwrap();
        let (s, warning) = load(&path);
        assert_eq!(warning, None);
        assert!(s.paused);
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
        // One provider of every kind, so the provider fields are walked too (the defaults have none).
        use crate::llm::{tests::provider, Kind, ModelRef};
        let mut s = Settings::default();
        s.models.providers = [Kind::OpenAiCompatible, Kind::Anthropic, Kind::Gemini, Kind::Builtin].map(|k| provider("p", k, "http://localhost")).to_vec();
        s.models.default = Some(ModelRef { provider: "p".into(), model: "m".into() });
        let mut all = Vec::new();
        names(&serde_json::to_value(s).unwrap(), &mut all);
        for n in all {
            let lower = n.to_lowercase();
            let secret = lower == "key" || n.ends_with("Key") || ["token", "secret", "password"].iter().any(|w| lower.contains(w));
            assert!(!secret, "settings field {n:?} looks like a secret; secrets belong in the OS keychain");
        }
    }
}
