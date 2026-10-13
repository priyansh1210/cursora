//! Skills (spec 06): Markdown files with a few fields on top, built-in or the user's own.
//! The fields are a small, strict subset of YAML: `key: value` lines, lists, comments.
use crate::chat::ModelInfo;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

// The system commands (SKL-2.6). chat.rs dispatches on these names; skills can't use them.
pub const PERSONA: &str = "persona";
pub const NEW_SKILL: &str = "new-skill";
pub const CLEAR: &str = "clear";
pub const HELP: &str = "help";
/// Reserved now; it works from spec 10, and until then it is left out of /help and the autocomplete.
pub const SNIP: &str = "snip";
/// The commands that work today: name, args hint, description.
const COMMANDS: [(&str, &str, &str); 4] = [
    (PERSONA, "<name>", "Switch to another persona"),
    (NEW_SKILL, "", "Make a skill from your last question"),
    (CLEAR, "", "Start over with the same selection"),
    (HELP, "", "List the skills and commands"),
];

const VARIABLES: [&str; 4] = ["input", "app", "language", "date"];
const FIELDS: [&str; 7] = ["name", "description", "args", "requires", "model", "temperature", "output"];
const MAX_BYTES: u64 = 64 * 1024;

/// SKL-3.1, in the same format as the user's files.
const BUILT_IN: [&str; 9] = [
    include_str!("../skills/summary.md"),
    include_str!("../skills/explain.md"),
    include_str!("../skills/simplify.md"),
    include_str!("../skills/define.md"),
    include_str!("../skills/translate.md"),
    include_str!("../skills/fix.md"),
    include_str!("../skills/rewrite.md"),
    include_str!("../skills/code.md"),
    include_str!("../skills/bugs.md"),
];

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub args: Option<String>,
    /// `requires: [vision]`
    pub vision: bool,
    /// `<provider>/<model>`
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub template: String,
    /// None for a built-in.
    pub file: Option<PathBuf>,
    /// A user skill that replaces the built-in of the same name.
    pub overrides: bool,
}

/// A file that isn't a usable skill (SKL-1.3). `line` is 0 when the whole file is the problem.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Invalid {
    pub file: PathBuf,
    pub line: usize,
    pub error: String,
}

/// The skill form's fields (SKL-4.1); Rust writes the file from them.
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Fields {
    pub name: String,
    pub description: String,
    pub args: String,
    pub vision: bool,
    pub model: String,
    pub temperature: Option<f64>,
    pub template: String,
}

/// The values a template's variables stand for (SKL-1.2).
pub struct Vars<'a> {
    pub input: &'a str,
    pub app: &'a str,
    pub language: &'a str,
    pub date: &'a str,
}

