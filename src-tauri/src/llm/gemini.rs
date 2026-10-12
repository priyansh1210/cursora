//! Google's Gemini API.
use super::net::{self, Request};
use super::{bad_data, join, Event, ModelEntry, Usage};
use crate::chat::{ChatError, ErrorKind, Message, Role};
use serde_json::{json, Value};

pub fn chat(base: &str, model: &str, messages: &[Message], key: Option<&str>) -> Result<Request, ChatError> {
    let model = model.strip_prefix("models/").unwrap_or(model);
    let system: Vec<&str> = messages.iter().filter(|m| m.role == Role::System).map(|m| m.text.as_str()).collect();
    let contents: Vec<Value> = messages.iter().filter(|m| m.role != Role::System).map(content).collect();
    let body = json!({ "systemInstruction": { "parts": [{ "text": system.join("\n\n") }] }, "contents": contents });
    Ok(Request { url: join(base, &format!("/v1beta/models/{model}:streamGenerateContent?alt=sse"))?, headers: header(key), body: Some(body) })
}

fn content(m: &Message) -> Value {
    let role = if m.role == Role::Assistant { "model" } else { "user" };
    let mut parts = vec![json!({ "text": m.text })];
    parts.extend(m.images.iter().map(|i| json!({ "inlineData": { "mimeType": i.mime, "data": i.base64 } })));
    json!({ "role": role, "parts": parts })
}

pub fn list(base: &str, key: Option<&str>) -> Result<Request, ChatError> {
    Ok(Request { url: join(base, "/v1beta/models?pageSize=1000")?, headers: header(key), body: None })
}

fn header(key: Option<&str>) -> Vec<(&'static str, String)> {
    key.map(|k| vec![("x-goog-api-key", k.to_string())]).unwrap_or_default()
}

/// One SSE event: a chunk of the answer. The last one has a `finishReason`; the stream then closes.
pub fn parse(usage: &mut Usage, data: &str) -> Vec<Event> {
    let Ok(j) = serde_json::from_str::<Value>(data) else { return vec![bad_data()] };
    if j.get("error").is_some() {
        return vec![Event::Error(net::http_error(None, data))];
    }
    if let Some(reason) = j.pointer("/promptFeedback/blockReason").and_then(Value::as_str) {
        return vec![Event::Error(net::error(ErrorKind::Unknown, format!("Gemini blocked the request ({reason}).")))];
    }
    if let Some(u) = j.get("usageMetadata") {
        *usage = Usage { input: u["promptTokenCount"].as_u64().unwrap_or(0) as u32, output: u["candidatesTokenCount"].as_u64().unwrap_or(0) as u32 };
    }
    let candidate = &j["candidates"][0];
    let text: String = candidate["content"]["parts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["thought"] != true) // thinking isn't shown
        .filter_map(|p| p["text"].as_str())
        .collect();
    let mut events = Vec::new();
    if !text.is_empty() {
        events.push(Event::Delta(text));
    }
    match candidate["finishReason"].as_str() {
        None | Some("") | Some("FINISH_REASON_UNSPECIFIED") => {}
        Some("STOP" | "MAX_TOKENS") => events.push(Event::Done(usage.known())),
        Some(other) => events.push(Event::Error(net::error(ErrorKind::Unknown, format!("Gemini stopped the answer ({other})."))))
    }
    events
}

/// Only models that can chat (`generateContent`); names lose their "models/" prefix.
pub fn models(j: &Value) -> Vec<ModelEntry> {
    j["models"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|m| m["supportedGenerationMethods"].as_array().is_some_and(|a| a.iter().any(|x| x == "generateContent")))
        .filter_map(|m| {
            let id = m["name"].as_str()?;
            Some(ModelEntry { id: id.strip_prefix("models/").unwrap_or(id).to_string(), context_size: m["inputTokenLimit"].as_u64().map(|n| n as u32) })
        })
        .collect()
}
