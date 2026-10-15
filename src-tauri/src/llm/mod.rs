//! AI providers (spec 04): configuration, local vs cloud, and which model answers.
mod anthropic;
pub mod commands;
pub mod engine;
mod gemini;
mod local;
pub mod net;
mod openai;
mod vision;

use crate::chat::{ChatError, ErrorKind, Locality, Message, Model, ModelInfo};
use crate::{secrets, AppState};
use net::{Flow, Policy};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use url::{Host, Url};

/// Debug builds talk to the Echo test model when CURSORA_ECHO is set, so the bubble can be tried without a model.
pub fn echo() -> bool {
    cfg!(debug_assertions) && std::env::var_os("CURSORA_ECHO").is_some()
}

/// The configured default model, ready to answer, or why there is none (LLM-9.1).
pub fn active(app: &AppHandle) -> Result<Arc<dyn Model>, String> {
    for_skill(app, None, None).map(|(m, _)| m)
}

/// A skill's own model when it names one that is available, else the active model and a note saying so (SKL-2.5).
pub fn for_skill(app: &AppHandle, reference: Option<&str>, temperature: Option<f64>) -> Result<(Arc<dyn Model>, Option<String>), String> {
    #[cfg(debug_assertions)]
    if echo() {
        return Ok((Arc::new(crate::chat::Echo), None));
    }
    let s = app.state::<AppState>().settings.lock().unwrap().clone();
    let (p, model, note) = match reference.map(|r| (r, pick(&s.models, s.local_only, r))) {
        Some((_, Ok((p, m)))) => (p, m, None),
        other => {
            let (p, m) = availability(&s.models, s.local_only)?;
            (p, m, other.and_then(|(r, why)| why.err().map(|why| format!("{r} isn't available ({why}), so {m} answered."))))
        }
    };
    Ok((Arc::new(ProviderModel { app: app.clone(), provider: p.clone(), model: model.into(), policy: Policy::of(&s), temperature }), note))
}

/// A skill's `model: <provider>/<model>`: the provider by name (ignoring case) or id, one of the models chosen for it,
/// and allowed in local-only mode. The error says why not.
pub fn pick<'a>(m: &'a ModelSettings, local_only: bool, reference: &'a str) -> Result<(&'a Provider, &'a str), String> {
    let (provider, model) = reference.split_once('/').map(|(p, m)| (p.trim(), m.trim())).ok_or("it isn't written as <provider>/<model>")?;
    let p = m.providers.iter().find(|p| p.id == provider || p.name.to_lowercase() == provider.to_lowercase()).ok_or_else(|| format!("no provider is called {provider}"))?;
    let model = p.models.iter().find(|x| *x == model).ok_or_else(|| format!("{model} isn't one of {}'s chosen models", p.name))?;
    if local_only && p.locality(&m.trusted_hosts) == Locality::Cloud {
        return Err("local-only mode is on".into());
    }
    Ok((p, model))
}

/// A configured provider and model, behind spec 03's `Model` seam.
pub struct ProviderModel {
    app: AppHandle,
    provider: Provider,
    model: String,
    policy: Policy,
    /// A skill's temperature; None leaves the provider's default.
    temperature: Option<f64>,
}

impl Model for ProviderModel {
    fn info(&self) -> ModelInfo {
        ModelInfo {
            name: self.model.clone(),
            provider: self.provider.name.clone(),
            locality: self.provider.locality(&self.policy.trusted),
            context_tokens: context_size(&self.provider, &self.model),
            vision: vision(&self.provider, &self.model),
        }
    }

    fn stream(&self, messages: &[Message], cancel: &AtomicBool, on_delta: &mut dyn FnMut(&str)) -> Result<(), ChatError> {
        let (p, started) = (&self.provider, Instant::now());
        let key = key(p, &self.policy)?;
        let _busy = (p.kind == Kind::Builtin).then(|| engine::busy(&self.app)); // never stopped mid-answer
        let base = match p.kind {
            Kind::Builtin => engine::ensure_running(&self.app, p, &self.policy, cancel)?,
            _ => p.base_url.clone(),
        };
        let req = chat_request(p.kind, &base, &self.model, messages, key.as_deref(), self.temperature)?;
        let (mut usage, mut done, mut first) = (Usage::default(), false, None);
        let result = tauri::async_runtime::block_on(net::stream(req, &self.policy, cancel, &mut |name, data| {
            for event in parse(p.kind, &mut usage, name, data) {
                match event {
                    Event::Delta(text) => {
                        first.get_or_insert_with(|| started.elapsed());
                        on_delta(&text);
                    }
                    Event::Done(_) => {
                        done = true;
                        return Ok(Flow::Stop);
                    }
                    Event::Error(e) => return Err(e),
                }
            }
            Ok(Flow::Continue)
        }));
        let result = match result {
            Ok(()) if !done && !cancel.load(Ordering::SeqCst) => Err(net::error(ErrorKind::Network, "The connection closed before the answer was complete.")),
            r => r,
        };
        // A crashed engine explains itself better than the dropped connection does (LLM-3.6).
        let result = match result {
            Err(e) if p.kind == Kind::Builtin && e.kind == ErrorKind::Network => Err(engine::exited_within(&self.app, Duration::from_secs(1)).map_or(e, |r| net::error(ErrorKind::Server, format!("The built-in engine stopped: {r}")))),
            r => r,
        };
        // Metadata only (APP-7.2).
        log::info!(
            "llm: {:?} {} {:?}, first reply {:?}, total {:?}, {usage:?}, {:?}",
            p.kind,
            self.model,
            self.provider.locality(&self.policy.trusted),
            first,
            started.elapsed(),
            result.as_ref().err().map(|e| e.kind)
        );
        result
    }
}

