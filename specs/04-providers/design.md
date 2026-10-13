# 04: AI providers (design)

Status: **Approved** (2026-10-12). Implements [requirements.md](requirements.md). It replaces spec 03's test model with real ones.

## Overview
```
Hotkey ─┬─ capture (spec 02) ───────────────────────→ the bubble opens at once (BUB-2.1)
        └─ health check (LLM-9.1: ≤ 3 s, a pass is reused for 60 s) ─→ if it fails, the bubble swaps its input for "No AI model is available"

Ask ─→ chat worker (spec 03) ─→ llm::active(app) → ProviderModel::stream(messages, cancel, on_delta)
                                  ├─ builtin: engine::ensure_running() → http://127.0.0.1:<port>/v1
                                  ├─ adapter (openai-compatible | anthropic | gemini): builds the request (pure)
                                  ├─ net::stream(), the one network chokepoint:
                                  │    local-only guard (PRI-1.1) · no redirects · no proxy for local hosts
                                  │    connect / first-reply / pause timeouts (LLM-1.5) · cancel checked every 25 ms (LLM-1.3)
                                  │    SSE framing → adapter.parse() → Delta | Done | Error (LLM-1.2)
                                  └─ HTTP status + body → ErrorKind (LLM-1.4)
Models tab / Setup ─→ commands: probe, list models, test connection, keys (keychain only), engine status
```
- **Most of it is pure and is tested without a network:** request bodies, stream parsing (against fixtures), error classification, locality, the vision lookup, the RAM suggestion and the engine arguments.
- **The network parts are tested against a tiny fake HTTP server on 127.0.0.1**, started inside the test. These are cancel, timeouts, the health check, Test connection and local-only.
- **Spec 03's `Model` trait stays as it is.** Spec 04 adds one implementation, `ProviderModel`, and `Message` gains images.

## Components

| File | Job |
|---|---|
| `src-tauri/src/llm/mod.rs` | Types (`Provider`, `Kind`, `ModelRef`, `Event`, `Capabilities`). `active(app)`, `ProviderModel` (implements `chat::Model`), `capabilities()` (LLM-5.3), `locality()` (LLM-8), `availability()` and `health()` (LLM-9.1). |
| `src-tauri/src/llm/net.rs` | **Every request to a provider goes through here.** It holds the local-only guard, the redirect and proxy rules, timeouts, cancel, SSE framing, and `classify()` (LLM-1.4). |
| `src-tauri/src/llm/openai.rs` | OpenAI-compatible: OpenAI, OpenRouter, Groq, DeepSeek, Mistral, LM Studio, llama.cpp, any custom server, and the built-in engine. Builds the request body, parses the stream, parses the model list. |
| `src-tauri/src/llm/anthropic.rs`, `gemini.rs` | The same three jobs for each of those APIs. |
| `src-tauri/src/llm/engine.rs` | The built-in llama-server: start, health wait, idle stop, stop on quit, crash reason (LLM-3). |
| `src-tauri/src/llm/local.rs` | The LM Studio probe (LLM-2.1), total RAM and the size suggestion (LLM-2.4). |
| `src-tauri/src/llm/vision.rs` | The table of known vision model names (LLM-5.2 step 3). |
| `src-tauri/src/llm/commands.rs` | The Models tab and Setup commands. |
| `src-tauri/src/llm/fixtures/` | Recorded or documented stream bodies and error bodies for the tests. |
| `src-tauri/src/secrets.rs` | The keychain: set, has, hint, delete (LLM-6). |
| `src-tauri/src/settings.rs` | The new `models` section. |
| `src-tauri/src/chat.rs` | Uses `llm::active`; `Message.images`; `Conversation.unavailable`; the too-long check. |
| `src/providers.ts` | The presets (LLM-4.1) and small pure helpers for the Models UI. |
| `src/Models.tsx` | The Settings → Models tab, plus the "add a provider" form that Setup also uses. |
| `src/Setup.tsx` | The new first step, "AI source" (APP-6.1 step 1). |
| `src/Chat.tsx` | The model switcher in the header (LLM-5.5); "No AI model is available" now shows the reason. |
| `scripts/fetch-llama.mjs` | Downloads the pinned llama.cpp build for the current OS, checks its SHA-256 and unpacks llama-server into `src-tauri/llama/`. |
| `src-tauri/tauri.llama.conf.json` | Adds `llama/` to the bundle's resources; used by the release build only. |

## Data

