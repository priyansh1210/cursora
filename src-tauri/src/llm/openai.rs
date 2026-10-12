//! OpenAI-compatible servers: OpenAI, OpenRouter, Groq, DeepSeek, Mistral, LM Studio, llama.cpp,
//! any custom server, and the built-in engine.
use super::net::{self, Request};
use super::{bad_data, join, Event, ModelEntry, Usage};
use crate::chat::{ChatError, Message, Role};
use serde_json::{json, Value};

pub fn chat(base: &str, model: &str, messages: &[Message], key: Option<&str>) -> Result<Request, ChatError> {
    let messages: Vec<Value> = messages.iter().map(message).collect();
    Ok(Request { url: join(base, "/chat/completions")?, headers: bearer(key), body: Some(json!({ "model": model, "stream": true, "messages": messages })) })
}

fn message(m: &Message) -> Value {
    let role = match m.role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
    };
    if m.images.is_empty() {
        return json!({ "role": role, "content": m.text });
    }
    let mut parts = vec![json!({ "type": "text", "text": m.text })];
    parts.extend(m.images.iter().map(|i| json!({ "type": "image_url", "image_url": { "url": format!("data:{};base64,{}", i.mime, i.base64) } })));
    json!({ "role": role, "content": parts })
}

pub fn list(base: &str, key: Option<&str>) -> Result<Request, ChatError> {
    Ok(Request { url: join(base, "/models")?, headers: bearer(key), body: None })
}

fn bearer(key: Option<&str>) -> Vec<(&'static str, String)> {
    key.map(|k| vec![("authorization", format!("Bearer {k}"))]).unwrap_or_default()
}

/// One SSE event. The answer is done at the first `finish_reason` or at `[DONE]`.
pub fn parse(usage: &mut Usage, data: &str) -> Vec<Event> {
    if data.trim() == "[DONE]" {
        return vec![Event::Done(usage.known())];
    }
    let Ok(j) = serde_json::from_str::<Value>(data) else { return vec![bad_data()] };
    if j.get("error").is_some() {
        return vec![Event::Error(net::http_error(None, data))];
    }
    if let Some(u) = j.get("usage").filter(|u| u.is_object()) {
        *usage = Usage { input: num(&u["prompt_tokens"]), output: num(&u["completion_tokens"]) };
    }
    let choice = &j["choices"][0];
    let mut events = Vec::new();
    // Thinking text (`reasoning_content`) isn't shown; it still resets the pause timer upstream.
    if let Some(text) = choice.pointer("/delta/content").and_then(Value::as_str).filter(|t| !t.is_empty()) {
        events.push(Event::Delta(text.into()));
    }
    if choice.get("finish_reason").is_some_and(|f| !f.is_null()) {
        events.push(Event::Done(usage.known()));
    }
    events
}

fn num(v: &Value) -> u32 {
    v.as_u64().unwrap_or(0) as u32
}

/// `{"data": [{"id": …}]}`; also Anthropic's list. The context size where the provider reports one.
pub fn models(j: &Value) -> Vec<ModelEntry> {
    let keys = ["context_length", "context_window", "max_context_length", "max_input_tokens"];
    j["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let id = m["id"].as_str()?.to_string();
            let context_size = keys.iter().find_map(|k| m[*k].as_u64()).map(|n| n as u32);
            Some(ModelEntry { id, context_size })
        })
        .collect()
}