/// Context sizes reported by model lists, by (provider id, model). Every list call refreshes them, health checks included.
static LISTED: LazyLock<Mutex<HashMap<(String, String), u32>>> = LazyLock::new(Mutex::default);

/// The engine's own size; otherwise the provider's override, then what its model list reported (LLM-5.3).
pub fn context_size(p: &Provider, model: &str) -> Option<u32> {
    if p.kind == Kind::Builtin {
        return Some(p.context_size.unwrap_or(engine::DEFAULT_CONTEXT));
    }
    p.context_size.or_else(|| LISTED.lock().unwrap().get(&(p.id.clone(), model.to_string())).copied())
}

/// What other specs read about a model (LLM-5.3).
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub vision: bool,
    pub context_size: Option<u32>,
}

pub fn capabilities(p: &Provider, model: &str) -> Capabilities {
    Capabilities { vision: vision(p, model), context_size: context_size(p, model) }
}

/// A provider's models (LLM-5.1). The built-in engine's model is its file.
pub async fn list_models(p: &Provider, policy: &Policy, limit: Duration) -> Result<Vec<ModelEntry>, ChatError> {
    if p.kind == Kind::Builtin {
        return Ok(p.models.iter().map(|id| ModelEntry { id: id.clone(), context_size: context_size(p, id) }).collect());
    }
    let key = key(p, policy)?;
    let j = net::json(list_request(p.kind, &p.base_url, key.as_deref())?, policy, limit).await?;
    let entries = models(p.kind, &j);
    let mut listed = LISTED.lock().unwrap();
    for e in &entries {
        if let Some(n) = e.context_size {
            listed.insert((p.id.clone(), e.id.clone()), n);
        }
    }
    Ok(entries)
}

/// The provider's API key. Local and LAN servers rarely need one, so a missing or locked keyring
/// only stops cloud providers (LLM-6.3); the built-in engine never asks.
fn key(p: &Provider, policy: &Policy) -> Result<Option<String>, ChatError> {
    if p.kind == Kind::Builtin {
        return Ok(None);
    }
    match secrets::get(&p.id) {
        Err(e) if p.locality(&policy.trusted) == Locality::Cloud => Err(net::error(ErrorKind::Auth, e)),
        r => Ok(r.ok().flatten()),
    }
}

/// Is the default model answering? A model-list call within 3 s; the engine is only checked, not started.
/// A pass is reused for 60 s, a failure never, so the next press works as soon as the model is back (LLM-9.1).
pub fn health(app: &AppHandle) -> Result<(), String> {
    static PASSED: Mutex<Option<(String, Instant)>> = Mutex::new(None);
    if echo() {
        return Ok(());
    }
    let s = app.state::<AppState>().settings.lock().unwrap().clone();
    let (p, model) = availability(&s.models, s.local_only)?;
    let key = format!("{}\n{model}\n{}", p.id, p.base_url);
    if PASSED.lock().unwrap().as_ref().is_some_and(|(k, at)| *k == key && at.elapsed() < Duration::from_secs(60)) {
        return Ok(());
    }
    let result = match p.kind {
        Kind::Builtin => engine::check(app, p),
        _ => tauri::async_runtime::block_on(list_models(p, &Policy::of(&s), Duration::from_secs(3))).map(|_| ()).map_err(|e| format!("{} isn't answering: {}", p.name, e.message)),
    };
    *PASSED.lock().unwrap() = result.is_ok().then(|| (key, Instant::now()));
    result
}

/// What every provider's stream becomes (LLM-1.2).
#[derive(Debug, PartialEq, Clone)]
pub enum Event {
    Delta(String),
    Done(Option<Usage>),
    Error(ChatError),
}

/// Token counts, when the provider reports them. Only logged.
#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct Usage {
    pub input: u32,
    pub output: u32,
}

impl Usage {
    fn known(self) -> Option<Usage> {
        (self != Usage::default()).then_some(self)
    }
}

#[derive(Serialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelEntry {
    pub id: String,
    pub context_size: Option<u32>,
}

fn bad_data() -> Event {
    Event::Error(net::error(ErrorKind::Unknown, "The model sent data Cursora couldn't read."))
}

/// LLM-2.2: trims spaces and a trailing "/", and removes a pasted "/chat/completions".
pub fn normalize_base_url(url: &str) -> String {
    let u = url.trim().trim_end_matches('/');
    u.strip_suffix("/chat/completions").unwrap_or(u).trim_end_matches('/').to_string()
}

fn join(base: &str, path: &str) -> Result<Url, ChatError> {
    Url::parse(&format!("{}{path}", normalize_base_url(base))).map_err(|_| net::error(ErrorKind::Unknown, format!("The base URL \"{base}\" isn't valid.")))
}