/// Reads one skill file. An error carries its 1-based line.
pub fn parse(text: &str) -> Result<Skill, (usize, String)> {
    let lines: Vec<&str> = text.trim_start_matches('\u{feff}').lines().collect();
    if lines.first().map(|l| l.trim_end()) != Some("---") {
        return Err((1, "a skill starts with a '---' line, then its fields".into()));
    }
    let end = lines.iter().skip(1).position(|l| l.trim_end() == "---").map(|i| i + 1).ok_or((lines.len(), "the fields never end: add a '---' line after them".into()))?;
    let mut fields: Vec<(&str, usize, String)> = vec![]; // key, line, value
    let mut list: Option<usize> = None; // the `requires:` entry that block items go into
    let mut requires: Vec<(usize, String)> = vec![];
    for (i, raw) in lines.iter().enumerate().take(end).skip(1) {
        let n = i + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(item) = line.strip_prefix("- ").or((line == "-").then_some("")) {
            if list.is_none() {
                return Err((n, "a list item with no 'requires:' above it".into()));
            }
            requires.push((n, unquote(item.trim())));
            continue;
        }
        list = None;
        let (key, value) = line.split_once(':').ok_or((n, "expected 'field: value'".into()))?;
        let key = FIELDS.iter().find(|f| **f == key.trim()).ok_or_else(|| (n, format!("unknown field '{}'", key.trim())))?;
        if fields.iter().any(|(k, ..)| k == key) {
            return Err((n, format!("'{key}' is set twice")));
        }
        let value = value.trim();
        if *key == "requires" {
            match value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
                Some(inner) => requires.extend(inner.split(',').map(str::trim).filter(|v| !v.is_empty()).map(|v| (n, unquote(v)))),
                None if value.is_empty() => list = Some(n),
                None => requires.push((n, unquote(value))),
            }
        }
        fields.push((key, n, unquote(value)));
    }
    let get = |key: &str| fields.iter().find(|(k, ..)| *k == key).map(|(_, n, v)| (*n, v.as_str()));
    let missing = |key: &str| (end + 1, format!("missing field '{key}'"));

    let (n, name) = get("name").ok_or_else(|| missing("name"))?;
    if !valid_name(name) {
        return Err((n, format!("'{name}' isn't a valid name: use 1 to 32 lowercase letters, digits and hyphens, starting with a letter or digit")));
    }
    if name == SNIP || COMMANDS.iter().any(|c| c.0 == name) {
        return Err((n, format!("/{name} is a system command, so a skill can't use that name")));
    }
    let (n, description) = get("description").ok_or_else(|| missing("description"))?;
    if description.is_empty() || description.chars().count() > 120 {
        return Err((n, "'description' must be 1 to 120 characters".into()));
    }
    let args = match get("args") {
        Some((n, a)) if a.chars().count() > 40 => return Err((n, "'args' must be at most 40 characters".into())),
        Some((_, a)) if !a.is_empty() => Some(a.to_string()),
        _ => None,
    };
    if let Some((n, r)) = requires.iter().find(|(_, r)| r != "vision") {
        return Err((*n, format!("'requires' can only list 'vision', not '{r}'")));
    }
    let model = match get("model") {
        Some((n, m)) => match m.split_once('/') {
            Some((p, id)) if !p.trim().is_empty() && !id.trim().is_empty() => Some(m.to_string()),
            _ => return Err((n, "'model' must be written as <provider>/<model>, e.g. Ollama/llama3.2".into())),
        },
        None => None,
    };
    let temperature = match get("temperature") {
        Some((n, t)) => match t.parse::<f64>() {
            Ok(t) if (0.0..=2.0).contains(&t) => Some(t),
            _ => return Err((n, "'temperature' must be a number from 0 to 2".into())),
        },
        None => None,
    };
    if let Some((n, o)) = get("output").filter(|(_, o)| *o != "chat") {
        return Err((n, format!("'output' can only be 'chat', not '{o}'")));
    }
    // SKL-1.2: every {{variable}} is known; they don't span lines.
    for (i, line) in lines.iter().enumerate().skip(end + 1) {
        let mut rest = *line;
        while let Some(start) = rest.find("{{") {
            let close = rest[start..].find("}}").ok_or((i + 1, "'{{' is never closed with '}}'".to_string()))?;
            let var = rest[start + 2..start + close].trim();
            if !VARIABLES.contains(&var) {
                return Err((i + 1, format!("unknown variable {{{{{var}}}}}: use {{{{input}}}}, {{{{app}}}}, {{{{language}}}} or {{{{date}}}}")));
            }
            rest = &rest[start + close + 2..];
        }
    }
    let template = lines[end + 1..].join("\n").trim().to_string();
    if template.is_empty() {
        return Err((end + 2, "the instructions below the fields are empty".into()));
    }
    let vision = !requires.is_empty();
    Ok(Skill { name: name.into(), description: description.into(), args, vision, model, temperature, template, file: None, overrides: false })
}

fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && name.len() <= 32
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// A quoted value loses its quotes; inside double quotes, \" and \\ stand for " and \.
fn unquote(v: &str) -> String {
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        v[1..v.len() - 1].replace("\\\"", "\"").replace("\\\\", "\\")
    } else if v.len() >= 2 && v.starts_with('\'') && v.ends_with('\'') {
        v[1..v.len() - 1].replace("''", "'")
    } else {
        v.to_string()
    }
}

/// Quoted only when plain YAML would misread it.
fn quote(v: &str) -> String {
    let plain = !v.contains(": ") && !v.contains(" #") && !v.starts_with(['"', '\'', '[', '{', '&', '*', '!', '|', '>', '%', '@', '`', '-', '#']);
    if plain { v.to_string() } else { format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\"")) }
}

/// The file the form's fields make.
pub fn to_file(f: &Fields) -> String {
    let mut out = format!("---\nname: {}\ndescription: {}\n", f.name.trim(), quote(f.description.trim()));
    if !f.args.trim().is_empty() {
        out += &format!("args: {}\n", quote(f.args.trim()));
    }
    if f.vision {
        out += "requires: [vision]\n";
    }
    if !f.model.trim().is_empty() {
        out += &format!("model: {}\n", quote(f.model.trim()));
    }
    if let Some(t) = f.temperature {
        out += &format!("temperature: {t}\n");
    }
    out + "---\n" + f.template.trim() + "\n"
}

