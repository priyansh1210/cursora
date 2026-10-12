# 04: AI providers (LLM)

## Scope
- **In:**
  - Configuring AI sources:
    - **local**: Ollama, LM Studio, any OpenAI-compatible server, or the built-in llama.cpp engine running any `.gguf` file
    - **cloud**: OpenAI, Anthropic, Gemini, OpenRouter, Groq, DeepSeek, Mistral, or a custom endpoint
  - Model listing and capability detection
  - Streaming, errors and cancellation
  - API key storage, the connection test, and classifying a provider as local or cloud
- **Out:**
  - Automatic routing and fallback (backlog)
  - Cost tracking (backlog)
  - Speech-to-text providers (VOC)

## Requirements

### LLM-1 Provider abstraction and streaming
- LLM-1.1 (auto) THE SYSTEM SHALL support the provider kinds `openai-compatible`, `anthropic`, `gemini` and `builtin`. Each SHALL support:
  - a system prompt
  - multi-turn messages
  - streaming
  - image inputs, where the model supports them
- LLM-1.2 (auto) THE SYSTEM SHALL convert each provider's stream format (SSE or NDJSON) into one common event sequence: `delta(text)`, then `done(usage?)` or `error(kind, message)`. This SHALL be verified against recorded fixtures for every kind.
- LLM-1.3 (auto) WHEN a request is cancelled THE SYSTEM SHALL abort it within 100 ms and emit no further deltas.
- LLM-1.4 (auto) THE SYSTEM SHALL classify failures from the HTTP status and response body into: `auth`, `rate_limit`, `model_not_found`, `context_too_long`, `network`, `server`, `blocked_local_only`, `unknown`.
- LLM-1.5 (auto) Timeouts:
  - connecting: 10 s
  - waiting for the first token: 120 s for local providers, 60 s for cloud
  - between tokens: 60 s

  Each SHALL be configurable.

### LLM-2 Local runtimes: any way the user runs a model
User story: As a user with my own machine, I want to use whatever local model setup I already have.
- LLM-2.1 (auto) WHEN the Models tab opens, or onboarding runs, THE SYSTEM SHALL probe Ollama (`http://localhost:11434`) and LM Studio (`http://localhost:1234`), and list the runtimes found with their models.
- LLM-2.2 (auto) THE user SHALL be able to add any OpenAI-compatible endpoint by base URL, with an optional key. This covers llama.cpp server, vLLM, Jan, LocalAI, KoboldCpp, text-generation-webui and GPT4All.
- LLM-2.3 (manual) WHERE Ollama is detected THE user SHALL be able to pull a model by name, with progress and cancel.
- LLM-2.4 (auto) THE SYSTEM SHALL suggest a model size from total RAM:

  | Total RAM | Suggested model size |
  |---|---|
  | < 8 GB | 1–3B |
  | 8–15 GB | 3–4B |
  | 16–31 GB | 7–8B |
  | ≥ 32 GB | up to 14B |

### LLM-3 Built-in engine (no external app needed)
User story: As a user, I want to just pick a model file and go, without installing Ollama.
- LLM-3.1 (manual) THE installers SHALL bundle llama.cpp's `llama-server` as a sidecar for Windows and Linux (CPU build).
- LLM-3.2 (manual) WHEN the user selects a `.gguf` file THE SYSTEM SHALL start the sidecar on a free port, wait for its health check, and register it as a local provider.
- LLM-3.3 (auto) THE sidecar SHALL be started bound to `127.0.0.1` only.
- LLM-3.4 (auto) WHERE the user also selects a multimodal projector (`mmproj`) file THE built-in model SHALL be marked as vision-capable.
- LLM-3.5 (manual) THE sidecar SHALL:
  - start on the first request, or at startup if "Keep model loaded" is on
  - stop after 10 idle minutes (configurable)
  - stop when Cursora quits
- LLM-3.6 (manual) IF the sidecar exits unexpectedly or fails to load the model THE SYSTEM SHALL show the reason in the bubble and in the Models tab.

### LLM-4 Cloud providers
- LLM-4.1 (auto) THE SYSTEM SHALL offer presets that fill in the kind and base URL: OpenAI, Anthropic, Google Gemini, OpenRouter, Groq, DeepSeek and Mistral. A "Custom (OpenAI-compatible)" option SHALL also be available.
- LLM-4.2 (manual) THE user SHALL be able to add, edit and remove providers. Each provider has a name, kind, base URL and API key.

### LLM-5 Models and capabilities
- LLM-5.1 (auto) THE SYSTEM SHALL list models from each provider's model-list endpoint, with no model IDs hardcoded. IF listing fails THE user SHALL be able to type a model ID by hand.
- LLM-5.2 (auto) THE SYSTEM SHALL decide whether a model supports vision by checking these sources in order:
  1. the user's override
  2. what the runtime reports (Ollama `/api/show` capabilities, the built-in engine's `mmproj`)
  3. the bundled table of known name patterns
  4. otherwise, `false`
- LLM-5.3 (auto) THE SYSTEM SHALL expose `capabilities(model) → { vision, contextTokens? }` for other specs to read.
- LLM-5.4 (manual) THE user SHALL pick one default model. Skills can override it (SKL-2.5).

### LLM-6 API key storage
- LLM-6.1 (auto) THE SYSTEM SHALL store API keys only in the OS keychain (Windows Credential Manager or Linux Secret Service), under service `cursora` and account `<provider-id>`.
- LLM-6.2 (auto) THE webview SHALL never receive a key. The UI sees only `hasKey` and the last 4 characters.
- LLM-6.3 (manual) IF the OS keychain is unavailable (e.g. no Secret Service on Linux) THE SYSTEM SHALL explain how to enable it, and SHALL NOT fall back to storing keys in plaintext.

### LLM-7 Connection test
- LLM-7.1 (auto) WHEN the user clicks "Test connection" THE SYSTEM SHALL list models (or, where listing is unsupported, request a 1-token completion). It SHALL report OK or the error kind (LLM-1.4) within 15 s.

### LLM-8 Local vs cloud
- LLM-8.1 (auto) A provider SHALL be classified **local** if and only if:
  - its kind is `builtin`, or
  - its base URL host is `localhost`, inside `127.0.0.0/8`, or `::1`.

  Every other provider SHALL be classified **cloud** for privacy purposes (PRI), unless LLM-8.2 applies.
- LLM-8.2 (auto) WHERE the user has marked a LAN host as **trusted** THE providers on that host SHALL count as local for local-only mode and redaction (PRI). A LAN host is a private address: `10/8`, `172.16/12`, `192.168/16`, `fe80::/10` or `*.local`. Public hosts SHALL never count as trusted.
- LLM-8.3 (manual) WHEN the user adds a provider on a LAN host (in setup or in the Models tab) THE SYSTEM SHALL ask whether to trust that host. The answer SHALL be changeable later in the Models tab.

### LLM-9 No working model
- LLM-9.1 (auto) WHEN the ask hotkey is pressed AND no model is configured, OR the active provider fails a health check (a model-list call that times out after 3 s), THE bubble SHALL say **"No AI model is available"**, give the reason, and offer "Open setup". It SHALL NOT show a chat input until a model works.

## Open questions
- _(none. LAN trust is decided during setup, per the user's answer.)_