### Settings (`models`)
```jsonc
"models": {
  "providers": [{
    "id": "p-1760270000123",            // made when the provider is added; also its keychain account (LLM-6.1)
    "name": "OpenRouter",
    "kind": "openai-compatible",        // openai-compatible | anthropic | gemini | builtin
    "preset": "openrouter",             // the preset that filled it in (LLM-4.1), or "custom"
    "baseUrl": "https://openrouter.ai/api/v1",
    "models": ["meta-llama/llama-3.3-70b-instruct"],  // the models the user chose to use
    "contextSize": null,                // the engine's -c (default 8192); for other kinds, an override
    "visionOverride": {},               // { "<model>": true|false }, LLM-5.2 step 1
    "engine": null                      // builtin only: { "gguf": path, "mmproj": path|null, "lifetime": "always" | "idle" | "answering", "idleMinutes": 10 }
  }],
  "default": { "provider": "p-1760270000123", "model": "meta-llama/llama-3.3-70b-instruct" },  // LLM-5.4
  "trustedHosts": ["192.168.1.20"],     // LLM-8.2
  "timeouts": { "connectSecs": 10, "firstReplyLocalSecs": 120, "firstReplyCloudSecs": 60, "pauseSecs": 60 }  // LLM-1.5
}
```
- **No key is ever stored here.** Field names also avoid "key" and "token", because the APP-4.4 test rejects them. That test now walks a settings value with one provider of every kind. Until now it only walked the defaults, whose provider list is empty, so it checked none of these fields.
- **Removing a provider deletes its key.** `update_settings` deletes the keychain entry of every provider that disappeared, whichever screen removed it.
- The `models` list holds the models the user chose to use. They fill the default-model picker and the bubble's switcher.

### What the webview gets (LLM-6.2)
```rust
struct ProviderView { provider: Provider, locality: Locality, has_key: bool, key_hint: Option<String> /* the last 4 characters */ }
```

### Rust types
```rust
pub enum Kind { OpenAiCompatible, Anthropic, Gemini, Builtin }
pub struct ModelRef { pub provider: String, pub model: String }
pub struct Image { pub mime: String, pub base64: String }         // Message gains `images: Vec<Image>` (LLM-1.1)
pub enum Event { Delta(String), Done(Option<Usage>), Error(ChatError) }  // LLM-1.2
pub struct Usage { pub input: u32, pub output: u32 }              // only logged, as metadata
pub struct Capabilities { pub vision: bool, pub context_size: Option<u32> }  // LLM-5.3

pub fn active(app: &AppHandle) -> Result<Arc<dyn Model>, String /* why there is none */>;
pub fn capabilities(app: &AppHandle, m: &ModelRef) -> Capabilities;  // read by specs 06 and 10
```
`ProviderModel::stream` keeps spec 03's signature: it is synchronous and runs on the chat worker thread. Inside, it runs the async request with `tauri::async_runtime::block_on`.

## Commands and events

| Name | Kind | Job |
|---|---|---|
| `providers_list()` | command | Every provider as a `ProviderView`. |
| `key_set(provider, key)`, `key_delete(provider)` | command | Keys go from the UI to Rust only, never back (LLM-6.2). |
| `models_list(provider)` | command | The provider's model list, with context size and vision where known (LLM-5.1). |
| `provider_test(provider)` | command | Test connection: the model list, with a 15 s limit. Returns the model count or the error kind (LLM-7.1). |
| `runtimes_probe()` | command | LM Studio on localhost, with its models (LLM-2.1). |
| `ram_suggestion()` | command | Total RAM and the suggested model size (LLM-2.4). |
| `engine_status()` | command | `stopped`, `starting`, `running`, or `failed` with the reason (LLM-3.6). |
| `enabled_models()` | command | The chosen models of every provider, with locality and whether local-only disables them. Used by the switcher. |
| `set_model(provider, model)` | command | Makes this the default and updates the calling window's conversation (LLM-5.5). |
| `engine-status` | event | Same shape as `engine_status()`. |
| `chat-load` | event (spec 03) | Also sent when a failed health check marks the bubble's conversation unavailable. |

Providers themselves are added, edited and removed through the existing `set_settings`, like every other setting.

## Key mechanics

### The adapters (LLM-1.1)