/// SKL-1.2. Unknown variables can't occur: `parse` refuses them.
pub fn render(template: &str, v: &Vars) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let Some(len) = rest[start..].find("}}") else { break };
        let close = start + len;
        out.push_str(&rest[..start]);
        out.push_str(match rest[start + 2..close].trim() {
            "input" => v.input,
            "app" => v.app,
            "language" => v.language,
            "date" => v.date,
            _ => &rest[start..close + 2],
        });
        rest = &rest[close + 2..];
    }
    out + rest
}

/// The built-ins, then the user's files in alphabetical order (SKL-1.4).
/// A user skill named like a built-in replaces it; a name already taken by a user file makes the later file invalid.
pub fn load(dir: Option<&Path>) -> (Vec<Skill>, Vec<Invalid>) {
    let mut skills: Vec<Skill> = BUILT_IN.iter().filter_map(|text| parse(text).ok()).collect();
    let mut invalid = vec![];
    let mut files: Vec<PathBuf> = dir
        .and_then(|d| std::fs::read_dir(d).ok())
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("md")))
        .collect();
    files.sort();
    for file in files {
        let skill = read(&file).and_then(|text| parse(&text).map_err(|(line, error)| Invalid { file: file.clone(), line, error }));
        let mut skill = match skill {
            Ok(s) => s,
            Err(e) => {
                invalid.push(e);
                continue;
            }
        };
        skill.file = Some(file.clone());
        match skills.iter_mut().find(|s| s.name == skill.name) {
            Some(taken) if taken.file.is_some() => {
                let other = taken.file.as_deref().and_then(Path::file_name).map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                invalid.push(Invalid { file, line: 0, error: format!("the name '{}' is already used by {other}", skill.name) });
            }
            Some(built_in) => *built_in = Skill { overrides: true, ..skill },
            None => skills.push(skill),
        }
    }
    (skills, invalid)
}

fn read(file: &Path) -> Result<String, Invalid> {
    let whole = |error: String| Invalid { file: file.to_path_buf(), line: 0, error };
    let size = std::fs::metadata(file).map_err(|e| whole(e.to_string()))?.len();
    if size > MAX_BYTES {
        return Err(whole("the file is larger than 64 KB".into()));
    }
    std::fs::read_to_string(file).map_err(|_| whole("the file isn't UTF-8 text".into()))
}

/// Up to 3 names within 2 edits of the typed one, or starting with it, nearest first (SKL-2.3).
pub fn closest<'a>(names: impl Iterator<Item = &'a str>, typed: &str) -> Vec<&'a str> {
    let mut near: Vec<(usize, &str)> = names.map(|n| (distance(n, typed), n)).filter(|(d, n)| *d <= 2 || (!typed.is_empty() && n.starts_with(typed))).collect();
    near.sort();
    near.dedup_by_key(|(_, n)| *n);
    near.into_iter().take(3).map(|(_, n)| n).collect()
}

fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != *cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

/// The reply to a command that is neither a system command nor a skill (SKL-2.3).
pub fn unknown(skills: &[Skill], typed: &str) -> String {
    let near = closest(skills.iter().map(|s| s.name.as_str()).chain(COMMANDS.iter().map(|c| c.0)), typed);
    if near.is_empty() {
        format!("Unknown skill `/{typed}`. Type `/help` to see the skills and commands.")
    } else {
        format!("Unknown skill `/{typed}`. Did you mean {}?", near.iter().map(|n| format!("`/{n}`")).collect::<Vec<_>>().join(", "))
    }
}

/// Why the model can't run this skill, if it can't (SKL-2.4).
pub fn refusal(s: &Skill, m: &ModelInfo) -> Option<String> {
    (s.vision && !m.vision).then(|| format!("`/{}` needs a vision model, and {} can't see images.", s.name, m.name))
}

/// One line of the autocomplete or of `/help`.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub name: String,
    pub args: String,
    pub description: String,
    pub command: bool,
}

/// The skills the model can run (SKL-2.4), then the commands.
pub fn entries(skills: &[Skill], can_see: impl Fn(&Skill) -> bool) -> Vec<Entry> {
    let skills = skills.iter().filter(|s| !s.vision || can_see(s)).map(|s| Entry { name: s.name.clone(), args: s.args.clone().unwrap_or_default(), description: s.description.clone(), command: false });
    let commands = COMMANDS.iter().map(|(n, a, d)| Entry { name: n.to_string(), args: a.to_string(), description: d.to_string(), command: true });
    skills.chain(commands).collect()
}