/// The streaming chat request for this kind of provider (LLM-1.1). The built-in engine speaks the OpenAI format.
pub fn chat_request(kind: Kind, base: &str, model: &str, messages: &[Message], key: Option<&str>, temperature: Option<f64>) -> Result<net::Request, ChatError> {
    let mut req = match kind {
        Kind::OpenAiCompatible | Kind::Builtin => openai::chat(base, model, messages, key),
        Kind::Anthropic => anthropic::chat(base, model, messages, key),
        Kind::Gemini => gemini::chat(base, model, messages, key),
    }?;
    if let (Some(t), Some(body)) = (temperature, req.body.as_mut()) {
        match kind {
            Kind::Gemini => body["generationConfig"] = serde_json::json!({ "temperature": t }),
            _ => body["temperature"] = serde_json::json!(t),
        }
    }
    Ok(req)
}

pub fn list_request(kind: Kind, base: &str, key: Option<&str>) -> Result<net::Request, ChatError> {
    match kind {
        Kind::OpenAiCompatible | Kind::Builtin => openai::list(base, key),
        Kind::Anthropic => anthropic::list(base, key),
        Kind::Gemini => gemini::list(base, key),
    }
}

/// One SSE event of a chat stream → common events (LLM-1.2).
pub fn parse(kind: Kind, usage: &mut Usage, name: &str, data: &str) -> Vec<Event> {
    match kind {
        Kind::OpenAiCompatible | Kind::Builtin => openai::parse(usage, data),
        Kind::Anthropic => anthropic::parse(usage, name, data),
        Kind::Gemini => gemini::parse(usage, data),
    }
}

/// A model-list response → model ids, with the context size where reported (LLM-5.1).
pub fn models(kind: Kind, j: &serde_json::Value) -> Vec<ModelEntry> {
    let all = match kind {
        Kind::Gemini => gemini::models(j),
        _ => openai::models(j), // Anthropic's list has the same shape
    };
    all.into_iter().filter(|m| chats(&m.id)).collect()
}

/// Model families that can't answer in text: speech, music, images, video, embeddings, safety checks, OCR,
/// computer use and the old completion-only models (LLM-5.6).
// shortcut: the name table goes stale, and OpenAI's own Responses-only models (o1-pro, o3-pro, codex) still show,
// since OpenRouter serves them as chat; a model left out can still be typed by hand.
const NOT_CHAT: &[&str] = &[
    "tts", "whisper", "transcribe", "realtime", "audio", "lyria", "imagen", "veo", "dall-e", "gpt-image", "sora", "embed",
    "moderation", "guard", "ocr", "computer-use", "davinci", "babbage",
];

fn chats(id: &str) -> bool {
    let m = id.to_ascii_lowercase();
    !NOT_CHAT.iter().any(|n| m.contains(n))
}

/// LLM-5.2: the user's override, then what the runtime reports (the engine's mmproj), then the name table.
pub fn vision(p: &Provider, model: &str) -> bool {
    if let Some(v) = p.vision_override.get(model) {
        return *v;
    }
    if p.kind == Kind::Builtin {
        return p.engine.as_ref().is_some_and(|e| e.mmproj.is_some());
    }
    vision::known(model)
}

/// Settings → Models. No field holds a key: keys live in the OS keychain (LLM-6.1).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ModelSettings {
    pub providers: Vec<Provider>,
    pub default: Option<ModelRef>,
    /// LAN hosts the user trusts to count as local (LLM-8.2).
    pub trusted_hosts: Vec<String>,
    pub timeouts: Timeouts,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    /// Made by the UI when the provider is added; also its keychain account.
    pub id: String,
    pub name: String,
    pub kind: Kind,
    /// The preset that filled it in (LLM-4.1), or "custom".
    #[serde(default)]
    pub preset: String,
    #[serde(default)]
    pub base_url: String,
    /// The models the user chose to use.
    #[serde(default)]
    pub models: Vec<String>,
    /// The engine's context size; for other kinds, an override of what the model list reports.
    #[serde(default)]
    pub context_size: Option<u32>,
    #[serde(default)]
    pub vision_override: BTreeMap<String, bool>,
    #[serde(default)]
    pub engine: Option<EngineSettings>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    #[serde(rename = "openai-compatible")]
    OpenAiCompatible,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "gemini")]
    Gemini,
    #[serde(rename = "builtin")]
    Builtin,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRef {
    pub provider: String,
    pub model: String,
}

/// The built-in engine's model files and lifetime (LLM-3).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct EngineSettings {
    pub gguf: String,
    pub mmproj: Option<String>,
    pub lifetime: Lifetime,
    /// For `Lifetime::Idle`.
    pub idle_minutes: u32,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self { gguf: String::new(), mmproj: None, lifetime: Lifetime::Always, idle_minutes: 10 }
    }
}

/// How long the built-in model stays loaded (LLM-3.5).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Lifetime {
    /// Starts with Cursora; stops on Quit, at shutdown, or by hand.
    Always,
    /// Starts on the first question; stops after `idle_minutes` without one.
    Idle,
    /// Starts for each question; stops when no conversation is on screen.
    Answering,
}

/// LLM-1.5.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Timeouts {
    pub connect_secs: u64,
    pub first_reply_local_secs: u64,
    pub first_reply_cloud_secs: u64,
    pub pause_secs: u64,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self { connect_secs: 10, first_reply_local_secs: 120, first_reply_cloud_secs: 60, pause_secs: 60 }
    }
}

impl Provider {
    pub fn locality(&self, trusted: &[String]) -> Locality {
        if self.kind == Kind::Builtin {
            return Locality::Local;
        }
        Url::parse(&self.base_url).map_or(Locality::Cloud, |u| locality(&u, trusted))
    }
}