| Kind | Chat request | Stream | Model list | Key |
|---|---|---|---|---|
| openai-compatible | `POST {base}/chat/completions`, `stream: true`. The system prompt is the first message. Images are `image_url` data URLs. | SSE: `choices[0].delta.content`, then `[DONE]` | `GET {base}/models` | `Authorization: Bearer`, only if a key is set |
| anthropic | `POST {base}/v1/messages`, `stream: true`. The system prompt is the top-level `system` field. `max_tokens` is 4096. Images are base64 `image` blocks. | SSE: `content_block_delta`, `message_delta`, `message_stop`, `error` | `GET {base}/v1/models` | `x-api-key` and `anthropic-version: 2023-06-01` |
| gemini | `POST {base}/v1beta/models/{model}:streamGenerateContent?alt=sse`. The system prompt is `systemInstruction`. Roles are `user` and `model`. Images are `inlineData`. | SSE: `candidates[0].content.parts[].text` | `GET {base}/v1beta/models`, keeping only models that support `generateContent` | `x-goog-api-key` |
| builtin | The openai-compatible adapter, pointed at the engine's port. | | | none |

- **Thinking text** (`reasoning_content`, or Anthropic thinking blocks) is not shown. It still counts as activity for the timeouts, so a model that thinks for a long time doesn't time out.
- **Anthropic needs `max_tokens`.** `// shortcut: fixed 4096 answer limit for Anthropic; make it a setting if long answers get cut off.`
- **An error that arrives inside a successful stream** goes through the same classifier. Examples: OpenRouter's `{"error": …}`, Anthropic's `event: error`.

### Stream framing and fixtures (LLM-1.2)
- **SSE** (`net::sse`) turns bytes into events:
  - it joins `data:` lines and handles `\r\n`
  - it skips comment lines such as OpenRouter's `: keep-alive`
  - it copes with a chunk that ends in the middle of a line or of a character
- **Fixtures** are in `src-tauri/src/llm/fixtures/`. Each kind has a normal answer and an error that arrives mid-stream. A test feeds each fixture in random chunk sizes, down to 1 byte, and expects the same event list every time.
  - The built-in engine fixtures are recorded on this PC.
  - The cloud fixtures are written from each API's documented stream examples. They are replaced by recordings once a key is available.

### Errors (LLM-1.4)
`classify(status: Option<u16>, body: &str) -> ErrorKind` is pure:

| Signal | Kind |
|---|---|
| 401, 403 | `auth` |
| 429 | `rate_limit` |
| 404, or a body saying the model is not found or does not exist | `model_not_found` |
| 400 or 413 whose body mentions the context length, "too long", "maximum context", or "exceeds the available context" | `context_too_long` |
| 5xx, or Anthropic's 529 "overloaded" | `server` |
| no connection, DNS failure, TLS failure, timeout | `network` |
| refused by the local-only guard | `blocked_local_only` |
| anything else | `unknown` |

The user sees the provider's own message when it has one. The log gets only the kind and the status code (APP-7.2).

### Cancel and timeouts (LLM-1.3, 1.5)
- **HTTP:** reqwest (async) on Tauri's existing tokio runtime. The chat worker calls `block_on`, so the synchronous `Model` trait stays.
- **The read loop** waits on three things at once (`select!`): the next chunk, the timeout, and a 25 ms tick that checks the cancel flag.
  - On cancel it returns at once and drops the connection.
  - llama-server stops generating when the client disconnects, so Stop also frees the local model.
- **Timeouts:**
  - connect: 10 s
  - first reply: from sending the request to the first stream event; 120 s for local providers, 60 s for cloud
  - pause: 60 s between events
- A timeout is a `network` error: "The model didn't reply within 60 s."
- All four timeouts are editable under Settings → Models → Advanced.

### The network chokepoint (PRI-1.1, constitution §3)
All provider traffic goes through `net.rs`, which:
1. **Refuses blocked hosts.** It classifies the host of the URL being requested (LLM-8). While local-only is on, it refuses any host that is neither local nor a trusted LAN host, with `blocked_local_only`. The check uses the URL actually requested, not the provider's label.
2. **Follows no redirects**, so a local server can't bounce a request to the internet.
3. **Uses no proxy for local and trusted hosts.** Otherwise a system proxy would carry "local" traffic off the machine.
4. **Adds the key at the last moment.** It reads the key from the keychain right before sending and never logs it.

PRI-1.1 belongs to spec 07, but it is built here. Local-only has to hold as soon as cloud providers exist (constitution §3). The rest of spec 07 stays in spec 07.

### Local vs cloud (LLM-8)
`locality(url, kind, trusted_hosts) -> Locality`:
- **Local:** the `builtin` kind, `localhost`, `127.0.0.0/8` or `::1`.
- **LAN:** a host in `trustedHosts` that is also a private address: `10/8`, `172.16/12`, `192.168/16`, `fe80::/10` or `*.local`.
- **Cloud:** everything else, including private hosts that aren't trusted. A public host in `trustedHosts` still counts as cloud.

