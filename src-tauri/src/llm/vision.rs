//! Known vision model names, LLM-5.2 step 3.
// shortcut: the name table goes stale; the per-model override in Settings covers new models.

/// Names start with one of these (after any "vendor/" prefix)...
const PREFIXES: &[&str] = &[
    "gpt-4o", "gpt-4.1", "gpt-4-turbo", "gpt-5", "o1", "o3", "o4", "chatgpt-4o", "claude", "gemini", "llava", "bakllava",
    "llama-4", "llama4", "pixtral", "minicpm-v", "moondream", "gemma-3", "gemma3", "mistral-small-3.1", "mistral-small3.1",
    "mistral-medium", "grok-4", "grok-2-vision",
];
/// ...or contain one of these...
const PARTS: &[&str] = &["-vl", "vision", "-vlm"];
/// ...and contain none of these.
const NOT: &[&str] = &["o1-mini", "o3-mini", "gemma-3-1b", "gemma3:1b", "gemma-3n", "embedding", "audio", "tts", "transcribe", "realtime", "search"];

pub fn known(model: &str) -> bool {
    let m = model.to_ascii_lowercase();
    let m = m.rsplit('/').next().unwrap_or(&m);
    (PREFIXES.iter().any(|p| m.starts_with(p)) || PARTS.iter().any(|p| m.contains(p))) && !NOT.iter().any(|n| m.contains(n))
}
