//! Anthropic's Messages API.
use super::net::{self, Request};
use super::{bad_data, join, Event, Usage};
use crate::chat::{ChatError, Message, Role};
use serde_json::{json, Value};

const VERSION: &str = "2023-06-01";
// shortcut: fixed 4096 answer limit for Anthropic; make it a setting if long answers get cut off.
const MAX_TOKENS: u32 = 4096;

pub fn chat(base: &str, model: &str, messages: &[Message], key: Option<&str>) -> Result<Request, ChatError> {
    let system: Vec<&str> = messages.iter().filter(|m| m.role == Role::System).map(|m| m.text.as_str()).collect();
    let turns: Vec<Value> = messages.iter().filter(|m| m.role != Role::System).map(message).collect();
    let body = json!({ "model": model, "max_tokens": MAX_TOKENS, "stream": true, "system": system.join("\n\n"), "messages": turns });
    Ok(Request { url: join(base, "/v1/messages")?, headers: headers(key), body: Some(body) })
}

fn message(m: &Message) -> Value {
    let role = if m.role == Role::Assistant { "assistant" } else { "user" };
    if m.images.is_empty() {
        return json!({ "role": role, "content": m.text });
    }
    let mut parts: Vec<Value> = m.images.iter().map(|i| json!({ "type": "image", "source": { "type": "base64", "media_type": i.mime, "data": i.base64 } })).collect();
    parts.push(json!({ "type": "text", "text": m.text }));
    json!({ "role": role, "content": parts })
}

pub fn list(base: &str, key: Option<&str>) -> Result<Request, ChatError> {
    Ok(Request { url: join(base, "/v1/models?limit=1000")?, headers: headers(key), body: None })
}

fn headers(key: Option<&str>) -> Vec<(&'static str, String)> {
    let mut h = vec![("anthropic-version", VERSION.to_string())];
    h.extend(key.map(|k| ("x-api-key", k.to_string())));
    h
}

/// One SSE event; the event name says what it is.
pub fn parse(usage: &mut Usage, name: &str, data: &str) -> Vec<Event> {
    let Ok(j) = serde_json::from_str::<Value>(data) else { return vec![bad_data()] };
    let n = |p: &str| j.pointer(p).and_then(Value::as_u64).unwrap_or(0) as u32;
    match name {
        "message_start" => usage.input = n("/message/usage/input_tokens"),
        "message_delta" => usage.output = n("/usage/output_tokens"),
        // Thinking blocks (`thinking_delta`) aren't shown.
        "content_block_delta" if j.pointer("/delta/type").and_then(Value::as_str) == Some("text_delta") => {
            return j.pointer("/delta/text").and_then(Value::as_str).map(|t| vec![Event::Delta(t.into())]).unwrap_or_default();
        }
        "message_stop" => return vec![Event::Done(usage.known())],
        "error" => return vec![Event::Error(net::http_error(None, data))],
        _ => {} // ping, content_block_start, content_block_stop
    }
    vec![]
}