Hosts are compared exactly as typed, with no DNS lookup, so `localhost.example.com` counts as cloud.

**Asking to trust a LAN host (LLM-8.3):** when a provider is added on a private host, Setup and the Models tab ask: "Trust 192.168.1.20? Requests to it will count as local." Trusted hosts are listed in the Models tab, each with Remove.

### Keys (LLM-6)
- **Storage:** keyring 4, service `cursora`, account = the provider id. That is Windows Credential Manager on Windows, and the Secret Service over D-Bus on Linux.
- **One-way:** the UI sends a key with `key_set`. `providers_list` sends back only `hasKey` and the last 4 characters.
- **Read per request:** the key is read from the keychain for each request and is not kept in memory.
- **No keychain (LLM-6.3):** `key_set` fails with an explanation: "Cursora keeps API keys in the system keyring, and none is available. Install and unlock GNOME Keyring or KWallet, then try again." The error itself goes to the log, not the message (issue 19). There is no plaintext fallback. Local providers, which need no key, still work.

### Models and capabilities (LLM-5)
- **Listing:** each kind uses its list call from the adapter table. If listing fails, the Models tab shows the error and a field to type a model ID (LLM-5.1).
- **Chosen models:** the user ticks the models to use. Those go into the provider's `models` list.
- **Context size, in order:**
  1. what the model list reports (OpenRouter `context_length`, Groq `context_window`, Mistral `max_context_length`, Gemini `inputTokenLimit`)
  2. the provider's `contextSize`
  3. otherwise unknown. Trimming then assumes 8192 tokens for local models and 32768 for cloud models. No current cloud chat model has less.

  The engine always has a known size: the one it is started with (`contextSize`, default 8192).
- **Vision (LLM-5.2), in order:**
  1. `visionOverride`
  2. what the runtime reports: an `mmproj` file for the engine
  3. the name table in `vision.rs`: gpt-4o, gpt-4.1, gpt-5, o3, o4, claude-3, claude-*-4, gemini, llava, bakllava, llama3.2-vision, llama4, qwen*-vl, gemma3 (except `:1b`), pixtral, mistral-small3.1 and later, minicpm-v, moondream, granite3.2-vision
  4. otherwise `false`

  `// shortcut: the name table goes stale; the override covers new models.`

### Default model and the switcher (LLM-5.4, 5.5)
- **Default model:** Settings → Models lists every ticked model in a "Default model" picker.
- **Switcher:** in the bubble header, the model name becomes a native `<select>`, grouped by provider.
  - In local-only mode, cloud models are disabled and marked "(local-only)".
  - Picking a model makes it the default. The header updates at once, and the next question uses the new model.

### No working model (LLM-9.1)
- **Availability:** `availability(settings)` is pure. It returns the model to use, or why there is none:
  - no default model is set
  - the default's provider was removed
  - local-only is on and the default is a cloud model
- **Health check:** the provider's model list, with a 3 s limit. For the built-in engine, it checks that the engine files and the `.gguf` exist and that the engine hasn't failed. It does not start the engine.
- **When it runs:** at the keypress, alongside the capture. A pass is reused for 60 s. A failure is never reused, so the next Ctrl+G checks again and works as soon as the model is back, without a restart.
- **What the user sees (tweak 2):**
  - **No model set up at all:** nothing is captured, so the user's app and clipboard are left alone. The bubble opens straight away with "No AI model is available", the reason and **Open setup**.
  - **The model doesn't answer:** the common local failures are known before the bubble shows: the runtime isn't running (refused at once), the model file is missing, or no key is stored. The bubble then opens with the message directly. Only a slow cloud check can finish after the bubble is open; the input is shown meanwhile and replaced if the check fails.
  - **Cursora keeps working normally:** the hotkey still opens and closes the bubble, focus goes back to the app, the capture still restores the clipboard, and the tray and Settings still work. There is no error dialog.

### Requests too long to send (changes spec 03)
After trimming, the system prompt, the context and the latest question can still exceed a **known** context size. When they do, the turn fails at once with `context_too_long`: "This is about 12,400 tokens; the model takes 8,192. Select less, or raise the context size in Models settings."
- **Why this changes spec 03:** spec 03 sent such requests anyway, but some local servers then cut them silently (LM Studio, or Ollama added as an OpenAI-compatible server). That can drop the start of the selection, or the system prompt's instruction to treat the selection as data.
- **When the context size is unknown,** the request goes out and the provider decides.

