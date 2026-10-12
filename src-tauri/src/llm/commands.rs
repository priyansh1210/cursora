//! Commands for Settings → Models, Setup's "AI source" step and the bubble's model switcher.
//! Providers themselves are saved through `set_settings`, like every other setting.
use super::{capabilities, engine, list_models, local, net::Policy, Kind, ModelRef, Provider};
use crate::chat::{self, ChatError, Conversation, ErrorKind, Locality};
use crate::{secrets, AppState};
use serde::Serialize;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewWindow};

fn settings(app: &AppHandle) -> crate::settings::Settings {
    app.state::<AppState>().settings.lock().unwrap().clone()
}

fn find(app: &AppHandle, id: &str) -> Result<(Provider, Policy), ChatError> {
    let s = settings(app);
    let p = s.models.providers.iter().find(|p| p.id == id).cloned();
    p.map(|p| (p, Policy::of(&s))).ok_or_else(|| ChatError { kind: ErrorKind::Unknown, message: "This provider was removed.".into() })
}

/// Everything about a provider the UI may see: never the key, only whether there is one and its last 4 characters (LLM-6.2).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderView {
    #[serde(flatten)]
    provider: Provider,
    locality: Locality,
    #[serde(flatten)]
    key: secrets::KeyStatus,
}

#[tauri::command(async)]
pub fn providers_list(app: AppHandle) -> Vec<ProviderView> {
    let s = settings(&app);
    s.models.providers.into_iter().map(|p| ProviderView { locality: p.locality(&s.models.trusted_hosts), key: secrets::status(&p.id), provider: p }).collect()
}

#[tauri::command(async)]
pub fn key_set(provider: String, key: String) -> Result<(), String> {
    secrets::set(&provider, &key)
}

#[tauri::command(async)]
pub fn key_delete(provider: String) {
    secrets::delete(&provider);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelView {
    id: String,
    context_size: Option<u32>,
    vision: bool,
}

/// LLM-5.1. On failure the UI shows the error and a field to type a model id.
#[tauri::command]
pub async fn models_list(app: AppHandle, provider: String) -> Result<Vec<ModelView>, ChatError> {
    let (p, policy) = find(&app, &provider)?;
    let models = list_models(&p, &policy, Duration::from_secs(15)).await?;
    Ok(models.into_iter().map(|m| ModelView { vision: capabilities(&p, &m.id).vision, context_size: m.context_size, id: m.id }).collect())
}

/// Test connection (LLM-7.1): the model list within 15 s; for the built-in engine, starting it.
/// Returns how many models were found.
#[tauri::command]
pub async fn provider_test(app: AppHandle, provider: String) -> Result<usize, ChatError> {
    let (p, policy) = find(&app, &provider)?;
    if p.kind == Kind::Builtin {
        // Blocks while the model loads, so it runs on a blocking thread.
        let started = tauri::async_runtime::spawn_blocking(move || engine::ensure_running(&app, &p, &policy, &AtomicBool::new(false)));
        return started.await.map_err(|e| ChatError { kind: ErrorKind::Unknown, message: e.to_string() })?.map(|_| 1);
    }
    list_models(&p, &policy, Duration::from_secs(15)).await.map(|m| m.len())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detected {
    name: &'static str,
    preset: &'static str,
    base_url: &'static str,
    models: Vec<String>,
}

/// LLM-2.1: LM Studio on this computer.
#[tauri::command]
pub async fn runtimes_probe() -> Vec<Detected> {
    let lm = local::probe(local::LM_STUDIO).await;
    lm.map(|models| Detected { name: "LM Studio", preset: "lmstudio", base_url: local::LM_STUDIO, models }).into_iter().collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RamSuggestion {
    total_gb: f64,
    size: &'static str,
}

/// LLM-2.4.
#[tauri::command]
pub fn ram_suggestion() -> Option<RamSuggestion> {
    local::total_ram().map(|b| RamSuggestion { total_gb: (b as f64 / (1u64 << 30) as f64 * 10.0).round() / 10.0, size: local::suggest_size(b) })
}

/// LLM-8.3: the host to ask about, if this base URL is on a private (LAN) address.
#[tauri::command]
pub fn lan_host(url: String) -> Option<String> {
    url::Url::parse(url.trim()).ok().and_then(|u| super::private_host(&u))
}

/// The built-in engine's file picker (LLM-3.2, 3.4).
#[tauri::command]
pub async fn pick_model_file(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().add_filter("GGUF model", &["gguf"]).pick_file(move |f| {
        let _ = tx.send(f.map(|f| f.to_string()));
    });
    rx.await.ok().flatten()
}

#[tauri::command(async)]
pub fn engine_status(app: AppHandle) -> engine::Status {
    engine::status(&app)
}

/// "Stop the model" in the Models tab (LLM-3.5).
#[tauri::command(async)]
pub fn engine_stop(app: AppHandle) {
    engine::stop_by_hand(&app);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Enabled {
    provider: String,
    provider_name: String,
    model: String,
    locality: Locality,
    /// Cloud models while local-only is on (LLM-5.5).
    disabled: bool,
}

/// The chosen models of every provider, for the default-model picker and the bubble's switcher.
#[tauri::command(async)]
pub fn enabled_models(app: AppHandle) -> Vec<Enabled> {
    let s = settings(&app);
    let mut out = Vec::new();
    for p in &s.models.providers {
        let locality = p.locality(&s.models.trusted_hosts);
        for m in &p.models {
            let disabled = s.local_only && locality == Locality::Cloud;
            out.push(Enabled { provider: p.id.clone(), provider_name: p.name.clone(), model: m.clone(), locality, disabled });
        }
    }
    out
}

/// The switcher (LLM-5.5): makes this the default, and the calling window's conversation answers with it from the next question.
#[tauri::command(async)]
pub fn set_model(window: WebviewWindow, app: AppHandle, provider: String, model: String) -> Result<Option<Conversation>, String> {
    let mut s = settings(&app);
    s.models.default = Some(ModelRef { provider, model });
    crate::update_settings(&app, s)?;
    let info = super::active(&app).map(|m| m.info());
    Ok(chat::switch_model(&app, window.label(), info))
}