/// Local: localhost, 127.0.0.0/8, ::1 (LLM-8.1). LAN: a private host the user trusts (LLM-8.2).
/// Everything else is cloud. Hosts are compared as typed, with no DNS lookup.
pub fn locality(url: &Url, trusted: &[String]) -> Locality {
    match url.host() {
        Some(Host::Domain(d)) if d.eq_ignore_ascii_case("localhost") => return Locality::Local,
        Some(Host::Ipv4(ip)) if ip.is_loopback() => return Locality::Local,
        Some(Host::Ipv6(ip)) if ip.is_loopback() => return Locality::Local,
        _ => {}
    }
    match private_host(url) {
        Some(h) if trusted.iter().any(|t| bare_host(t) == h) => Locality::Lan,
        _ => Locality::Cloud,
    }
}

/// The host, if it is a private (LAN) address: 10/8, 172.16/12, 192.168/16, fe80::/10 or *.local.
/// Only these can be trusted (LLM-8.2), and adding one asks whether to (LLM-8.3).
pub fn private_host(url: &Url) -> Option<String> {
    let host = url.host()?;
    let private = match &host {
        Host::Ipv4(ip) => ip.is_private(),
        Host::Ipv6(ip) => ip.segments()[0] & 0xffc0 == 0xfe80,
        Host::Domain(d) => d.to_ascii_lowercase().ends_with(".local"),
    };
    private.then(|| bare_host(&host.to_string()))
}

/// "[FE80::1]" and "fe80::1" name the same host.
pub fn bare_host(h: &str) -> String {
    h.trim().trim_matches(['[', ']']).to_ascii_lowercase()
}

/// What the bubble and the Privacy tab say when local-only mode is on and no local model is set up (PRI-1.3).
pub const NO_LOCAL_MODEL: &str = "No local model — set one up.";

/// The model the next question goes to, or why there is none (LLM-9.1).
pub fn availability(m: &ModelSettings, local_only: bool) -> Result<(&Provider, &str), String> {
    let Some(d) = &m.default else { return Err("No model is set up yet.".into()) };
    let Some(p) = m.providers.iter().find(|p| p.id == d.provider) else {
        return Err("The provider of the default model was removed.".into());
    };
    if local_only && p.locality(&m.trusted_hosts) == Locality::Cloud {
        return Err(match first_local(m) {
            None => NO_LOCAL_MODEL.into(),
            Some(_) => format!("Local-only mode is on, and {} runs in the cloud.", d.model),
        });
    }
    Ok((p, &d.model))
}

/// The first chosen model of the first local or trusted-LAN provider.
fn first_local(m: &ModelSettings) -> Option<ModelRef> {
    m.providers.iter().filter(|p| p.locality(&m.trusted_hosts) != Locality::Cloud).find_map(|p| Some(ModelRef { provider: p.id.clone(), model: p.models.first()?.clone() }))
}