### Local runtimes (LLM-2)
- **Probe (LLM-2.1):** `GET http://localhost:1234/v1/models`, with a 1.5 s limit. It runs when the Models tab or Setup opens and returns LM Studio's base URL and models. **Add** creates a provider from the LM Studio preset.
- **Custom server (LLM-2.2):** a base URL and an optional key. `normalize_base_url` trims spaces and a trailing `/`, and removes a pasted `/chat/completions`.
- **RAM (LLM-2.4):** total RAM comes from `GlobalMemoryStatusEx` on Windows and `/proc/meminfo` on Linux, then the requirements table picks the size. It is shown in Setup and next to the engine's file picker.
- **Ollama** (detection, pulling models, its own API) is in the [backlog](../backlog.md). Until then, it can be added as an OpenAI-compatible server (`http://localhost:11434/v1`). Its context size is then unknown to Cursora, and Ollama cuts long requests at its default window.

### The built-in engine (LLM-3)

**What ships:**
- llama.cpp's own CPU builds, at a pinned build number (`b11570` today):
  - Windows: `llama-b11570-bin-win-cpu-x64.zip`, 19.5 MB
  - Linux: `llama-b11570-bin-ubuntu-x64.tar.gz`, 17.7 MB
- Only llama-server and its libraries are kept, about 40 MB unpacked. That includes one CPU library per processor generation; llama.cpp picks the best one when it starts, so it is fast on new CPUs and still runs on old ones.
- Each installer grows by about 20 MB.

**How it gets into the installer:**
- `scripts/fetch-llama.mjs` downloads the archive for the current OS, checks its pinned SHA-256, and unpacks it into `src-tauri/llama/`. That folder is gitignored.
- The release job runs the script, then builds with `tauri.llama.conf.json`, which adds the folder to the bundle's resources.
- Normal builds and `cargo test` don't need it. To try the engine in dev, run `npm run fetch-llama` once.
- **Resources, not Tauri's `externalBin`:** llama-server is a small launcher plus about 20 libraries that must sit next to it, and `externalBin` copies a single file.
- **Linux:** checked in WSL, llama-server finds its libraries in its own folder (RUNPATH `$ORIGIN`). It needs `libgomp` and `libssl 3`, which Ubuntu 22.04 and later have.

**Running it (`engine.rs`):**
- **Start (LLM-3.2):**
  1. Pick a free port: bind `127.0.0.1:0`, read the port, release it.
  2. Run `llama-server -m <gguf> [--mmproj <file>] --host 127.0.0.1 --port <port> -c <contextSize> --no-webui`.
  3. Poll `GET /health` every 250 ms until it returns 200. It returns 503 while the model loads.

  Loading counts toward the first-reply timeout, and Stop works while the model loads. `// shortcut: free-port race; retry once with a new port if "address already in use" shows up.`
- **Local only (LLM-3.3):** the engine is bound to `127.0.0.1`. `engine_args()` is pure and tested.
- **No orphans:**
  - Windows: the engine starts with `CREATE_NO_WINDOW`, so no console window flashes. It runs in a Job Object that kills it whenever Cursora exits, crashes included.
  - Linux: `PR_SET_PDEATHSIG` does the same.

  Without this, a crash would leave several GB of model in memory.
- **When it runs (LLM-3.5):** `lifetime` decides, read from the current settings, so changing it never reloads the model.
  - `always` (the default): it starts with Cursora (and when the user switches to `always`) and only stops on Quit, at shutdown, or by hand: "Stop the model" in the tray menu and a **Stop** button next to the engine's state in the Models tab (`engine_stop`). The next question starts it again.
  - `idle`: it starts on the first question and stops after `idleMinutes` without one; a timer checks every minute.
  - `answering`: it starts for each question and stops a moment after the bubble and the expanded window are both closed, so expanding a conversation doesn't unload it.
  - **Never mid-answer:** each answer holds the engine (`engine::busy`) until it ends; the idle time counts from the end of the last answer.
  - Only one engine runs at a time. Asking another built-in model restarts it with that file; only the files and the context size count, not the lifetime.
  - Setup asks the lifetime question after the built-in engine passes its test; the Models tab has the same three choices.
- **Failures (LLM-3.6):** if the engine exits while starting or later, the reason is its exit code plus the last 20 lines of its output.
  - Those lines are kept in memory only and are not logged, since they can contain file paths.
  - The turn fails with `server`: "The built-in engine stopped: …". The Models tab shows the same reason through `engine_status`.
- **Vision (LLM-3.4):** an `mmproj` file marks the engine's model as vision-capable.

