//! Conversations in the bubble and the expanded window (spec 03). Rust owns them; the views only draw them.
use crate::capture::{Capture, Content, Notice, SourceApp};
use crate::history::History;
use crate::{bubble, llm, prompt, AppState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const CHAT_LABEL: &str = "chat";
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
    let personality_on = app.state::<AppState>().settings.lock().unwrap().personality_on;
    let model = llm::active(app);
    let conv = Conversation {
        id: now_ms(),
        app: capture.app,
        content: capture.content,
        notice: capture.notice,
        model: model.as_ref().ok().map(|m| m.info()),
        persona: personality_on.then(|| "Cursora".into()), // shortcut: fixed persona name until spec 05 adds personas
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
    begin(&app, window.label(), |conv| {
        conv.turns.push(Turn { question, answer: String::new(), status: TurnStatus::Streaming, error: None, asked: now_ms(), answered: None })
    })
}

/// Re-runs the latest question; Retry uses it too (BUB-4.4, BUB-6.1).
#[tauri::command]
pub fn regenerate(window: WebviewWindow, app: AppHandle) -> Result<(), String> {
    begin(&app, window.label(), |conv| {
        if let Some(t) = conv.turns.last_mut() {
            *t = Turn { answer: String::new(), status: TurnStatus::Streaming, error: None, asked: now_ms(), answered: None, ..t.clone() };
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
pub fn open_in_chat(app: &AppHandle, conv: Conversation) -> Result<(), String> {
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

/// Marks the turn as streaming via `change`, then answers it on a worker thread.
fn begin(app: &AppHandle, label: &str, change: impl FnOnce(&mut Conversation)) -> Result<(), String> {
    let model = llm::active(app)?;
    let share_title = app.state::<AppState>().settings.lock().unwrap().capture.share_window_title;
    let (id, messages, dropped, cancel, too_long) = {
        let mut slots = slots(app);
        let slot = slots.get_mut(label).ok_or("This conversation is closed.")?;
        if slot.conv.turns.last().is_some_and(|t| t.status == TurnStatus::Streaming) {
            return Err("Wait for the current answer, or stop it.".into());
        }
        change(&mut slot.conv);
        if slot.conv.turns.is_empty() {
            return Ok(());
        }
        let info = model.info();
        // Unknown sizes: no current cloud chat model has less than 32k.
        let assumed = info.context_tokens.or((info.locality == Locality::Cloud).then_some(CLOUD_CONTEXT_TOKENS));
        let (messages, dropped) = assemble(&slot.conv, share_title, budget(assumed));
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
/// Errored turns and empty answers aren't sent back; a cancelled partial answer is.
pub fn assemble(conv: &Conversation, share_title: bool, budget: usize) -> (Vec<Message>, usize) {
    let system = prompt::system();
    let context = prompt::context(&conv.app, &conv.content, share_title);
    let Some((latest, earlier)) = conv.turns.split_last() else { return (vec![], 0) };
    let pairs: Vec<&Turn> = earlier.iter().filter(|t| t.status != TurnStatus::Error && !t.answer.is_empty()).collect();
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

/// Debug builds with CURSORA_ECHO set: streams a canned Markdown answer, so the bubble can be tried without a model.
/// "error: <kind>" (e.g. "error: auth") answers with that error.
#[cfg(debug_assertions)]
pub struct Echo;

#[cfg(debug_assertions)]
impl Model for Echo {
    fn info(&self) -> ModelInfo {
        ModelInfo { name: "echo".into(), provider: "Debug".into(), locality: Locality::Local, context_tokens: Some(DEFAULT_CONTEXT_TOKENS) }
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
        Turn { question: q.into(), answer: a.into(), status, error: None, asked: 0, answered: None }
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
        messages.iter().map(|m| m.text.matches("<context source=").count()).sum()
    }

    #[test]
    fn bub_5_1_follow_ups_send_the_conversation_with_the_context_once() {
        let c = conv(vec![turn("q1", "a1", TurnStatus::Done), turn("q2", "a2", TurnStatus::Done), turn("q3", "", TurnStatus::Streaming)]);
        let (m, dropped) = assemble(&c, false, budget(None));
        assert_eq!(dropped, 0);
        let roles: Vec<Role> = m.iter().map(|m| m.role).collect();
        assert_eq!(roles, [Role::System, Role::User, Role::Assistant, Role::User, Role::Assistant, Role::User]);
        assert_eq!(context_count(&m), 1);
        assert!(m[1].text.contains("SELECTED") && m[1].text.ends_with("q1"), "the context rides on the oldest question");
        assert_eq!(m[5].text, "q3");
        // Errored turns aren't sent back; a cancelled partial answer is.
        let c = conv(vec![turn("bad", "", TurnStatus::Error), turn("half", "partial", TurnStatus::Cancelled), turn("q", "", TurnStatus::Streaming)]);
        let (m, _) = assemble(&c, false, budget(None));
        assert_eq!(m.iter().map(|m| m.text.lines().last().unwrap()).collect::<Vec<_>>()[1..], ["half", "partial", "q"]);
        // No context: nothing is wrapped.
        let mut c = conv(vec![turn("q", "", TurnStatus::Streaming)]);
        c.content = Content::None;
        assert_eq!(context_count(&assemble(&c, false, budget(None)).0), 0);
    }

    #[test]
    fn bub_5_2_oldest_pairs_are_dropped_to_fit_the_budget() {
        let long = "x".repeat(400); // 100 tokens
        let c = conv(vec![
            turn("old question", &long, TurnStatus::Done),
            turn("middle question", &long, TurnStatus::Done),
            turn("latest", "", TurnStatus::Streaming),
        ]);
        let fixed = tokens(&prompt::system()) + tokens(&prompt::context(&c.app, &c.content, false).unwrap()) + tokens("latest");
        let (m, dropped) = assemble(&c, false, fixed + 110); // room for one pair only
        assert_eq!(dropped, 1);
        assert!(m.iter().all(|m| !m.text.contains("old question")));
        assert!(m[1].text.contains("SELECTED") && m[1].text.ends_with("middle question"), "context moves to the oldest kept question");
        assert_eq!((m[0].role, m.last().unwrap().text.as_str()), (Role::System, "latest"));
        // Even a budget too small for the essentials keeps them.
        let (m, dropped) = assemble(&c, false, 1);
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
            ModelInfo { name: "fake".into(), provider: "Test".into(), locality: Locality::Local, context_tokens: None }
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