/// `/help` (SKL-2.7).
pub fn help(entries: &[Entry]) -> String {
    let line = |e: &Entry| format!("- `/{}{}` {}", e.name, if e.args.is_empty() { String::new() } else { format!(" {}", e.args) }, e.description);
    let (commands, skills): (Vec<&Entry>, Vec<&Entry>) = entries.iter().partition(|e| e.command);
    format!(
        "**Skills**\n{}\n\n**Commands**\n{}",
        skills.into_iter().map(line).collect::<Vec<_>>().join("\n"),
        commands.into_iter().map(line).collect::<Vec<_>>().join("\n")
    )
}

/// Today's local date for `{{date}}`.
pub fn today() -> String {
    #[cfg(windows)]
    {
        // SAFETY: GetLocalTime only fills in the returned struct.
        let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
        format!("{:04}-{:02}-{:02}", t.wYear, t.wMonth, t.wDay)
    }
    #[cfg(not(windows))]
    {
        // SAFETY: localtime_r writes only into `tm`, which outlives the call.
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        unsafe { libc::localtime_r(&libc::time(std::ptr::null_mut()), &mut tm) };
        format!("{:04}-{:02}-{:02}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday)
    }
}

/// Writes the form's skill as `<name>.md` (SKL-4.4). `editing` is the file being edited, if any; a rename removes it.
pub fn save(dir: &Path, f: &Fields, editing: Option<&Path>) -> Result<PathBuf, String> {
    let text = to_file(f);
    let skill = parse(&text).map_err(|(_, e)| e)?;
    if let Some(p) = editing {
        inside(dir, p)?;
    }
    let target = write(dir, &skill.name, &text, editing)?;
    if let Some(old) = editing.filter(|old| !same_file(old, &target)) {
        let _ = std::fs::remove_file(old);
    }
    Ok(target)
}

/// Writes a skill called `name` as `<name>.md`, unless another file already has that name or that file name.
/// `editing` is the file being replaced, which may keep its own name.
fn write(dir: &Path, name: &str, text: &str, editing: Option<&Path>) -> Result<PathBuf, String> {
    let ours = |p: &Path| editing.is_some_and(|e| same_file(e, p));
    if let Some(other) = load(Some(dir)).0.into_iter().filter_map(|s| s.file.filter(|_| s.name == name)).find(|p| !ours(p)) {
        return Err(format!("{} already uses the name '{name}'", other.display()));
    }
    let target = dir.join(format!("{name}.md"));
    if target.exists() && !ours(&target) {
        return Err(format!("{} already exists in the skills folder", target.display()));
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(&target, text).map_err(|e| format!("Couldn't save {}: {e}", target.display()))?;
    Ok(target)
}

/// One file, also when the two names differ only in letter case on Windows (`ELI5.md` is `eli5.md` there).
fn same_file(a: &Path, b: &Path) -> bool {
    a == b || matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b)
}

/// A path from the UI must be a skill file in the skills folder, nothing else.
fn inside(dir: &Path, p: &Path) -> Result<(), String> {
    let ok = p.parent() == Some(dir) && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("md"));
    ok.then_some(()).ok_or_else(|| format!("{} isn't in the skills folder", p.display()))
}

/// The user's skills folder; Settings' "Open folder" uses it too.
pub fn dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map(|d| d.join("skills")).map_err(|e| e.to_string())
}

/// `{{language}}`: the persona's reply language, if personality is on and it has one (SKL-1.2).
pub fn language(persona: Option<&crate::persona::Persona>) -> String {
    persona.and_then(|p| p.reply_language.clone()).unwrap_or_else(|| crate::prompt::QUESTION_LANGUAGE.into())
}

fn persona_in_use(app: &AppHandle) -> Option<crate::persona::Persona> {
    crate::persona::in_use(&app.state::<crate::AppState>().settings.lock().unwrap())
}

/// The skill called `name`, read from the files now.
pub fn find(app: &AppHandle, name: &str) -> Option<Skill> {
    load(dir(app).ok().as_deref()).0.into_iter().find(|s| s.name == name)
}

/// A template waiting for the Skills tab: from `/new-skill` or "Save as skill" (SKL-4.2, 4.3).
#[derive(Default)]
pub struct Draft(Mutex<Option<String>>);

/// Keeps the template for the form, then opens Settings → Skills.
pub fn draft(app: &AppHandle, template: String) {
    *app.state::<Draft>().0.lock().unwrap() = Some(template);
    let app = app.clone();
    // A new window from a synchronous command deadlocks on Windows (wry#583), so from another thread.
    std::thread::spawn(move || {
        crate::open_window_at(&app, "settings", "Skills");
        let _ = app.emit_to("settings", "skill-draft", ());
    });
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    skills: Vec<Skill>,
    invalid: Vec<Invalid>,
}