/// PRI-1.3: the model to switch to as local-only mode comes on, if the default is a cloud model and a local one exists.
pub fn local_default(m: &ModelSettings) -> Option<ModelRef> {
    let d = m.default.as_ref()?;
    let cloud = m.providers.iter().find(|p| p.id == d.provider)?.locality(&m.trusted_hosts) == Locality::Cloud;
    cloud.then(|| first_local(m)).flatten()
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn provider(id: &str, kind: Kind, base_url: &str) -> Provider {
        Provider {
            id: id.into(),
            name: id.into(),
            kind,
            preset: "custom".into(),
            base_url: base_url.into(),
            models: vec!["m".into()],
            context_size: None,
            vision_override: BTreeMap::new(),
            engine: (kind == Kind::Builtin).then(EngineSettings::default),
        }
    }

    fn loc(url: &str, trusted: &[&str]) -> Locality {
        let trusted: Vec<String> = trusted.iter().map(|s| s.to_string()).collect();
        locality(&Url::parse(url).unwrap(), &trusted)
    }

    #[test]
    fn skl_2_5_a_skill_names_its_model_and_temperature() {
        let mut ollama = provider("p-1", Kind::OpenAiCompatible, "http://localhost:11434/v1");
        (ollama.name, ollama.models) = ("Ollama".into(), vec!["llama3.2".into(), "meta/llama-3".into()]);
        let mut cloud = provider("p-2", Kind::OpenAiCompatible, "https://api.openai.com/v1");
        (cloud.name, cloud.models) = ("OpenAI".into(), vec!["gpt-4o-mini".into()]);
        let m = ModelSettings { providers: vec![ollama, cloud], ..ModelSettings::default() };
        let got = |r: &str, local_only| pick(&m, local_only, r).map(|(p, model)| format!("{}/{model}", p.id));
        assert_eq!(got("ollama/llama3.2", false), Ok("p-1/llama3.2".into()), "provider names ignore case");
        assert_eq!(got("p-1/meta/llama-3", false), Ok("p-1/meta/llama-3".into()), "by id, and the model id keeps its '/'");
        assert_eq!(got("Ollama/mistral", false), Err("mistral isn't one of Ollama's chosen models".into()));
        assert_eq!(got("LM Studio/x", false), Err("no provider is called LM Studio".into()));
        assert_eq!(got("OpenAI/gpt-4o-mini", false), Ok("p-2/gpt-4o-mini".into()));
        assert_eq!(got("OpenAI/gpt-4o-mini", true), Err("local-only mode is on".into()));
        // The temperature goes where each format expects it, and only when the skill sets one.
        for (kind, path) in [(Kind::OpenAiCompatible, "/temperature"), (Kind::Anthropic, "/temperature"), (Kind::Gemini, "/generationConfig/temperature")] {
            let body = chat_request(kind, "https://host/", "m", &conversation(), Some("k"), Some(0.25)).unwrap().body.unwrap();
            assert_eq!(body.pointer(path), Some(&serde_json::json!(0.25)), "{kind:?}");
            let plain = chat_request(kind, "https://host/", "m", &conversation(), Some("k"), None).unwrap().body.unwrap();
            assert!(plain.get("temperature").is_none() && plain.get("generationConfig").is_none(), "{kind:?}");
        }
    }

    #[test]
    fn pri_1_3_local_only_switches_a_cloud_default_to_the_first_local_model() {
        let cloud = provider("c", Kind::Anthropic, "https://api.anthropic.com");
        let mut empty = provider("o", Kind::OpenAiCompatible, "http://localhost:11434/v1");
        empty.models.clear(); // set up, but no model chosen
        let lan = provider("l", Kind::OpenAiCompatible, "http://192.168.1.20:1234/v1");
        let to = |p: &str, m: &str| Some(ModelRef { provider: p.into(), model: m.into() });
        let mut m = ModelSettings { providers: vec![cloud, empty, lan], default: to("c", "m"), trusted_hosts: vec!["192.168.1.20".into()], ..ModelSettings::default() };
        assert_eq!(local_default(&m), to("l", "m"), "the first local or trusted-LAN provider with a chosen model");
        m.default = to("l", "m");
        assert_eq!(local_default(&m), None, "a local default stays");
        // With no local model, the default stays and the bubble says so.
        m = ModelSettings { trusted_hosts: vec![], default: to("c", "m"), ..m };
        assert_eq!(local_default(&m), None);
        assert_eq!(availability(&m, true).unwrap_err(), NO_LOCAL_MODEL);
        assert!(availability(&m, false).is_ok());
    }

    #[test]
    fn llm_8_1_only_this_machine_is_local() {
        for url in ["http://localhost:1234/v1", "http://LOCALHOST", "http://127.0.0.1:8080", "http://127.45.0.9", "http://[::1]:11434"] {
            assert_eq!(loc(url, &[]), Locality::Local, "{url}");
        }
        for url in ["https://api.openai.com/v1", "http://localhost.example.com", "http://8.8.8.8", "http://192.168.1.20:1234", "not a url"] {
            assert_eq!(Url::parse(url).map_or(Locality::Cloud, |u| locality(&u, &[])), Locality::Cloud, "{url}");
        }
        assert_eq!(provider("e", Kind::Builtin, "").locality(&[]), Locality::Local);
    }

    #[test]
    fn llm_8_2_trusted_private_hosts_are_lan_and_public_hosts_never_are() {
        assert_eq!(loc("http://192.168.1.20:1234/v1", &["192.168.1.20"]), Locality::Lan);
        assert_eq!(loc("http://10.0.0.5", &["10.0.0.5"]), Locality::Lan);
        assert_eq!(loc("http://172.20.1.1", &["172.20.1.1"]), Locality::Lan);
        assert_eq!(loc("http://[fe80::1]:8080", &["FE80::1"]), Locality::Lan);
        assert_eq!(loc("http://gpu-box.local:1234", &["GPU-box.local"]), Locality::Lan);
        // Private but not trusted, or trusted but public.
        assert_eq!(loc("http://192.168.1.20", &["192.168.1.21"]), Locality::Cloud);
        assert_eq!(loc("http://172.32.0.1", &["172.32.0.1"]), Locality::Cloud);
        assert_eq!(loc("https://api.openai.com", &["api.openai.com"]), Locality::Cloud);
        assert_eq!(loc("http://8.8.8.8", &["8.8.8.8"]), Locality::Cloud);
    }

    #[test]
    fn llm_9_1_availability_names_the_reason() {
        let mut m = ModelSettings::default();
        assert_eq!(availability(&m, false).unwrap_err(), "No model is set up yet.");
        m.providers = vec![provider("cloud", Kind::OpenAiCompatible, "https://api.openai.com/v1"), provider("lm", Kind::OpenAiCompatible, "http://localhost:1234/v1")];
        m.default = Some(ModelRef { provider: "gone".into(), model: "m".into() });
        assert!(availability(&m, false).unwrap_err().contains("removed"));
        m.default = Some(ModelRef { provider: "cloud".into(), model: "gpt".into() });
        assert_eq!(availability(&m, false).unwrap().0.id, "cloud");
        assert!(availability(&m, true).unwrap_err().contains("Local-only"));
        m.default = Some(ModelRef { provider: "lm".into(), model: "qwen".into() });
        assert_eq!(availability(&m, true).unwrap().1, "qwen");
    }

    use crate::chat::{Image, Role};
    use serde_json::{json, Value};

    fn conversation() -> Vec<Message> {
        let mut first = Message::new(Role::User, "what is this?");
        first.images.push(Image { mime: "image/png".into(), base64: "iVBOR".into() });
        vec![Message::new(Role::System, "be brief"), first, Message::new(Role::Assistant, "a cat"), Message::new(Role::User, "sure?")]
    }

    fn body(kind: Kind) -> (net::Request, Value) {
        let r = chat_request(kind, "https://host/base/", "m-1", &conversation(), Some("k-123"), None).unwrap();
        let b = r.body.clone().unwrap();
        (r, b)
    }

    #[test]
    fn llm_1_1_openai_compatible_request() {
        let (r, b) = body(Kind::OpenAiCompatible);
        assert_eq!(r.url.as_str(), "https://host/base/chat/completions");
        assert_eq!(r.headers, [("authorization", "Bearer k-123".to_string())]);
        assert_eq!((b["model"].clone(), b["stream"].clone()), (json!("m-1"), json!(true)));
        let roles: Vec<&str> = b["messages"].as_array().unwrap().iter().map(|m| m["role"].as_str().unwrap()).collect();
        assert_eq!(roles, ["system", "user", "assistant", "user"]);
        assert_eq!(b["messages"][0]["content"], "be brief");
        assert_eq!(b["messages"][1]["content"][0], json!({ "type": "text", "text": "what is this?" }));
        assert_eq!(b["messages"][1]["content"][1]["image_url"]["url"], "data:image/png;base64,iVBOR");
        assert_eq!(b["messages"][3]["content"], "sure?");
    }

    #[test]
    fn llm_1_1_anthropic_request() {
        let (r, b) = body(Kind::Anthropic);
        assert_eq!(r.url.as_str(), "https://host/base/v1/messages");
        assert!(r.headers.contains(&("x-api-key", "k-123".into())) && r.headers.contains(&("anthropic-version", "2023-06-01".into())));
        assert_eq!((b["system"].clone(), b["stream"].clone(), b["max_tokens"].clone()), (json!("be brief"), json!(true), json!(4096)));
        let roles: Vec<&str> = b["messages"].as_array().unwrap().iter().map(|m| m["role"].as_str().unwrap()).collect();
        assert_eq!(roles, ["user", "assistant", "user"]);
        assert_eq!(b["messages"][0]["content"][0]["source"], json!({ "type": "base64", "media_type": "image/png", "data": "iVBOR" }));
        assert_eq!(b["messages"][0]["content"][1]["text"], "what is this?");
    }

    #[test]
    fn llm_1_1_gemini_request() {
        let (r, b) = body(Kind::Gemini);
        assert_eq!(r.url.as_str(), "https://host/base/v1beta/models/m-1:streamGenerateContent?alt=sse");
        assert_eq!(r.headers, [("x-goog-api-key", "k-123".to_string())]);
        assert_eq!(b["systemInstruction"]["parts"][0]["text"], "be brief");
        let roles: Vec<&str> = b["contents"].as_array().unwrap().iter().map(|m| m["role"].as_str().unwrap()).collect();
        assert_eq!(roles, ["user", "model", "user"]);
        assert_eq!(b["contents"][0]["parts"][1]["inlineData"], json!({ "mimeType": "image/png", "data": "iVBOR" }));
    }

    /// Feeds a fixture through SSE framing in uneven chunks (1 to 17 bytes), then through the parser.
    fn run_fixture(kind: Kind, body: &str, seed: usize) -> (String, Event) {
        let mut sse = net::Sse::default();
        let mut usage = Usage::default();
        let (mut text, mut rest, mut i) = (String::new(), body.as_bytes(), seed);
        let mut events = Vec::new();
        while !rest.is_empty() {
            i = (i * 7 + 3) % 17;
            let (chunk, tail) = rest.split_at((i + 1).min(rest.len()));
            events.extend(sse.push(chunk));
            rest = tail;
        }
        events.extend(sse.finish());
        for (name, data) in events {
            for e in parse(kind, &mut usage, &name, &data) {
                match e {
                    Event::Delta(t) => text.push_str(&t),
                    end => return (text, end),
                }
            }
        }
        panic!("the stream never ended: {text:?}")
    }

    #[test]
    fn llm_1_2_every_kind_becomes_the_same_events() {
        let usage = |input, output| Event::Done(Some(Usage { input, output }));
        let error = |kind, message: &str| Event::Error(ChatError { kind, message: message.into() });
        let cases = [
            (Kind::OpenAiCompatible, include_str!("fixtures/openai.txt"), "Hello, wörld ✓", Event::Done(None)),
            // Recorded from the bundled llama-server (b11570) with Qwen2.5 0.5B.
            (Kind::Builtin, include_str!("fixtures/llama-server.txt"), "Hello there!", Event::Done(None)),
            (Kind::OpenAiCompatible, include_str!("fixtures/openrouter-error.txt"), "Partial", error(ErrorKind::Server, "Provider returned error")),
            (Kind::Anthropic, include_str!("fixtures/anthropic.txt"), "Hello, wörld ✓", usage(25, 15)),
            (Kind::Anthropic, include_str!("fixtures/anthropic-error.txt"), "Partial", error(ErrorKind::Server, "Overloaded")),
            (Kind::Gemini, include_str!("fixtures/gemini.txt"), "Hello, wörld ✓", usage(9, 5)),
            (Kind::Gemini, include_str!("fixtures/gemini-blocked.txt"), "Partial", error(ErrorKind::Unknown, "Gemini stopped the answer (SAFETY).")),
        ];
        for (kind, fixture, text, end) in cases {
            for seed in 0..5 {
                assert_eq!(run_fixture(kind, fixture, seed), (text.to_string(), end.clone()), "{kind:?} seed {seed}");
            }
        }
    }

    #[test]
    fn llm_2_2_custom_servers_take_any_base_url_and_an_optional_key() {
        assert_eq!(normalize_base_url(" http://192.168.1.5:8080/v1/ "), "http://192.168.1.5:8080/v1");
        assert_eq!(normalize_base_url("http://localhost:8000/v1/chat/completions"), "http://localhost:8000/v1");
        let r = chat_request(Kind::OpenAiCompatible, "http://localhost:8000/v1/chat/completions", "local", &conversation(), None, None).unwrap();
        assert_eq!(r.url.as_str(), "http://localhost:8000/v1/chat/completions");
        assert!(r.headers.is_empty(), "no key, no Authorization header");
        assert!(chat_request(Kind::OpenAiCompatible, "not a url", "m", &[], None, None).is_err());
    }

    #[test]
    fn llm_5_1_model_lists_are_read_from_each_api() {
        let openrouter = json!({ "data": [{ "id": "openai/gpt-4o", "context_length": 128000 }, { "id": "meta-llama/llama-3.3-70b-instruct", "context_length": 131072 }] });
        assert_eq!(models(Kind::OpenAiCompatible, &openrouter)[0], ModelEntry { id: "openai/gpt-4o".into(), context_size: Some(128000) });
        let groq = json!({ "object": "list", "data": [{ "id": "llama-3.1-8b-instant", "context_window": 131072 }] });
        assert_eq!(models(Kind::OpenAiCompatible, &groq)[0].context_size, Some(131072));
        let openai = json!({ "object": "list", "data": [{ "id": "gpt-4.1", "object": "model", "owned_by": "system" }] });
        assert_eq!(models(Kind::OpenAiCompatible, &openai), [ModelEntry { id: "gpt-4.1".into(), context_size: None }]);
        let anthropic = json!({ "data": [{ "type": "model", "id": "claude-sonnet-4-5", "display_name": "Claude Sonnet 4.5" }], "has_more": false });
        assert_eq!(models(Kind::Anthropic, &anthropic)[0].id, "claude-sonnet-4-5");
        let gemini = json!({ "models": [
            { "name": "models/gemini-2.5-flash", "inputTokenLimit": 1048576, "supportedGenerationMethods": ["generateContent", "countTokens"] },
            { "name": "models/text-embedding-004", "supportedGenerationMethods": ["embedContent"] }
        ] });
        assert_eq!(models(Kind::Gemini, &gemini), [ModelEntry { id: "gemini-2.5-flash".into(), context_size: Some(1048576) }]);
        assert!(models(Kind::OpenAiCompatible, &json!({ "unexpected": true })).is_empty());
    }

    #[test]
    fn llm_5_6_model_lists_leave_out_models_that_cant_answer_in_text() {
        // The first two after flash were offered by Gemini's list and chosen, and neither could answer (issue 16).
        let gemini = json!({ "models": [
            { "name": "models/gemini-2.5-flash", "supportedGenerationMethods": ["generateContent"] },
            { "name": "models/gemini-2.5-pro-preview-tts", "supportedGenerationMethods": ["generateContent"] },
            { "name": "models/lyria-3.5", "supportedGenerationMethods": ["generateContent"] },
            { "name": "models/gemini-embedding-001", "supportedGenerationMethods": ["generateContent", "embedContent"] }
        ] });
        let ids = |l: Vec<ModelEntry>| l.into_iter().map(|m| m.id).collect::<Vec<_>>();
        assert_eq!(ids(models(Kind::Gemini, &gemini)), ["gemini-2.5-flash"]);
        let openai = json!({ "data": [
            { "id": "gpt-4.1" }, { "id": "tts-1" }, { "id": "whisper-1" }, { "id": "gpt-4o-transcribe" }, { "id": "dall-e-3" },
            { "id": "text-embedding-3-small" }, { "id": "omni-moderation-latest" }, { "id": "gpt-4o-realtime-preview" },
            { "id": "gpt-4o-audio-preview" }, { "id": "gpt-image-1" }, { "id": "sora-2" }, { "id": "computer-use-preview" },
            { "id": "davinci-002" }, { "id": "babbage-002" }
        ] });
        assert_eq!(ids(models(Kind::OpenAiCompatible, &openai)), ["gpt-4.1"]);
        let groq = json!({ "data": [
            { "id": "llama-3.3-70b-versatile" }, { "id": "whisper-large-v3-turbo" }, { "id": "playai-tts" },
            { "id": "meta-llama/llama-guard-4-12b" }, { "id": "meta-llama/llama-prompt-guard-2-86m" }
        ] });
        assert_eq!(ids(models(Kind::OpenAiCompatible, &groq)), ["llama-3.3-70b-versatile"]);
        let mistral = json!({ "data": [{ "id": "mistral-large-latest" }, { "id": "mistral-embed" }, { "id": "mistral-moderation-latest" }, { "id": "mistral-ocr-latest" }] });
        assert_eq!(ids(models(Kind::OpenAiCompatible, &mistral)), ["mistral-large-latest"]);
    }

    #[test]
    fn llm_5_2_vision_comes_from_the_override_then_the_runtime_then_the_table() {
        let mut cloud = provider("c", Kind::OpenAiCompatible, "https://openrouter.ai/api/v1");
        assert!(vision(&cloud, "openai/gpt-4o") && vision(&cloud, "qwen/qwen2.5-vl-72b-instruct") && vision(&cloud, "claude-sonnet-4-5"));
        assert!(!vision(&cloud, "deepseek-chat") && !vision(&cloud, "o3-mini") && !vision(&cloud, "gemma3:1b"));
        cloud.vision_override.insert("deepseek-chat".into(), true);
        cloud.vision_override.insert("gpt-4o".into(), false);
        assert!(vision(&cloud, "deepseek-chat") && !vision(&cloud, "gpt-4o"));
        let mut engine = provider("e", Kind::Builtin, "");
        assert!(!vision(&engine, "llava-v1.6"), "the engine reports no mmproj, so the name table isn't asked");
        engine.engine.as_mut().unwrap().mmproj = Some("mmproj.gguf".into());
        assert!(vision(&engine, "anything"));
    }

    #[test]
    fn llm_6_3_a_broken_keyring_stops_only_cloud_providers() {
        let fail = |id: &str| {
            secrets::get(id).unwrap(); // puts the mock store in place
            keyring_core::Entry::new("cursora", id)
                .unwrap().as_any().downcast_ref::<keyring_core::mock::Cred>().unwrap().set_error(keyring_core::Error::NoStorageAccess("locked".into()));
        };
        let policy = Policy::default();
        let lm = provider("lm-63", Kind::OpenAiCompatible, "http://localhost:1234/v1");
        fail("lm-63");
        assert_eq!(key(&lm, &policy).unwrap(), None);
        let cloud = provider("cloud-63", Kind::OpenAiCompatible, "https://api.openai.com/v1");
        fail("cloud-63");
        assert_eq!(key(&cloud, &policy).unwrap_err().kind, ErrorKind::Auth);
        assert_eq!(key(&provider("e-63", Kind::Builtin, ""), &policy).unwrap(), None);
    }

    use net::tests::{serve, Step};

    fn block<T>(f: impl Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(f)
    }

    fn server(id: &str, port: u16) -> Provider {
        provider(id, Kind::OpenAiCompatible, &format!("http://127.0.0.1:{port}/v1"))
    }

    const MODELS: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"data\":[{\"id\":\"a\",\"context_length\":32000},{\"id\":\"b\"}]}";

    #[test]
    fn llm_7_1_test_connection_reports_ok_or_the_error_kind_within_its_limit() {
        let (port, _) = serve(vec![Step::Write(MODELS)]);
        assert_eq!(block(list_models(&server("ok", port), &Policy::default(), Duration::from_secs(15))).unwrap().len(), 2);
        let (port, _) = serve(vec![Step::Write("HTTP/1.1 401 Unauthorized\r\nConnection: close\r\nContent-Length: 50\r\n\r\n{\"error\":{\"message\":\"Incorrect API key provided\"}}")]);
        let e = block(list_models(&server("bad-key", port), &Policy::default(), Duration::from_secs(15))).unwrap_err();
        assert_eq!((e.kind, e.message.as_str()), (ErrorKind::Auth, "Incorrect API key provided"));
        let (port, _) = serve(vec![Step::Sleep(5000)]);
        let start = Instant::now();
        let e = block(list_models(&server("stalls", port), &Policy::default(), Duration::from_millis(300))).unwrap_err();
        assert_eq!(e.kind, ErrorKind::Network);
        assert!(start.elapsed() < Duration::from_secs(1), "took {:?}", start.elapsed());
    }

    #[test]
    fn llm_9_1_the_health_check_list_call_gives_up_at_its_limit() {
        // health() runs this call with a 3 s limit; a server that never answers can't hold the bubble up.
        let (port, _) = serve(vec![Step::Sleep(5000)]);
        let start = Instant::now();
        assert!(block(list_models(&server("dead", port), &Policy::default(), Duration::from_millis(200))).is_err());
        assert!(start.elapsed() < Duration::from_millis(800));
    }

    #[test]
    fn llm_3_4_an_mmproj_file_makes_the_built_in_model_vision_capable() {
        let mut p = provider("eng-34", Kind::Builtin, "");
        assert!(!capabilities(&p, "qwen").vision);
        p.engine.as_mut().unwrap().mmproj = Some("D:/models/mmproj-qwen.gguf".into());
        assert!(capabilities(&p, "qwen").vision);
        assert!(engine::args(p.engine.as_ref().unwrap(), 8192, 1).windows(2).any(|w| w == ["--mmproj", "D:/models/mmproj-qwen.gguf"]));
    }

    #[test]
    fn llm_5_3_capabilities_give_vision_and_context_size() {
        let mut engine = provider("eng", Kind::Builtin, "");
        assert_eq!(capabilities(&engine, "m"), Capabilities { vision: false, context_size: Some(8192) });
        engine.context_size = Some(4096);
        engine.engine.as_mut().unwrap().mmproj = Some("mmproj.gguf".into());
        assert_eq!(capabilities(&engine, "m"), Capabilities { vision: true, context_size: Some(4096) });
        // A model list that reports sizes makes them known.
        let (port, _) = serve(vec![Step::Write(MODELS)]);
        let lm = server("lm-53", port);
        assert_eq!(capabilities(&lm, "a").context_size, None);
        block(list_models(&lm, &Policy::default(), Duration::from_secs(5))).unwrap();
        assert_eq!(capabilities(&lm, "a"), Capabilities { vision: false, context_size: Some(32000) });
        assert_eq!(capabilities(&lm, "b").context_size, None);
        let cloud = provider("cloud-53", Kind::OpenAiCompatible, "https://api.openai.com/v1");
        assert_eq!(capabilities(&cloud, "gpt-4o"), Capabilities { vision: true, context_size: None });
    }
}