### Setup step "AI source" (APP-6.1 step 1)
This becomes the first step of setup and uses the same components as the Models tab:
- **Found on this computer:** LM Studio, if it is running, with its models and a **Use** button.
- **Built-in engine:** **Choose a .gguf file**, plus an optional `mmproj` file. It uses the dialog plugin, which Cursora already has.
- **Cloud:** a preset, the key and **Test connection**, then pick a model.
- **Other server:** a base URL and an optional key. A LAN host brings up the trust question.
- **A working model is required (tweak 3).** **Next** stays disabled until the chosen model passes Test connection. For the built-in engine, that means it started and its health check passed. There is no Skip.
  - Closing the setup window leaves setup unfinished, so it opens again at the next start.
  - Re-running setup when a model already works shows that model, with Next enabled.

### Logging (APP-7.2)
- **Logged for each request:** provider kind, model id, locality, the time to the first reply and in total, token usage counts, and the error kind and status.
- **Never logged:** keys, request or response bodies, or URLs with keys in them.

## Changes to spec 03 code
- **Model:** `chat::active_model()` becomes `llm::active(app)`. Echo stays in debug builds, but only with `CURSORA_ECHO=1`, so "No AI model is available" can be tried in dev too.
- **Images:** `Message` gains `images`, which stays empty until spec 10.
- **Unavailable model:** `Conversation` gains `unavailable: Option<String>`. The "No AI model is available" view shows it as the reason.
- **Too long:** `begin()` adds the check described above.
- **Header:** the model name becomes the switcher.

## Dependencies
- **reqwest 0.13**, with default features off and `native-tls`, `system-proxy` and `json` on.
  - Tauri already locks this version (for its mobile builds), but desktop builds compile it new, with hyper and the platform's TLS.
  - native-tls uses SChannel on Windows and OpenSSL on Linux. `libssl-dev` is already a CI dependency.
- **tokio 1** with `macros` and `time`. Tauri already compiles it, so this only adds features.
- **keyring 4.** Its default features give Windows Credential Manager and, on Linux, the Secret Service through zbus, which Tauri already compiles there. Task 1 checks its v4 API and its test store.
- **`windows` crate features:** `Win32_System_SystemInformation` (RAM) and `Win32_System_JobObjects`.
- **Linux:** `libc`, already compiled, for `prctl`.
- **No `sysinfo`:** total RAM is one OS call on each system.
- **npm:** nothing new.

## Decisions
1. **Native Ollama support is moved to the backlog (decided 2026-10-12).** Ollama can't be installed on the development PC, so it couldn't be tested here. The local options in v1 are the built-in llama-server, LM Studio and any OpenAI-compatible server.
   - *Rejected:* a native `ollama` kind now, without a way to test it.
2. **Async reqwest, with a 25 ms cancel tick.**
   - *Rejected:* the blocking `ureq`. A read that is waiting for the first reply can't be interrupted, so Stop could hang for up to 120 s while the local model keeps generating.
3. **native-tls.**
   - *Rejected:* reqwest's default (rustls with aws-lc-rs), which compiles a C crypto library and gives a bigger binary. Both options use the OS certificate store.
4. **One chokepoint, which checks the URL.** The local-only, redirect and proxy rules live in one place, where they can't be skipped by a new call site.
5. **The engine ships as resources, fetched when the release is built.**
   - *Rejected:* committing about 40 MB of binaries to git.
   - *Rejected:* building llama.cpp from source in CI. It's slow, and a single CPU target is either slow everywhere or crashes on older CPUs.
6. **The health check doesn't block the open (tweak 2).**
   - *Rejected:* waiting for the check. That breaks the 200 ms open whenever a cloud check takes 300 ms or more.
7. **Ticked models.**
   - *Rejected:* listing every model live in the switcher. OpenRouter has hundreds, and listing costs a request every time the switcher opens.
8. **The switcher is a native `<select>`.**
   - *Rejected:* a custom popover. It's more code for the same job.
9. **The presets live in TypeScript** (`src/providers.ts`). Only the UI uses them, and a saved provider carries its own kind and base URL.

## Requirement tweaks (approved, applied)
1. **Ollama (applied, decided 2026-10-12):** LLM-2.1 now probes LM Studio only, LLM-2.3 (Ollama pull) is retired, and LLM-1.2 and LLM-5.2 no longer mention Ollama. Native Ollama support is in the [backlog](../backlog.md).
2. **LLM-9.1:** the end changes to: "…THE bubble SHALL show 'No AI model is available', the reason and 'Open setup' instead of the chat input. The check SHALL NOT delay opening the bubble (BUB-2.1); if it hasn't finished when the bubble opens, the input is shown and replaced if the check fails. A passed check is reused for 60 s, a failed one never. WHEN no model is set up THE SYSTEM SHALL NOT capture. Everything else keeps working normally."
3. **APP-6.1 step 1:** adds "Setup SHALL NOT continue past this step until the chosen model passes Test connection (LLM-7.1)."