/// What the answering model can run, then the commands (SKL-2.4): the autocomplete and `/help`.
pub fn available(app: &AppHandle) -> Vec<Entry> {
    let (skills, _) = load(dir(app).ok().as_deref());
    entries(&skills, |s| crate::llm::for_skill(app, s.model.as_deref(), None).is_ok_and(|(m, _)| m.info().vision))
}

/// The bubble's autocomplete (SKL-2.1).
#[tauri::command]
pub fn skills_available(app: AppHandle) -> Vec<Entry> {
    available(&app)
}

#[tauri::command]
pub fn skills_list(app: AppHandle) -> Listing {
    let (skills, invalid) = load(dir(&app).ok().as_deref());
    Listing { skills, invalid }
}

/// The form's live validation: the instructions as they will read, or what is wrong.
#[tauri::command]
pub fn skill_check(app: AppHandle, fields: Fields) -> Result<String, String> {
    let s = parse(&to_file(&fields)).map_err(|(_, e)| e)?;
    let language = language(persona_in_use(&app).as_ref());
    Ok(render(&s.template, &Vars { input: "(text after the command)", app: crate::chat::SAMPLE_APP, language: &language, date: &today() }))
}

#[tauri::command]
pub fn skill_save(app: AppHandle, fields: Fields, editing: Option<PathBuf>) -> Result<(), String> {
    save(&dir(&app)?, &fields, editing.as_deref()).map(|_| ())
}

#[tauri::command]
pub fn skill_delete(app: AppHandle, file: PathBuf) -> Result<(), String> {
    inside(&dir(&app)?, &file)?;
    std::fs::remove_file(&file).map_err(|e| format!("Couldn't delete {}: {e}", file.display()))
}

/// The form's Test button: the skill once on a sample text, with its own model if it names one (SKL-4.1).
#[tauri::command(async)]
pub async fn skill_test(app: AppHandle, fields: Fields, sample: String) -> Result<String, String> {
    let skill = parse(&to_file(&fields)).map_err(|(_, e)| e)?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    // A model blocks while it streams, so it runs on its own thread, as chat answers do.
    std::thread::spawn(move || {
        let _ = tx.send(test(&app, &skill, &sample));
    });
    rx.await.map_err(|e| e.to_string())?
}

fn test(app: &AppHandle, skill: &Skill, sample: &str) -> Result<String, String> {
    let (model, note) = crate::llm::for_skill(app, skill.model.as_deref(), skill.temperature)?;
    if let Some(reason) = refusal(skill, &model.info()) {
        return Err(reason);
    }
    let persona = persona_in_use(app);
    let conv = crate::chat::sample(sample, &format!("/{}", skill.name));
    let instructions = render(&skill.template, &Vars { input: "", app: &conv.app.name, language: &language(persona.as_ref()), date: &today() });
    let (messages, _) = crate::chat::assemble(&conv, persona.as_ref(), Some(&instructions), false, usize::MAX);
    let mut answer = String::new();
    match crate::chat::run_turn(&*model, &messages, &std::sync::atomic::AtomicBool::new(false), &mut |t| answer.push_str(t)) {
        (_, Some(e)) => Err(e.message),
        _ => Ok(note.map_or(answer.clone(), |n| format!("{n}\n\n{answer}"))),
    }
}

/// Takes the waiting template, if any.
#[tauri::command]
pub fn skill_draft(app: AppHandle) -> Option<String> {
    app.state::<Draft>().0.lock().unwrap().take()
}

/// "Save as skill" on a question in the bubble (SKL-4.3).
#[tauri::command]
pub fn skill_new(app: AppHandle, question: String) {
    draft(&app, question);
}

/// Checks the chosen file and copies it into the skills folder. None if the dialog was cancelled.
#[tauri::command(async)]
pub async fn skill_import(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().add_filter("Skill", &["md"]).pick_file(move |f| {
        let _ = tx.send(f.and_then(|f| f.into_path().ok()));
    });
    let Some(path) = rx.await.ok().flatten() else { return Ok(None) };
    let text = read(&path).map_err(|e| e.error)?;
    let skill = parse(&text).map_err(|(line, e)| format!("Line {line}: {e}"))?;
    write(&dir(&app)?, &skill.name, &text, None)?;
    Ok(Some(skill.name))
}

