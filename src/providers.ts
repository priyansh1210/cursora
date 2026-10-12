// Types mirror src-tauri/src/llm/mod.rs and llm/commands.rs.
import type { ChatError, Locality } from "./conversation";

export type Kind = "openai-compatible" | "anthropic" | "gemini" | "builtin";
export type Lifetime = "always" | "idle" | "answering";
export type Engine = { gguf: string; mmproj: string | null; lifetime: Lifetime; idleMinutes: number };

/** LLM-3.5: how long the built-in model stays loaded. "always" is the default. */
export const LIFETIMES: { value: Lifetime; label: string; hint: string }[] = [
  { value: "always", label: "Always on", hint: "Loaded with Cursora, ready at once. Stays until you stop it or quit." },
  { value: "idle", label: "Off when idle", hint: "Unloads after some minutes without questions, freeing memory." },
  { value: "answering", label: "Only while answering", hint: "Loads for each question and unloads when the bubble closes. Uses the least memory." },
];
export type Provider = {
  id: string;
  name: string;
  kind: Kind;
  preset: string;
  baseUrl: string;
  models: string[];
  contextSize: number | null;
  visionOverride: Record<string, boolean>;
  engine: Engine | null;
};
export type ProviderView = Provider & { locality: Locality; hasKey: boolean; keyHint: string | null };
export type ModelView = { id: string; contextSize: number | null; vision: boolean };
export type Enabled = { provider: string; providerName: string; model: string; locality: Locality; disabled: boolean };
export type EngineStatus = { state: "stopped" | "starting" | "running" } | { state: "failed"; reason: string };
export type Detected = { name: string; preset: string; baseUrl: string; models: string[] };
export type { ChatError };

export type Preset = { id: string; name: string; kind: Kind; baseUrl: string; needsKey: boolean };

/** LLM-4.1: each preset fills in the kind and base URL. */
export const PRESETS: Preset[] = [
  { id: "openai", name: "OpenAI", kind: "openai-compatible", baseUrl: "https://api.openai.com/v1", needsKey: true },
  { id: "anthropic", name: "Anthropic", kind: "anthropic", baseUrl: "https://api.anthropic.com", needsKey: true },
  { id: "gemini", name: "Google Gemini", kind: "gemini", baseUrl: "https://generativelanguage.googleapis.com", needsKey: true },
  { id: "openrouter", name: "OpenRouter", kind: "openai-compatible", baseUrl: "https://openrouter.ai/api/v1", needsKey: true },
  { id: "groq", name: "Groq", kind: "openai-compatible", baseUrl: "https://api.groq.com/openai/v1", needsKey: true },
  { id: "deepseek", name: "DeepSeek", kind: "openai-compatible", baseUrl: "https://api.deepseek.com/v1", needsKey: true },
  { id: "mistral", name: "Mistral", kind: "openai-compatible", baseUrl: "https://api.mistral.ai/v1", needsKey: true },
  { id: "lmstudio", name: "LM Studio", kind: "openai-compatible", baseUrl: "http://localhost:1234/v1", needsKey: false },
  { id: "custom", name: "Custom (OpenAI-compatible)", kind: "openai-compatible", baseUrl: "", needsKey: false },
];

export function newProvider(p: Preset, name: string, baseUrl: string, now = Date.now()): Provider {
  return { id: `p-${now}`, name: name.trim() || p.name, kind: p.kind, preset: p.id, baseUrl: baseUrl.trim(), models: [], contextSize: null, visionOverride: {}, engine: null };
}

/** The built-in engine runs one model file; its model name is the file's name. */
export function engineProvider(gguf: string, mmproj: string | null, now = Date.now()): Provider {
  return {
    id: `p-${now}`,
    name: "Built-in engine",
    kind: "builtin",
    preset: "builtin",
    baseUrl: "",
    models: [fileStem(gguf)],
    contextSize: null,
    visionOverride: {},
    engine: { gguf, mmproj, lifetime: "always", idleMinutes: 10 },
  };
}

export function fileStem(path: string): string {
  return (path.split(/[\\/]/).pop() ?? path).replace(/\.gguf$/i, "");
}