## Known limits
- **Cloud fixtures** come from documentation until real streams are recorded.
- **Token counts** are estimates (characters ÷ 4), as in spec 03.
- **The engine is CPU only** (LLM-3.1). Users with a GPU are better off with LM Studio, and Setup says so.
- **No Windows on ARM build of the engine.** Only x64 is bundled.
- **The engine's first answer waits for the model to load:** a few seconds for a small model, longer from a hard disk. The bubble shows "Thinking…" meanwhile.

## Test strategy
| Criterion | Test |
|---|---|
| LLM-1.1 | For each kind, the request body puts the system prompt in the right place, keeps user and assistant turns in order, turns streaming on, and carries an image in that kind's format. |
| LLM-1.2 | Each fixture, fed in random chunk sizes → the expected Delta…Done or Error sequence. |
| LLM-1.3 | A fake server sends one delta, then stalls. On cancel, the stream returns in under 100 ms and no delta comes after the cancel. |
| LLM-1.4 | A table of real status and body pairs from each provider → the expected kind. |
| LLM-1.5 | A fake server that never replies, or pauses mid-stream, with the timeouts set to 200 ms → a `network` error naming the timeout. The defaults are 10/120/60/60. |
| LLM-2.1 | The probe against a fake LM Studio server (port passed in) finds it with its models. With nothing listening, it finds nothing. |
| LLM-2.2 | Custom URL normalization. A custom provider's chat goes to `{base}/chat/completions`, with the bearer key only when one is set. |
| LLM-2.4 | `suggest_size` on each RAM boundary. |
| LLM-3.3 | `engine_args` contains `--host 127.0.0.1` and never `0.0.0.0`. |
| LLM-3.4 | A built-in model with an `mmproj` file → `capabilities().vision`. |
| LLM-4.1 | TS: every preset has a kind and a base URL, the names match the requirement's list, and Custom exists. |
| LLM-5.1 | Each kind's model-list parser on a recorded body. Typed model IDs are kept when listing fails. |
| LLM-5.2 | The override beats the engine's `mmproj`, which beats the table, which beats `false`. |
| LLM-5.3 | `capabilities()` for a built-in, a local-server and a cloud model. |
| LLM-6.1 | keyring's test store: set, get and delete under service `cursora`, account = provider id. Removing a provider deletes its key. |
| LLM-6.2 | `ProviderView` JSON has `hasKey` and the last 4 characters, and never the key. |
| LLM-7.1 | Test connection: a fake server that answers → OK; one that returns 401 → `auth`; one that stalls → a result within the limit (shortened in the test). |
| LLM-8.1 | The locality table: `localhost`, `127.x`, `::1`, `[::1]:port` and builtin → local; public hosts and `localhost.example.com` → cloud. |
| LLM-8.2 | A trusted private host → LAN; an untrusted private host → cloud; a "trusted" public host → still cloud. |
| LLM-9.1 | `availability` with no default, a removed provider, and a cloud default in local-only. The health check against a stalling server fails at its limit. A failed check isn't reused, and with no model set up the hotkey doesn't capture. |
| PRI-1.1 | With local-only on, a cloud URL is refused with `blocked_local_only` and the fake server sees no connection. Local and trusted LAN hosts pass. |
| APP-4.4 | Extended: settings with one provider of each kind have no field names that look like secrets. |
| Manual | LLM-3.1, 3.2, 3.5, 3.6, 4.2, 5.4, 5.5, 6.3 and 8.3, plus the spec 03 checks that need a real model (BUB-4.3, 4.5, 6.1 Regenerate). |

## Changes during implementation
- **2026-10-12, task 1:**
  - **Keychain:** `keyring-core` with `windows-native-keyring-store` (Windows) and `zbus-secret-service-keyring-store` with OpenSSL crypto (Linux), instead of the `keyring` crate. keyring 4's own documentation says apps should use these directly; it also gives the tests keyring-core's mock store. The store is picked on first use and again after a failure, so a keyring started later works without restarting Cursora (LLM-6.3).
  - reqwest with `native-tls` builds on Windows and in WSL; Linux links the system OpenSSL.
  - The llama.cpp build is pinned at `b11570`, with the SHA-256 values GitHub publishes for its two CPU archives.
