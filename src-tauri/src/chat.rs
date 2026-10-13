//! Conversations in the bubble and the expanded window (spec 03). Rust owns them; the views only draw them.
use crate::capture::{Capture, Content, Notice, SourceApp};
use crate::history::History;
use crate::persona::{self, Persona};
use crate::{bubble, llm, prompt, skills, AppState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const CHAT_LABEL: &str = "chat";
const CLOSED: &str = "This conversation is closed.";
/// The app of the sample conversation in previews and the skill form's Test.
pub const SAMPLE_APP: &str = "Notepad";
/// Used when the model doesn't report its context size (BUB-5.2).
const DEFAULT_CONTEXT_TOKENS: u32 = 8192;
const CLOUD_CONTEXT_TOKENS: u32 = 32_768;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Locality {
    Local,
    Lan,
    Cloud,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub name: String,
    pub provider: String,
    pub locality: Locality,
    pub context_tokens: Option<u32>,
    /// Whether it can see images (SKL-2.4; spec 10's snip).
    #[serde(default)]
    pub vision: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub role: Role,
    pub text: String,
    /// Sent only to vision models; spec 10 fills it (LLM-1.1).
    pub images: Vec<Image>,
}

impl Message {
    pub fn new(role: Role, text: impl Into<String>) -> Self {
        Self { role, text: text.into(), images: vec![] }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub mime: String,
    pub base64: String,
}

/// The LLM-1.4 failure kinds.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    Auth,
    RateLimit,
    ModelNotFound,
    ContextTooLong,
    Network,
    Server,
    BlockedLocalOnly,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChatError {
    pub kind: ErrorKind,
    pub message: String,
}

/// The seam spec 04 fills with real providers.
pub trait Model: Send + Sync {
    fn info(&self) -> ModelInfo;
    /// Calls `on_delta` for each piece of text as it arrives; returns when done, failed or cancelled.
    fn stream(&self, messages: &[Message], cancel: &AtomicBool, on_delta: &mut dyn FnMut(&str)) -> Result<(), ChatError>;
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TurnStatus {
    Streaming,
    Done,
    Cancelled,
    Error,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    pub question: String,
    pub answer: String,
    pub status: TurnStatus,
    pub error: Option<ChatError>,
    /// Unix ms.
    pub asked: i64,
    pub answered: Option<i64>,
    /// Cursora's own reply to a command such as `/persona`: shown and saved, never sent to the model (PER-3.1).
    #[serde(default)]
    pub local: bool,
    /// Said about the answer, e.g. that a skill's own model wasn't available (SKL-2.5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    /// Unix ms when it was opened; also its history key.
    pub id: i64,
    pub app: SourceApp,
    pub content: Content,
    pub notice: Option<Notice>,
    pub model: Option<ModelInfo>,
    pub persona: Option<String>,
    pub turns: Vec<Turn>,
    /// Earlier question/answer pairs left out of the last request (BUB-5.2).
    pub dropped: usize,
    /// Why no model can answer; the bubble then shows "No AI model is available" (LLM-9.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<String>,
}

/// One conversation per window label: the bubble's and the expanded window's.
#[derive(Default)]
pub struct ChatState(Mutex<HashMap<String, Slot>>);

fn slots(app: &AppHandle) -> MutexGuard<'_, HashMap<String, Slot>> {
    app.state::<ChatState>().inner().0.lock().unwrap()
}

struct Slot {
    conv: Conversation,
    cancel: Arc<AtomicBool>,
}

#[derive(Serialize, Clone)]
struct Delta<'a> {
    id: i64,
    /// Where `text` starts in the answer, in UTF-16 units like JS strings. A view that is behind reloads instead.
    offset: usize,
    text: &'a str,
}

#[derive(Serialize, Clone)]
struct Done {
    id: i64,
    status: TurnStatus,
    error: Option<ChatError>,
    dropped: usize,
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

/// Starts the bubble's conversation for a new capture (BUB-5.3). Any earlier one is already in history.
pub fn start(app: &AppHandle, capture: Capture) -> Conversation {
    let persona = persona::in_use(&app.state::<AppState>().settings.lock().unwrap()).map(|p| p.name);
    let model = llm::active(app);
    let conv = Conversation {
        id: now_ms(),
        app: capture.app,
        content: capture.content,
        notice: capture.notice,
        model: model.as_ref().ok().map(|m| m.info()),
        persona,
        turns: vec![],
        dropped: 0,
        unavailable: model.err(),
    };
    put(app, bubble::LABEL, conv.clone());
    conv
}

/// The model switcher picked another model (LLM-5.5): the window's conversation answers with it from the next question.
pub fn switch_model(app: &AppHandle, label: &str, info: Result<ModelInfo, String>) -> Option<Conversation> {
    let mut slots = slots(app);
    let conv = &mut slots.get_mut(label)?.conv;
    (conv.model, conv.unavailable) = match info {
        Ok(i) => (Some(i), None),
        Err(reason) => (None, Some(reason)),
    };
    Some(conv.clone())
}

/// The health check failed: the bubble's conversation shows why instead of its input (LLM-9.1).
pub fn set_unavailable(app: &AppHandle, id: i64, reason: String) -> Option<Conversation> {
    with_conv(app, id, |c| {
        c.unavailable = Some(reason);
        c.clone()
    })
}

fn put(app: &AppHandle, label: &str, conv: Conversation) {
    let old = slots(app).insert(label.into(), Slot { conv, cancel: Arc::default() });
    retire(app, old);
}

/// A conversation leaving its window. If it was mid-answer, its worker can no longer find it,
/// so the turn is marked cancelled and saved here (BUB-2.6).
fn retire(app: &AppHandle, slot: Option<Slot>) {
    let Some(mut slot) = slot else { return };
    slot.cancel.store(true, Ordering::SeqCst);
    if mark_cancelled(&mut slot.conv) {
        save(app, &slot.conv);
    }
}

/// Marks a still-streaming last turn as cancelled, keeping its partial answer. False if nothing was streaming.
pub fn mark_cancelled(conv: &mut Conversation) -> bool {
    match conv.turns.last_mut() {
        Some(t) if t.status == TurnStatus::Streaming => {
            (t.status, t.answered) = (TurnStatus::Cancelled, Some(now_ms()));
            true
        }
        _ => false,
    }
}

fn save(app: &AppHandle, conv: &Conversation) {
    let cfg = app.state::<AppState>().settings.lock().unwrap().history.clone();
    if let Err(e) = app.state::<History>().record(&cfg, conv, now_ms()) {
        log::warn!("history: {e}");
    }
}

/// Cancels the window's running answer, if any (BUB-2.6, BUB-4.3).
pub fn cancel(app: &AppHandle, label: &str) {
    if let Some(slot) = slots(app).get(label) {
        slot.cancel.store(true, Ordering::SeqCst);
    }
}

/// The expanded window was closed: stop its answer and forget it (it is already in history).
pub fn drop_slot(app: &AppHandle, label: &str) {
    let old = slots(app).remove(label);
    retire(app, old);
}

#[tauri::command]
pub fn get_conversation(window: WebviewWindow, app: AppHandle) -> Option<Conversation> {
    slots(&app).get(window.label()).map(|s| s.conv.clone())
}

#[tauri::command]
pub fn ask(window: WebviewWindow, app: AppHandle, question: String) -> Result<(), String> {
    let question = question.trim().to_string();
    if question.is_empty() {
        return Ok(());
    }
    let label = window.label();
    let Some((name, rest)) = command(&question) else { return answer(&app, label, question) };
    let rest = rest.to_string();
    match name.as_str() {
        skills::PERSONA => persona_command(&app, label, question, &rest),
        skills::HELP => local(&app, label, question, skills::help(&skills::available(&app))),
        skills::CLEAR => clear(&app, label),
        skills::NEW_SKILL => new_skill(&app, label),
        skills::SNIP => local(&app, label, question, "Snipping isn't available yet.".into()), // spec 10
        _ => match skills::find(&app, &name) {
            None => local(&app, label, question, skills::unknown(&skills::load(skills::dir(&app).ok().as_deref()).0, &name)),
            Some(s) => {
                let (model, _) = llm::for_skill(&app, s.model.as_deref(), s.temperature)?;
                match skills::refusal(&s, &model.info()) {
                    Some(reason) => local(&app, label, question, reason),
                    None => answer(&app, label, question),
                }
            }
        },
    }
}

/// Sends a question to the model as a new turn.
fn answer(app: &AppHandle, label: &str, question: String) -> Result<(), String> {
    let q = question.clone();
    begin(app, label, &q, move |conv| {
        conv.turns.push(Turn { question, answer: String::new(), status: TurnStatus::Streaming, error: None, asked: now_ms(), answered: None, local: false, note: None })
    })
}

/// `/name rest`: the name in lower case and the trimmed rest, if the question is a command (SKL-2.2).
/// A name is letters, digits and hyphens, so a question such as "/etc/hosts?" isn't one.
pub fn command(question: &str) -> Option<(String, &str)> {
    let rest = question.strip_prefix('/')?;
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let name = &rest[..end];
    (!name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')).then(|| (name.to_ascii_lowercase(), rest[end..].trim()))
}

fn idle(conv: &Conversation) -> Result<(), String> {
    match conv.turns.last() {
        Some(t) if t.status == TurnStatus::Streaming => Err("Wait for the current answer, or stop it.".into()),
        _ => Ok(()),
    }
}

/// Cursora's own reply: shown and saved, never sent to a model (PER-3.1, SKL-2.3).
fn local(app: &AppHandle, label: &str, question: String, answer: String) -> Result<(), String> {
    let now = now_ms();
    let conv = {
        let mut slots = slots(app);
        let conv = &mut slots.get_mut(label).ok_or(CLOSED)?.conv;
        idle(conv)?;
        conv.turns.push(Turn { question, answer, status: TurnStatus::Done, error: None, asked: now, answered: Some(now), local: true, note: None });
        conv.clone()
    };
    save(app, &conv);
    let _ = app.emit_to(label, "chat-load", ());
    Ok(())
}

/// `/persona <name>` (PER-3.1). Switching also turns personality on.
fn persona_command(app: &AppHandle, label: &str, question: String, query: &str) -> Result<(), String> {
    idle(&slots(app).get(label).ok_or(CLOSED)?.conv)?;
    let mut s = app.state::<AppState>().settings.lock().unwrap().clone();
    let all = persona::all(&s);
    let (chosen, answer) = persona::command(&all, &persona::active(&s).id, query);
    if let Some(p) = chosen {
        (s.active_persona, s.personality_on) = (p.id.clone(), true);
        crate::update_settings(app, s)?;
    }
    local(app, label, question, answer)
}

/// `/clear` (SKL-2.7). The old conversation is already in history.
fn clear(app: &AppHandle, label: &str) -> Result<(), String> {
    {
        let mut slots = slots(app);
        let slot = slots.get_mut(label).ok_or(CLOSED)?;
        idle(&slot.conv)?;
        slot.conv = cleared(&slot.conv, now_ms());
    }
    let _ = app.emit_to(label, "chat-load", ());
    Ok(())
}

/// A new conversation with the same capture, model and persona.
pub fn cleared(c: &Conversation, id: i64) -> Conversation {
    Conversation { id, turns: vec![], dropped: 0, ..c.clone() }
}

/// `/new-skill` (SKL-4.2): the skill form, with this conversation's previous question as the template.
fn new_skill(app: &AppHandle, label: &str) -> Result<(), String> {
    let previous = slots(app).get(label).and_then(|s| s.conv.turns.iter().rev().find(|t| !t.local && command(&t.question).is_none()).map(|t| t.question.clone()));
    skills::draft(app, previous.unwrap_or_default());
    let _ = app.emit_to(label, "chat-load", ()); // drops the view's placeholder turn
    Ok(())
}

/// The persona or the personality switch changed: open conversations show it now and use it from their next message (PER-2.2).
pub fn persona_changed(app: &AppHandle, name: Option<String>) {
    for slot in slots(app).values_mut() {
        slot.conv.persona = name.clone();
    }
    let _ = app.emit("chat-persona", name);
}

/// Re-runs the latest question, a skill too; Retry uses it as well (BUB-4.4, BUB-6.1). A local reply stays as it is.
#[tauri::command]
pub fn regenerate(window: WebviewWindow, app: AppHandle) -> Result<(), String> {
    let label = window.label();
    let Some(question) = slots(&app).get(label).and_then(|s| s.conv.turns.last()).filter(|t| !t.local).map(|t| t.question.clone()) else { return Ok(()) };
    begin(&app, label, &question, |conv| {
        if let Some(t) = conv.turns.last_mut() {
            *t = Turn { answer: String::new(), status: TurnStatus::Streaming, error: None, asked: now_ms(), answered: None, note: None, ..t.clone() };
        }
    })
}

#[tauri::command]
pub fn stop(window: WebviewWindow, app: AppHandle) {
    cancel(&app, window.label());
}

/// BUB-3.2: only before the first question, since later answers depend on the context.
#[tauri::command]
pub fn remove_context(window: WebviewWindow, app: AppHandle) -> Option<Conversation> {
    let mut slots = slots(&app);
    let slot = slots.get_mut(window.label())?;
    if slot.conv.turns.is_empty() {
        slot.conv.content = Content::None;
        slot.conv.notice = None;
    }
    Some(slot.conv.clone())
}

/// Moves the bubble's conversation into the resizable window, without cancelling a running answer (BUB-2.5).
/// Async: creating a window inside a synchronous command deadlocks on Windows (wry#583).
#[tauri::command(async)]
pub fn expand(app: AppHandle) -> Result<(), String> {
    let old = {
        let mut slots = slots(&app); // one lock, so no streamed text lands between remove and insert
        let Some(slot) = slots.remove(bubble::LABEL) else { return Ok(()) };
        slots.insert(CHAT_LABEL.into(), slot)
    };
    retire(&app, old);
    bubble::hide(&app); // not close(): focus goes to the new window, not back to the app
    show_chat_window(&app)
}

/// Opens a saved conversation in the expanded window, where it can be continued (BUB-9.2).
/// The header shows today's persona, which answers from the next message.
pub fn open_in_chat(app: &AppHandle, mut conv: Conversation) -> Result<(), String> {
    conv.persona = persona::in_use(&app.state::<AppState>().settings.lock().unwrap()).map(|p| p.name);
    put(app, CHAT_LABEL, conv);
    show_chat_window(app)
}

fn show_chat_window(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(CHAT_LABEL) {
        let _ = w.emit_to(CHAT_LABEL, "chat-load", ());
        let _ = w.set_focus(); // a normal window the user is already working in, so no foreground-lock fallback is needed
        return Ok(());
    }
    WebviewWindowBuilder::new(app, CHAT_LABEL, WebviewUrl::default())
        .additional_browser_args(crate::BROWSER_ARGS)
        .title("Cursora")
        .inner_size(720.0, 640.0)
        .min_inner_size(380.0, 300.0)
        .on_navigation(|url| bubble::is_app_url(url))
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Marks the turn as streaming via `change`, then answers `question` on a worker thread.
/// A question that names a skill runs it, with the skill's model if it has one (SKL-2.2, 2.5).
fn begin(app: &AppHandle, label: &str, question: &str, change: impl FnOnce(&mut Conversation)) -> Result<(), String> {
    let skill = command(question).and_then(|(name, input)| skills::find(app, &name).map(|s| (s, input.to_string())));
    let (model, note) = match &skill {
        Some((s, _)) => llm::for_skill(app, s.model.as_deref(), s.temperature)?,
        None => (llm::active(app)?, None),
    };
    if let Some(reason) = skill.as_ref().and_then(|(s, _)| skills::refusal(s, &model.info())) {
        return Err(reason);
    }
    let s = app.state::<AppState>().settings.lock().unwrap().clone();
    let (share_title, persona) = (s.capture.share_window_title, persona::in_use(&s));
    let language = skills::language(persona.as_ref());
    let (id, messages, dropped, cancel, too_long) = {
        let mut slots = slots(app);
        let slot = slots.get_mut(label).ok_or(CLOSED)?;
        idle(&slot.conv)?;
        change(&mut slot.conv);
        let Some(last) = slot.conv.turns.last_mut().filter(|t| t.status == TurnStatus::Streaming) else { return Ok(()) }; // nothing to answer
        last.note = note;
        let date = skills::today();
        let instructions = skill.map(|(s, input)| skills::render(&s.template, &skills::Vars { input: &input, app: &slot.conv.app.name, language: &language, date: &date }));
        let info = model.info();
        // Unknown sizes: no current cloud chat model has less than 32k.
        let assumed = info.context_tokens.or((info.locality == Locality::Cloud).then_some(CLOUD_CONTEXT_TOKENS));
        let (messages, dropped) = assemble(&slot.conv, persona.as_ref(), instructions.as_deref(), share_title, budget(assumed));
        let too_long = info.context_tokens.and_then(|ctx| too_long(&messages, ctx));
        (slot.conv.model, slot.conv.dropped) = (Some(info), dropped); // a reopened conversation answers with today's model
        slot.cancel = Arc::default();
        (slot.conv.id, messages, dropped, slot.cancel.clone(), too_long)
    };
    let app = app.clone();
    std::thread::spawn(move || {
        let mut offset = 0;
        let (status, error) = match too_long {
            Some(e) => (TurnStatus::Error, Some(e)),
            None => run_turn(&*model, &messages, &cancel, &mut |text| {
                with_conv(&app, id, |c| c.turns.last_mut().unwrap().answer.push_str(text));
                for label in [bubble::LABEL, CHAT_LABEL] {
                    let _ = app.emit_to(label, "chat-delta", Delta { id, offset, text });
                }
                offset += text.encode_utf16().count();
            }),
        };
        let conv = with_conv(&app, id, |c| {
            let t = c.turns.last_mut().unwrap();
            (t.status, t.error, t.answered) = (status, error.clone(), Some(now_ms()));
            c.clone()
        });
        for label in [bubble::LABEL, CHAT_LABEL] {
            let _ = app.emit_to(label, "chat-done", Done { id, status, error: error.clone(), dropped });
        }
        if let Some(conv) = conv {
            save(&app, &conv);
        }
    });
    Ok(())
}

/// Finds the conversation wherever it is now; Expand may have moved it while it streamed.
fn with_conv<T>(app: &AppHandle, id: i64, f: impl FnOnce(&mut Conversation) -> T) -> Option<T> {
    let mut slots = slots(app);
    slots.values_mut().find(|s| s.conv.id == id).map(|s| f(&mut s.conv))
}

/// Streams one answer. Once cancelled, no more deltas are passed on, even if the model keeps going (BUB-2.6, LLM-1.3).
pub fn run_turn(model: &dyn Model, messages: &[Message], cancel: &AtomicBool, on_delta: &mut dyn FnMut(&str)) -> (TurnStatus, Option<ChatError>) {
    let result = model.stream(messages, cancel, &mut |text| {
        if !cancel.load(Ordering::SeqCst) {
            on_delta(text);
        }
    });
    match (cancel.load(Ordering::SeqCst), result) {
        (true, _) => (TurnStatus::Cancelled, None),
        (false, Ok(())) => (TurnStatus::Done, None),
        (false, Err(e)) => (TurnStatus::Error, Some(e)),
    }
}

/// Tokens available for the request: the context window minus room for the answer.
pub fn budget(context_tokens: Option<u32>) -> usize {
    let ctx = context_tokens.unwrap_or(DEFAULT_CONTEXT_TOKENS) as usize;
    ctx - (ctx / 4).min(1024)
}

// shortcut: chars/4 token estimate; use the provider's token counts if trimming is visibly off.
fn tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

/// A request that can't fit a known context fails before it is sent: some local servers would cut it
/// silently, dropping the start of the selection or the system prompt's rules (spec 04 design).
pub fn too_long(messages: &[Message], context_tokens: u32) -> Option<ChatError> {
    let need: usize = messages.iter().map(|m| tokens(&m.text)).sum();
    (need > budget(Some(context_tokens))).then(|| ChatError {
        kind: ErrorKind::ContextTooLong,
        message: format!("This is about {need} tokens, more than fits in the model's {context_tokens}-token context with room for an answer. Select less, or raise the context size in Models settings."),
    })
}

/// The request for the latest question (BUB-5.1, 5.2):
/// system prompt, then the context on the oldest kept question (so exactly once), then the turns.
/// Oldest pairs go first when over budget; the system prompt, context and latest question always stay.
/// Errored turns, empty answers and local replies aren't sent back; a cancelled partial answer is.
/// `persona` is None while personality is off (PER-2.1); `skill` is the rendered instructions of a skill, if one runs.
pub fn assemble(conv: &Conversation, persona: Option<&Persona>, skill: Option<&str>, share_title: bool, budget: usize) -> (Vec<Message>, usize) {
    let system = prompt::system(persona, skill);
    let context = prompt::context(&conv.app, &conv.content, share_title);
    let Some((latest, earlier)) = conv.turns.split_last() else { return (vec![], 0) };
    let pairs: Vec<&Turn> = earlier.iter().filter(|t| t.status != TurnStatus::Error && !t.answer.is_empty() && !t.local).collect();
    let pair_tokens = |t: &Turn| tokens(&t.question) + tokens(&t.answer);
    let mut total = tokens(&system) + context.as_deref().map_or(0, tokens) + tokens(&latest.question) + pairs.iter().map(|t| pair_tokens(t)).sum::<usize>();
    let mut start = 0;
    while start < pairs.len() && total > budget {
        total -= pair_tokens(pairs[start]);
        start += 1;
    }
    let mut context = context;
    let mut user = |q: &str| Message::new(Role::User, context.take().map_or_else(|| q.to_string(), |c| format!("{c}\n\n{q}")));
    let mut messages = vec![Message::new(Role::System, system)];
    for t in &pairs[start..] {
        messages.push(user(&t.question));
        messages.push(Message::new(Role::Assistant, t.answer.clone()));
    }
    messages.push(user(&latest.question));
    (messages, start)
}

/// The exact messages for a sample selection and question with the current settings (PER-4.4).
#[tauri::command]
pub fn prompt_preview(app: AppHandle) -> String {
    let s = app.state::<AppState>().settings.lock().unwrap().clone();
    let conv = sample("Step 1: write the draft.\nStep 2: send it to Priya by Friday.", "What is due on Friday?");
    prompt::render(&assemble(&conv, persona::in_use(&s).as_ref(), None, s.capture.share_window_title, usize::MAX).0)
}

/// A conversation about `text` selected in Notepad, waiting for the answer to `question`: for previews and tests.
pub fn sample(text: &str, question: &str) -> Conversation {
    Conversation {
        id: 0,
        app: SourceApp { name: SAMPLE_APP.into(), title: format!("plan.txt - {SAMPLE_APP}"), ..Default::default() },
        content: Content::Text { text: text.into(), truncated: false },
        notice: None,
        model: None,
        persona: None,
        turns: vec![Turn { question: question.into(), answer: String::new(), status: TurnStatus::Streaming, error: None, asked: 0, answered: None, local: false, note: None }],
        dropped: 0,
        unavailable: None,
    }
}

/// Debug builds with CURSORA_ECHO set: streams a canned Markdown answer, so the bubble can be tried without a model.
/// "error: <kind>" (e.g. "error: auth") answers with that error.
#[cfg(debug_assertions)]
pub struct Echo;

#[cfg(debug_assertions)]
impl Model for Echo {
    fn info(&self) -> ModelInfo {
        ModelInfo { name: "echo".into(), provider: "Debug".into(), locality: Locality::Local, context_tokens: Some(DEFAULT_CONTEXT_TOKENS), vision: false }
    }

    fn stream(&self, messages: &[Message], cancel: &AtomicBool, on_delta: &mut dyn FnMut(&str)) -> Result<(), ChatError> {
        let question = messages.last().and_then(|m| m.text.lines().last()).unwrap_or_default();
        if let Some(kind) = question.strip_prefix("error:") {
            let kind = serde_json::from_value(serde_json::Value::String(kind.trim().into())).unwrap_or(ErrorKind::Unknown);
            return Err(ChatError { kind, message: "Echo test error".into() });
        }
        let answer = format!(
            "You asked: **{question}**\n\nThe request had {} messages ({} characters).\n\n```rust\nfn main() {{\n    println!(\"hello\");\n}}\n```\n\n- one\n- two\n",
            messages.len(),
            messages.iter().map(|m| m.text.chars().count()).sum::<usize>()
        );
        std::thread::sleep(std::time::Duration::from_millis(400)); // shows "thinking"
        for word in answer.split_inclusive(' ') {
            if cancel.load(Ordering::SeqCst) {
                break;
            }
            on_delta(word);
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(q: &str, a: &str, status: TurnStatus) -> Turn {
        Turn { question: q.into(), answer: a.into(), status, error: None, asked: 0, answered: None, local: false, note: None }
    }

    fn conv(turns: Vec<Turn>) -> Conversation {
        Conversation {
            id: 1,
            app: SourceApp { name: "notepad".into(), ..Default::default() },
            content: Content::Text { text: "SELECTED".into(), truncated: false },
            notice: None,
            model: None,
            persona: None,
            turns,
            dropped: 0,
            unavailable: None,
        }
    }

    fn context_count(messages: &[Message]) -> usize {
        messages.iter().map(|m| m.text.matches("<selected_text source=").count()).sum()
    }

    #[test]
    fn bub_5_1_follow_ups_send_the_conversation_with_the_context_once() {
        let c = conv(vec![turn("q1", "a1", TurnStatus::Done), turn("q2", "a2", TurnStatus::Done), turn("q3", "", TurnStatus::Streaming)]);
        let (m, dropped) = assemble(&c, None, None, false, budget(None));
        assert_eq!(dropped, 0);
        let roles: Vec<Role> = m.iter().map(|m| m.role).collect();
        assert_eq!(roles, [Role::System, Role::User, Role::Assistant, Role::User, Role::Assistant, Role::User]);
        assert_eq!(context_count(&m), 1);
        assert!(m[1].text.contains("SELECTED") && m[1].text.ends_with("q1"), "the context rides on the oldest question");
        assert_eq!(m[5].text, "q3");
        // Errored turns aren't sent back; a cancelled partial answer is.
        let c = conv(vec![turn("bad", "", TurnStatus::Error), turn("half", "partial", TurnStatus::Cancelled), turn("q", "", TurnStatus::Streaming)]);
        let (m, _) = assemble(&c, None, None, false, budget(None));
        assert_eq!(m.iter().map(|m| m.text.lines().last().unwrap()).collect::<Vec<_>>()[1..], ["half", "partial", "q"]);
        // No context: nothing is wrapped.
        let mut c = conv(vec![turn("q", "", TurnStatus::Streaming)]);
        c.content = Content::None;
        assert_eq!(context_count(&assemble(&c, None, None, false, budget(None)).0), 0);
    }

    #[test]
    fn per_3_1_persona_replies_are_never_sent_to_the_model() {
        let local = Turn { local: true, ..turn("/persona ada", "Now speaking as **Ada**.", TurnStatus::Done) };
        let c = conv(vec![local, turn("q1", "a1", TurnStatus::Done), turn("q2", "", TurnStatus::Streaming)]);
        let (m, _) = assemble(&c, None, None, false, budget(None));
        assert_eq!(m.len(), 4);
        assert!(m.iter().all(|m| !m.text.contains("/persona") && !m.text.contains("Ada")));
        assert!(m[1].text.contains("SELECTED") && m[1].text.ends_with("q1"), "the context moves to the first real question");
    }

    #[test]
    fn skl_2_2_a_command_is_its_name_in_any_case_and_the_trimmed_rest() {
        assert_eq!(command("/translate  French "), Some(("translate".into(), "French")));
        assert_eq!(command("/Persona Ada"), Some(("persona".into(), "Ada")), "issue 23");
        assert_eq!(command("/help"), Some(("help".into(), "")));
        assert_eq!(command("/new-skill\nnow"), Some(("new-skill".into(), "now")));
        for not_one in ["/etc/hosts?", "/ summary", "/", "what does /persona do", "summary"] {
            assert_eq!(command(not_one), None, "{not_one}");
        }
    }

    #[test]
    fn skl_2_7_clear_keeps_the_capture_and_starts_with_no_turns() {
        let mut c = conv(vec![turn("q1", "a1", TurnStatus::Done)]);
        c.dropped = 1;
        let fresh = cleared(&c, 99);
        assert_eq!((fresh.id, fresh.turns.len(), fresh.dropped), (99, 0, 0));
        assert_eq!((&fresh.app, &fresh.content, &fresh.model, &fresh.persona), (&c.app, &c.content, &c.model, &c.persona));
    }

    #[test]
    fn bub_5_2_oldest_pairs_are_dropped_to_fit_the_budget() {
        let long = "x".repeat(400); // 100 tokens
        let c = conv(vec![
            turn("old question", &long, TurnStatus::Done),
            turn("middle question", &long, TurnStatus::Done),
            turn("latest", "", TurnStatus::Streaming),
        ]);
        let fixed = tokens(&prompt::system(None, None)) + tokens(&prompt::context(&c.app, &c.content, false).unwrap()) + tokens("latest");
        let (m, dropped) = assemble(&c, None, None, false, fixed + 110); // room for one pair only
        assert_eq!(dropped, 1);
        assert!(m.iter().all(|m| !m.text.contains("old question")));
        assert!(m[1].text.contains("SELECTED") && m[1].text.ends_with("middle question"), "context moves to the oldest kept question");
        assert_eq!((m[0].role, m.last().unwrap().text.as_str()), (Role::System, "latest"));
        // Even a budget too small for the essentials keeps them.
        let (m, dropped) = assemble(&c, None, None, false, 1);
        assert_eq!(dropped, 2);
        assert_eq!(m.len(), 2);
        assert_eq!(context_count(&m), 1);
    }

    #[test]
    fn bub_5_2_a_request_that_cant_fit_a_known_context_fails_before_sending() {
        let fits = vec![Message::new(Role::System, "x".repeat(400)), Message::new(Role::User, "x".repeat(4000))]; // ~1100 tokens
        assert_eq!(too_long(&fits, 8192), None);
        let e = too_long(&fits, 1024).unwrap(); // 1024 - 256 for the answer
        assert_eq!(e.kind, ErrorKind::ContextTooLong);
        assert!(e.message.contains("1100 tokens") && e.message.contains("1024-token"), "{}", e.message);
    }

    /// Streams `parts`; the test's on_delta may cancel along the way.
    struct Scripted(Vec<&'static str>, Option<ChatError>);

    impl Model for Scripted {
        fn info(&self) -> ModelInfo {
            ModelInfo { name: "fake".into(), provider: "Test".into(), locality: Locality::Local, context_tokens: None, vision: false }
        }
        fn stream(&self, _: &[Message], _: &AtomicBool, on_delta: &mut dyn FnMut(&str)) -> Result<(), ChatError> {
            for p in &self.0 {
                on_delta(p); // a model that ignores cancel, the worst case
            }
            self.1.clone().map_or(Ok(()), Err)
        }
    }

    #[test]
    fn bub_4_1_deltas_are_passed_on_in_order_as_they_arrive() {
        let mut got = vec![];
        let status = run_turn(&Scripted(vec!["Hel", "lo", " world"], None), &[], &AtomicBool::new(false), &mut |t| got.push(t.to_string()));
        assert_eq!(got, ["Hel", "lo", " world"]);
        assert_eq!(status, (TurnStatus::Done, None));
        let err = ChatError { kind: ErrorKind::Auth, message: "401".into() };
        assert_eq!(run_turn(&Scripted(vec![], Some(err.clone())), &[], &AtomicBool::new(false), &mut |_| {}), (TurnStatus::Error, Some(err)));
    }

    #[test]
    fn bub_2_6_a_conversation_leaving_its_window_mid_answer_is_marked_cancelled() {
        let mut c = conv(vec![turn("q1", "a1", TurnStatus::Done), turn("q2", "partial", TurnStatus::Streaming)]);
        assert!(mark_cancelled(&mut c));
        let last = c.turns.last().unwrap();
        assert_eq!((last.status, last.answer.as_str()), (TurnStatus::Cancelled, "partial"));
        assert!(!mark_cancelled(&mut c), "a finished turn is left alone, so it is saved once");
    }

    #[test]
    fn bub_2_6_closing_mid_answer_cancels_and_keeps_the_partial_answer() {
        let cancel = AtomicBool::new(false);
        let mut answer = String::new();
        let status = run_turn(&Scripted(vec!["one ", "two ", "three"], None), &[], &cancel, &mut |t| {
            answer.push_str(t);
            if answer == "one two " {
                cancel.store(true, Ordering::SeqCst); // the bubble closes here
            }
        });
        assert_eq!(answer, "one two ", "nothing after the cancel");
        assert_eq!(status, (TurnStatus::Cancelled, None));
    }
}