/// Saves a skill's file where the user chooses. False if the dialog was cancelled.
#[tauri::command(async)]
pub async fn skill_export(app: AppHandle, name: String) -> Result<bool, String> {
    use tauri_plugin_dialog::DialogExt;
    let skill = find(&app, &name).ok_or("This skill no longer exists.")?;
    let text = match &skill.file {
        Some(f) => read(f).map_err(|e| e.error)?,
        None => BUILT_IN.iter().find(|t| parse(t).is_ok_and(|s| s.name == name)).map(|t| t.to_string()).unwrap_or_default(),
    };
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().add_filter("Skill", &["md"]).set_file_name(format!("{name}.md")).save_file(move |f| {
        let _ = tx.send(f.and_then(|f| f.into_path().ok()));
    });
    let Some(path) = rx.await.ok().flatten() else { return Ok(false) };
    std::fs::write(&path, text).map_err(|e| format!("Couldn't save {}: {e}", path.display()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(fields: &str, body: &str) -> String {
        format!("---\n{fields}\n---\n{body}\n")
    }

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cursora-skills-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn skl_1_1_fields_are_read_and_checked_with_their_line() {
        let s = parse(&file("# a comment\nname: eli5\ndescription: \"Explain: simply\"\nargs: '<topic>'\nrequires:\n  - vision\nmodel: Ollama/llama3.2\ntemperature: 0.3\noutput: chat", "Explain it.")).unwrap();
        assert_eq!((s.name.as_str(), s.description.as_str(), s.args.as_deref()), ("eli5", "Explain: simply", Some("<topic>")));
        assert_eq!((s.vision, s.model.as_deref(), s.temperature, s.template.as_str()), (true, Some("Ollama/llama3.2"), Some(0.3), "Explain it."));
        assert!(parse(&file("name: a\ndescription: d\nrequires: [vision]", "x")).unwrap().vision);
        assert!(parse(&file("name: a\ndescription: d\nmodel: OpenRouter/meta-llama/llama-3", "x")).is_ok(), "model ids may contain '/'");
        assert!(parse("---\r\nname: a\r\ndescription: d\r\n---\r\nx\r\n").is_ok(), "Windows line ends");
        let cases: [(&str, usize, &str); 16] = [
            ("name: a\ndescription: d\n---\nx", 1, "starts with a '---' line"),
            ("---\nname: a\ndescription: d\nx", 4, "the fields never end"),
            ("---\nname a\ndescription: d\n---\nx", 2, "expected 'field: value'"),
            ("---\nname: a\ndescripton: d\n---\nx", 3, "unknown field 'descripton'"),
            ("---\nname: a\nname: b\ndescription: d\n---\nx", 3, "'name' is set twice"),
            ("---\ndescription: d\n---\nx", 3, "missing field 'name'"),
            ("---\nname: a\n---\nx", 3, "missing field 'description'"),
            ("---\nname: Eli5\ndescription: d\n---\nx", 2, "isn't a valid name"),
            ("---\nname: -x\ndescription: d\n---\nx", 2, "isn't a valid name"),
            (&format!("---\nname: {}\ndescription: d\n---\nx", "a".repeat(33)), 2, "isn't a valid name"),
            (&format!("---\nname: a\ndescription: {}\n---\nx", "d".repeat(121)), 3, "1 to 120 characters"),
            ("---\nname: a\ndescription: d\nrequires: [audio]\n---\nx", 4, "can only list 'vision'"),
            ("---\nname: a\ndescription: d\nmodel: llama3.2\n---\nx", 4, "<provider>/<model>"),
            ("---\nname: a\ndescription: d\ntemperature: 3\n---\nx", 4, "from 0 to 2"),
            ("---\nname: a\ndescription: d\noutput: replace\n---\nx", 4, "can only be 'chat'"),
            ("---\nname: a\ndescription: d\n---\n\n", 5, "instructions below the fields are empty"),
        ];
        for (text, line, error) in cases {
            let (l, e) = parse(text).unwrap_err();
            assert!(e.contains(error) && l == line, "{text:?}: got line {l} {e:?}, want line {line} {error:?}");
        }
        // The form's fields make a file that reads back the same, quoting what plain YAML would misread.
        let f = Fields { name: "eli5".into(), description: "Say \"it\": simply".into(), args: "<topic>".into(), vision: true, model: "Ollama/llama3.2".into(), temperature: Some(0.5), template: "Explain {{input}}.".into() };
        let s = parse(&to_file(&f)).unwrap();
        assert_eq!((s.description.as_str(), s.args.as_deref(), s.vision, s.model.as_deref(), s.temperature, s.template.as_str()), ("Say \"it\": simply", Some("<topic>"), true, Some("Ollama/llama3.2"), Some(0.5), "Explain {{input}}."));
    }

    #[test]
    fn skl_1_2_templates_use_the_four_variables_and_no_others() {
        let s = parse(&file("name: a\ndescription: d", "To {{ language }}: {{input}}\nFrom {{app}} on {{date}}.")).unwrap();
        let v = Vars { input: "French", app: "Notepad", language: "Hindi", date: "2026-10-13" };
        assert_eq!(render(&s.template, &v), "To Hindi: French\nFrom Notepad on 2026-10-13.");
        let braces = parse(&file("name: a\ndescription: d", "Write {a}} like this for {{input}}.")).unwrap();
        assert_eq!(render(&braces.template, &v), "Write {a}} like this for French.", "a lone '}}' first doesn't stop the filling-in");
        assert_eq!(language(None), crate::prompt::QUESTION_LANGUAGE);
        let (line, e) = parse(&file("name: a\ndescription: d", "ok\nUse {{selection}} here")).unwrap_err();
        assert_eq!(line, 6);
        assert!(e.contains("unknown variable {{selection}}"), "{e}");
        assert!(parse(&file("name: a\ndescription: d", "open {{input")).unwrap_err().1.contains("never closed"));
        assert_eq!(today().len(), 10);
    }

    #[test]
    fn skl_1_3_invalid_files_are_listed_with_their_line_and_the_others_still_load() {
        let d = temp_dir("invalid");
        let w = |n: &str, t: &str| std::fs::write(d.join(n), t).unwrap();
        w("good.md", &file("name: good\ndescription: d", "x"));
        w("bad-yaml.md", "---\nname a\n---\nx");
        w("missing.md", &file("name: missing", "x"));
        w("bad-name.md", &file("name: Bad_Name\ndescription: d", "x"));
        w("variable.md", &file("name: var\ndescription: d", "{{nope}}"));
        w("reserved.md", &file("name: help\ndescription: d", "x"));
        w("z-duplicate.md", &file("name: good\ndescription: d", "y"));
        std::fs::write(d.join("big.md"), "x".repeat(70_000)).unwrap();
        w("notes.txt", "not a skill");
        let (skills, invalid) = load(Some(&d));
        assert!(skills.iter().any(|s| s.name == "good" && s.template == "x"));
        let got: Vec<(String, usize)> = invalid.iter().map(|i| (i.file.file_name().unwrap().to_string_lossy().into_owned(), i.line)).collect();
        assert_eq!(got, [("bad-name.md", 2), ("bad-yaml.md", 2), ("big.md", 0), ("missing.md", 3), ("reserved.md", 2), ("variable.md", 5), ("z-duplicate.md", 0)].map(|(f, l)| (f.to_string(), l)));
        assert!(invalid.last().unwrap().error.contains("already used by good.md"));
        assert_eq!(skills.len(), 10, "the nine built-ins and good");
    }

    #[test]
    fn skl_1_4_user_skills_override_built_ins_and_the_first_file_wins() {
        let d = temp_dir("override");
        std::fs::write(d.join("a.md"), file("name: summary\ndescription: mine", "Mine.")).unwrap();
        std::fs::write(d.join("b.md"), file("name: summary\ndescription: other", "Other.")).unwrap();
        let (skills, invalid) = load(Some(&d));
        let s: Vec<&Skill> = skills.iter().filter(|s| s.name == "summary").collect();
        assert_eq!(s.len(), 1);
        assert_eq!((s[0].template.as_str(), s[0].overrides, s[0].file.as_deref()), ("Mine.", true, Some(d.join("a.md").as_path())));
        assert_eq!(invalid.len(), 1);
        assert_eq!(invalid[0].file, d.join("b.md"));
        assert_eq!(load(None).0.len(), 9, "no folder: the built-ins");
    }

    #[test]
    fn skl_2_3_unknown_names_get_the_closest_ones() {
        let (skills, _) = load(None);
        assert_eq!(unknown(&skills, "sumary"), "Unknown skill `/sumary`. Did you mean `/summary`?");
        assert_eq!(closest(["code", "clear", "explain"].into_iter(), "c"), ["code", "clear"]);
        assert!(unknown(&skills, "zzzzzzzz").contains("Type `/help`"));
    }

    #[test]
    fn skl_2_4_a_vision_skill_is_hidden_and_refused_for_a_model_that_cant_see() {
        let d = temp_dir("vision");
        std::fs::write(d.join("look.md"), file("name: look\ndescription: d\nrequires: [vision]", "Describe it.")).unwrap();
        let (skills, _) = load(Some(&d));
        let names = |sees: bool| entries(&skills, |_| sees).into_iter().map(|e| e.name).collect::<Vec<_>>();
        assert!(!names(false).contains(&"look".into()) && names(true).contains(&"look".into()));
        let look = skills.iter().find(|s| s.name == "look").unwrap();
        let model = |vision| ModelInfo { name: "qwen".into(), provider: "Built-in".into(), locality: crate::chat::Locality::Local, context_tokens: None, vision };
        assert_eq!(refusal(look, &model(false)).unwrap(), "`/look` needs a vision model, and qwen can't see images.");
        assert_eq!(refusal(look, &model(true)), None);
    }

    #[test]
    fn skl_2_6_system_command_names_are_invalid_for_skills() {
        for name in ["persona", "new-skill", "snip", "help", "clear"] {
            let e = parse(&file(&format!("name: {name}\ndescription: d"), "x")).unwrap_err().1;
            assert!(e.contains("is a system command"), "{name}: {e}");
        }
    }

    #[test]
    fn skl_2_7_help_lists_the_skills_and_the_working_commands() {
        let help = help(&entries(&load(None).0, |_| false));
        assert!(help.starts_with("**Skills**\n- `/summary` Summarize"), "{help}");
        assert!(help.contains("- `/translate <language>` Translate the selection"));
        assert!(help.contains("**Commands**\n- `/persona <name>`") && help.contains("`/clear`") && help.contains("`/new-skill`") && help.contains("`/help`"));
        assert!(!help.contains("snip"));
    }

    #[test]
    fn skl_3_1_the_built_in_skills_are_valid() {
        let names: Vec<String> = BUILT_IN.iter().map(|t| parse(t).map(|s| s.name).unwrap_or_else(|e| panic!("{e:?} in {t}"))).collect();
        assert_eq!(names, ["summary", "explain", "simplify", "define", "translate", "fix", "rewrite", "code", "bugs"]);
        assert_eq!(load(None), (load(None).0, vec![]));
    }

    #[test]
    fn skl_4_4_a_saved_skill_is_a_file_the_next_load_has() {
        let d = temp_dir("save");
        let f = Fields { name: "eli5".into(), description: "Explain simply".into(), template: "Explain it to a five-year-old.".into(), ..Fields::default() };
        let path = save(&d, &f, None).unwrap();
        assert_eq!(path, d.join("eli5.md"));
        assert!(load(Some(&d)).0.iter().any(|s| s.name == "eli5" && s.file.as_deref() == Some(path.as_path())));
        assert!(save(&d, &f, None).unwrap_err().contains("already uses the name 'eli5'"), "a second new skill can't take the name");
        // Renaming while editing replaces the file.
        let renamed = save(&d, &Fields { name: "simple".into(), ..f.clone() }, Some(&path)).unwrap();
        assert!(!path.exists() && renamed.exists());
        assert!(save(&d, &Fields { name: "".into(), ..f.clone() }, None).is_err());
        assert!(save(&d, &f, Some(Path::new("C:/Windows/win.ini"))).unwrap_err().contains("isn't in the skills folder"));
        // A file named in other letters than <name>.md (dropped in by hand) can be edited; Windows sees one file.
        let hand = d.join("MySkill.md");
        std::fs::write(&hand, file("name: myskill\ndescription: d", "Old.")).unwrap();
        let saved = save(&d, &Fields { name: "myskill".into(), description: "d".into(), template: "New.".into(), ..Fields::default() }, Some(&hand)).unwrap();
        let (skills, invalid) = load(Some(&d));
        assert!(invalid.is_empty(), "{invalid:?}");
        assert_eq!(skills.iter().filter(|s| s.name == "myskill").map(|s| s.template.as_str()).collect::<Vec<_>>(), ["New."]);
        assert!(saved.exists());
    }

    #[test]
    fn skl_4_5_files_changed_outside_cursora_count_at_the_next_load() {
        let d = temp_dir("reload");
        let has = |t: &str| load(Some(&d)).0.iter().any(|s| s.name == "mine" && s.template == t);
        std::fs::write(d.join("mine.md"), file("name: mine\ndescription: d", "One.")).unwrap();
        assert!(has("One."));
        std::fs::write(d.join("mine.md"), file("name: mine\ndescription: d", "Two.")).unwrap();
        assert!(has("Two."));
        std::fs::remove_file(d.join("mine.md")).unwrap();
        assert!(!has("Two.") && !has("One."));
    }
}