- **2026-10-12, build:**
  - **Lighter dev builds:** `crate-type` is now only `rlib`, and dev builds keep line tables only (`[profile.dev] debug = "line-tables-only"`). The `staticlib` and `cdylib` outputs exist for iOS and Android, and full debug info for every dependency made the PDBs huge; on this 8 GB PC the link step stalled for 15 minutes at a time. Release builds are unchanged.
  - **Two small commands were added:** `lan_host(url)` tells the UI when to ask the trust question (LLM-8.3) without a second copy of the private-address rules in TypeScript, and `pick_model_file()` opens the `.gguf` picker through the dialog plugin Cursora already has, so no new npm package or permission is needed.
  - **The health check doubles as the context-size source:** each model-list call remembers the sizes it reports, in memory, so a model's context size is known after the first hotkey press.
  - **`[DONE]` is passed to the OpenAI adapter** rather than swallowed by the SSE framing, so every adapter decides for itself whether an answer ended normally. A stream that closes without an ending is an error: "The connection closed before the answer was complete."
  - **The engine is started on the main thread.** On Linux, `PR_SET_PDEATHSIG` fires when the *thread* that started the child exits, and chat worker threads end after every answer.
  - **Settings edits in the Models UI start from the latest saved settings** (`editModels`), so adding a provider and then choosing its model can't undo each other.
- **2026-10-12, end-to-end run:**
  - **A dropped stream waits up to a second for the engine's exit** (`engine::exited_within`) before choosing its message. The connection drops a moment before Windows reports the process gone, so a killed engine showed "Couldn't reach the provider" instead of "The built-in engine stopped: …" (LLM-3.6).
  - **The bubble shows the error's own words** under the short message (all kinds except `unknown`, which already shows them). Without it the engine's reason never reached the bubble, as LLM-3.6 requires.
  - **`settings.json` with a byte order mark loads.** Windows PowerShell and older Notepad add one; Cursora took the file as broken and fell back to defaults (APP-4.2).
  - **The `llama-server` fixture is a real recording** from b11570 with a 0.5B model, so the `builtin` stream test parses what the engine really sends.
  - **`fetch-llama` adds llama.cpp's MIT `LICENSE`** from the pinned tag; the Windows archive doesn't include it. It uses Windows' own `tar.exe`, since Git's GNU tar reads `C:` as a host name.
  - **Setup shows the current model whenever one is set**, also after a new source was added, so the user sees which model the test checks.
- **2026-10-12, review:**
  - **A missing or locked keyring stops only cloud providers** (LLM-6.3 now says so). Every request read the keychain first, so on Linux without a Secret Service even LM Studio and the built-in engine failed. Local and trusted LAN providers now go on without a key, and the built-in engine never reads the keychain.
  - **The engine check after a dropped stream also covers a clean close** ("The connection closed before the answer was complete"), which is what Linux usually reports when the engine is killed, and runs only for network errors, so a too-long request no longer waits a second for nothing.
  - **The Models tab shows a failed engine's reason as lines**, in the same small scrolling block as the bubble.
  - **Preload failures log only the error kind:** the reason can hold the engine's output, which stays in memory (APP-7.2).
- **2026-10-13, manual checks:**
  - **A provider's name and base URL are editable in its card** (LLM-4.2). The card only showed them, so a server that moved meant removing and adding the provider again.
  - **Working offline (LLM-8.4, added):** nothing on the local path needs the internet. The UI's content security policy allows only bundled files and IPC, and Rust connects only to the base URL of the provider in use (the built-in engine is on 127.0.0.1). The end-to-end run records every connection Cursora, its webview and the engine make to another computer: Cursora and the engine made none. The WebView2 runtime made its own (Microsoft services, and DNS-over-HTTPS probes to the PC's DNS servers), so every window now starts it with `BROWSER_ARGS`: wry's defaults plus background networking, component updates, domain reliability and pings off. The DNS-over-HTTPS probe remains, since no startup argument turns it off; it carries no Cursora data. Checked with Wi-Fi off: everything works.
  - **How long the model stays loaded is the user's choice (LLM-3.5, changed 2026-10-13):** `keepLoaded` became `lifetime`: "always" (the default; only Quit, shutdown or "Stop the model" unload it), "idle" (after `idleMinutes`) or "answering" (unloads once no conversation is on screen). Setup asks after the built-in engine passes its test. The engine restarts only for other files or another context size, and its stop rule reads the current settings.
  - **An answer holds the engine** (`engine::busy`), so the idle timer can't stop it mid-answer. Before, the timer counted from the start of a question, so an answer longer than the idle time lost its engine.
