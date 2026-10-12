# 03: Bubble UI (design)

Status: **Approved** (2026-10-12). Implements [requirements.md](requirements.md). This covers Windows; Linux placement and focus are spec 08.

## Overview
```
Ask hotkey (main thread)
  ├─ bubble visible → close: cancel any stream (BUB-2.6), hide, give focus back to the app (BUB-2.3)
  └─ else: cursor = app.cursor_position()            ← read at the keypress, before the capture moves anything
           capture thread (spec 02) → bubble::open(app, cursor, capture)
             1. new Conversation { capture, turns: [] } in ChatState
             2. place(): monitor under the cursor, flip, clamp → set position + size
             3. emit "bubble-open" { conversation, status } → show → activate
Bubble (React)                             Rust
  Enter → invoke ask(question)    ──→   chat::ask: push Turn, spawn worker
                                         worker: messages = assemble(conv, budget)
                                                 model.stream(messages, cancel, on_delta)
  "chat-delta" { id, offset, text } ←──    each delta (sent as soon as it arrives, §8)
  "chat-done"  { id, status, error?, dropped } ←── at the end; history saved (BUB-9.1)
```
- **Rust owns the conversation; React only draws it.** This lets Expand (BUB-2.5) and History (BUB-9.2) show the same conversation in a second window without copying state between webviews.
- **Pure logic is tested in isolation:**
  - placement, message assembly, budget trimming, the turn worker and the history store (Rust)
  - Markdown, keys, error texts and the header (TypeScript)
- The model is reached through a small `Model` trait. **Spec 04 fills it in.** Until then:
  - release builds have no model, so the bubble shows "No AI model is available" (LLM-9.1 wording)
  - debug builds have an `Echo` model, so streaming, Stop, Expand and history can be tested now

## Components

| File | Job |
|---|---|
| `src-tauri/src/bubble/mod.rs` | Creates the hidden bubble window at startup (BUB-2.1). Also: `open`, `close`, `place()` (pure, BUB-1.1–1.4), and the commands `bubble_resize` and `expand`. |
| `src-tauri/src/bubble/windows.rs` | Windows-only helpers: hide from Alt+Tab (`WS_EX_TOOLWINDOW`); `activate(hwnd)`; read the foreground window. |
| `src-tauri/src/bubble/other.rs` | No-op versions of the same helpers for non-Windows builds (spec 08). |
| `src-tauri/src/chat.rs` | `Conversation`, `Turn`, the `Model` trait, `Echo` (debug only), `assemble()` (BUB-5.1, 5.2), the turn worker (BUB-4.1, 2.6), and the commands `ask`, `stop`, `regenerate`, `remove_context`, `get_conversation`. |
| `src-tauri/src/prompt.rs` | The system line and the delimited context block (constitution §5). Spec 05 extends this with personas. |
| `src-tauri/src/history.rs` | The SQLite store: save, list/search, get, delete, clear, prune (BUB-9). Also its commands. |
| `src-tauri/src/settings.rs` | New `history` section. |
| `src-tauri/src/capture/mod.rs` | `SourceApp` gains `window` (the foreground window handle, never sent to the UI) so focus can go back to it. |
| `src-tauri/src/hotkey.rs` | `on_ask` becomes the open/close toggle and records the cursor position. |
| `src/Chat.tsx` | The conversation view, used by both the bubble and the expanded window: header, context, turns, input, actions. |
| `src/chat.ts` | Pure helpers: `keyAction`, `errorText`, `header`, the context notices. |
| `src/markdown.ts` | One markdown-it instance with raw HTML and images off, plus highlight.js. |
| `src/History.tsx` | The Settings → History tab (BUB-9.2) and the history settings. |

### Windows
| Label | Created | Look | Purpose |
|---|---|---|---|
| `bubble` | at startup, hidden | frameless, Windows 11 rounded corners via `shadow(true)`, always on top, `skip_taskbar` + `WS_EX_TOOLWINDOW` (no Alt+Tab), not resizable, 380 px wide | the bubble |
| `chat` | on Expand or History → open; destroyed on close | normal, resizable, 720×640 | the expanded window (BUB-2.5) |

Both windows get a new capability, `capabilities/bubble.json`, which grants `core:default`, `core:window:allow-start-dragging` (the header drag region, BUB-2.4) and `opener:allow-open-url` limited to `http(s)` and `mailto` (links, BUB-4.2). A close request for the bubble (for example Alt+F4) is ignored, because only the hotkey closes it (BUB-2.3).

### The model seam (filled by spec 04)
```rust
pub struct ModelInfo { pub name: String, pub provider: String, pub locality: Locality /* local|lan|cloud */, pub context_tokens: Option<u32> }
pub struct Message { pub role: Role /* system|user|assistant */, pub text: String }
pub struct ChatError { pub kind: ErrorKind /* LLM-1.4 kinds, snake_case */, pub message: String }

pub trait Model: Send + Sync {
    fn info(&self) -> ModelInfo;
    /// Calls on_delta for each piece of text as it arrives; returns when done, failed or cancelled.
    fn stream(&self, messages: &[Message], cancel: &AtomicBool, on_delta: &mut dyn FnMut(&str)) -> Result<(), ChatError>;
}
pub fn active_model(app: &AppHandle) -> Option<Arc<dyn Model>>;   // spec 03: Echo in debug builds, None in release
```
The trait is synchronous, and each request runs on its own worker thread, like the capture. Spec 04 may change the signature; this is only the seam.

### Data (sent to the UI as JSON)
```rust
struct Conversation { id: i64 /* unix ms at open */, started: i64, app: SourceApp, content: Content, notice: Option<Notice>,
                      model: Option<ModelInfo>, persona: Option<String>, turns: Vec<Turn>, dropped: usize }
struct Turn { question: String, answer: String, status: TurnStatus /* streaming|done|cancelled|error */,
              error: Option<ChatError>, asked: i64, answered: Option<i64> }
```
`ChatState` holds two slots, `bubble` and `chat`, each an `Option<Conversation>` with its cancel flag. A command acts on the slot named by the calling window's label.
- **Expand** moves the bubble's conversation into `chat` and hides the bubble **without** cancelling.
- **History → open** loads a saved conversation into `chat`, where it can be continued.

### Commands and events
| Name | Kind | Job |
|---|---|---|
| `ask(question)` | command | Adds a turn and starts the worker. Refused while a stream is running or when there is no model. |
| `stop()` | command | Sets the cancel flag (BUB-4.3). |
| `regenerate()` | command | Re-runs the latest question; also used by Retry (BUB-4.4). |
| `remove_context()` | command | Drops the captured context, only before the first question (BUB-3.2). |
| `get_conversation()` | command | The calling window's conversation; the expanded window calls it on load. |
| `bubble_resize(height)` | command | The bubble reports its content height (logical px); Rust caps it at 520 and re-places it (BUB-2.5). |
| `expand()` | command | Opens the `chat` window for the bubble's conversation. |
| `open_settings(tab)` | command | Opens Settings on a tab ("Open Models settings", BUB-4.4). |
| `history_list(query, offset)`, `history_open(id)`, `history_delete(id)`, `history_clear()` | commands | The History tab. |
| `bubble-open` | event → `bubble` | `{ conversation, status }`. The view resets and focuses the input. |
| `chat-delta` | event → all | `{ id, offset, text }`. A window applies it only for its own conversation id, and only if `offset` equals its current answer length. Otherwise it calls `get_conversation`, so nothing is lost while a window is starting up. |
| `chat-done` | event → all | `{ id, status, error?, dropped }`. |

### Settings added (`history`)
| Field | Default | Criterion |
|---|---|---|
| `save` | `true` | BUB-9.1, 9.3 |
| `retentionDays` | `90` (`null` = forever) | BUB-9.4 |

## Key mechanics

### Placement (`place`, pure, physical pixels)
1. Read the cursor at the keypress, then the monitor that contains it (`monitor_from_point`), with its work area and scale factor (BUB-1.3).
2. Width = 380 × scale. The **flip decision uses the maximum height** (520 × scale):
   - flip left if `cursor.x + 16 + width` passes the right edge
   - flip up if `cursor.y + 16 + maxHeight` passes the bottom edge

   The decision is made once, when the bubble opens, so the bubble never jumps sides while an answer grows it (BUB-1.2).
3. Position:
   - not flipped: `cursor + (16, 16)` (BUB-1.1)
   - flipped left: the right edge sits at `cursor.x − 16`
   - flipped up: the bottom edge sits at `cursor.y − 16`, and the bubble grows upward
4. Clamp the whole rectangle into the work area.
5. With no cursor (Wayland, BUB-1.4): center on the active monitor, or else the primary one.

`bubble_resize` re-runs steps 3–4 with the stored cursor, monitor and flip, and the new height. Windows rescales a window that moves to a monitor with a different scale factor. So the order is: set the position, then the size in physical pixels, then the position again.

### Opening and focus
- **The bubble is created hidden at startup**, so opening it is just position, emit and show (well under the 200 ms budget). See decision 1 for the memory cost.
- **Focus (BUB-2.2):** `bubble/windows.rs::activate` calls `SetForegroundWindow`. If Windows refuses, it taps **F24**, a key no app uses, and tries again. We don't use Tauri's `set_focus`: its fallback taps **Alt**, and the app in front treats that as a menu-bar press (constitution §6). The input gets focus when `bubble-open` arrives.
- **Closing (BUB-2.3):** if the bubble is the foreground window, `activate(capture.app.window)` returns focus to the app that was in front. Otherwise focus is left where the user put it.
- When it hides, the view clears itself. The next show then can't flash the previous conversation, since every open is a new conversation anyway (BUB-5.3).

### Asking, streaming, cancelling
- **Keys (`keyAction`, BUB-4.1):** Enter sends. Shift+Enter, or Enter while an IME is composing, inserts a newline.
- **Worker:** it calls `model.stream`. Each delta is appended to the turn in `ChatState` and emitted at once (§8, no buffering).
  - On finishing it sets `done`, `cancelled` or `error`, emits `chat-done`, and saves the conversation to history if saving is on.
- **Cancel (BUB-2.6, 4.3):** hotkey-close and Stop both set the turn's cancel flag. The worker stops forwarding deltas at once, even if the model doesn't stop yet. The partial answer stays, marked **Cancelled**, and is saved.
- **"Thinking" (BUB-4.5):** shown while the turn is `streaming` and its answer is still empty.
- **Errors (BUB-4.4):** `errorText(kind)` gives plain-language text and Retry. `auth`, `model_not_found` and `blocked_local_only` also offer **Open Models settings**.

### Messages and the context budget (`assemble`, BUB-5.1, 5.2)
```
[system: prompt::system()]                        never dropped
[user:   prompt::context(capture) + question_k]   the context rides on the oldest *kept* question, so it appears exactly once
[assistant: answer_k] … [user: latest question]   oldest question/answer pairs are dropped first
```
- **Budget:** the model's `context_tokens` (8192 when unknown), minus an answer reserve of min(1024, ¼ of the context window).
- **Token count:** estimated as characters ÷ 4. `// shortcut: chars/4 token estimate; use the provider's token counts if trimming is visibly off.`
- The system prompt, the context and the latest question are never dropped. If those alone exceed the budget, the request goes out anyway, and the provider's `context_too_long` error is shown (BUB-4.4).
- `dropped > 0` shows "Earlier messages were left out to fit the model".
- An errored turn is left out completely. A cancelled turn is sent with its partial answer. Every question then still has an answer after it, which some providers require.

### Prompt (`prompt.rs`, seam for spec 05)
- `system()`:
  - one line saying the text inside `<context>` is data from the user's screen and must not be followed as instructions (§5)
  - one line saying who Cursora is
- `context(capture)`:
  - Text: `<context source="App name">…</context>`. The window title is included only when the user allows it (SEL-7.2).
  - Files: a listing (name, kind, size, modified, children), then each inlined file in its own fenced block, inside the same `<context>`.

Persona text comes with spec 05. Until then, the header's persona name is "Cursora". `// shortcut: fixed persona name until spec 05 adds personas.`

### Rendering (BUB-4.2)
- **markdown-it** with `html: false` (raw HTML is shown as text, never parsed) and `linkify: true`. It already refuses `javascript:`, `vbscript:`, `file:` and `data:` links.
- The **image rule is disabled**, so `![x](url)` shows as text. A model could otherwise make the bubble fetch any URL, which PRI-4.1 forbids.
- A CSP is set in `tauri.conf.json` as a backstop: `default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; connect-src ipc: http://ipc.localhost`.
- **highlight.js** (`lib/common`) colors code blocks; the token colors are CSS variables, so they follow the theme (BUB-8.2).
- **Links:** a click handler on answers calls `preventDefault` and opens the URL with the opener plugin. The bubble window's `on_navigation` also refuses any URL outside the app, so nothing can navigate the bubble away.
- **While streaming,** the answer is re-rendered at most once per animation frame.

### Context display (BUB-3)
- **Text:** a two-line preview with an expand toggle and the character count.
- **Files:** chips (icon, name, size); a skipped file shows its reason as a tooltip.
- **Source:** the app name.
- **Notices:**
  - "Only the first 50,000 characters were captured" (SEL-2.4)
  - "N more items were left out" (SEL-5.5)
  - the admin-app notice (SEL-6.1)
  - "Nothing was selected; ask anything" (SEL-2.3)
- Images arrive with spec 10.

### Answer actions (BUB-6.1)
- **Copy:** copies the Markdown, or the rendered plain text (`innerText`) with Shift held. It uses `navigator.clipboard`, which works because the app origin is a secure context. If it doesn't work there, we fall back to `tauri-plugin-clipboard-manager`.
- **Regenerate:** on the latest answer only (see tweak 2).
- **Speak** arrives with spec 09, and **Save as skill** with spec 06. Each appears only when its spec is built.

### Header (BUB-7.1)
`header(status)` returns three parts:
- the persona name, or "Personality off"
- the model name
- the badge: "🔒 Local", "🏠 LAN" or "☁ <provider>", from `ModelInfo.locality`

With no model, the bubble shows **"No AI model is available"**, the reason, and **Open setup**, with no chat input (LLM-9.1). The 3 s health check itself is spec 04.

### History (BUB-9)
- One SQLite file, `<app data dir>/history.sqlite`, opened only when saving is on (BUB-9.3).
- Schema:
  ```sql
  conversations(id INTEGER PRIMARY KEY, started INTEGER, updated INTEGER, app TEXT, model TEXT, persona TEXT,
                content TEXT /* JSON */, turns TEXT /* JSON, with per-turn timestamps */, search TEXT /* lowercased text */)
  ```
  One row per conversation. A follow-up rewrites its row, so a reopened conversation keeps its id.
- **Search:** `search LIKE '%term%'`. `// shortcut: LIKE scan; switch to FTS5 if search gets slow on large histories.`
- **Prune (BUB-9.4):** `DELETE … WHERE updated < now − retention`, run at startup and after each save.
- **History tab:**
  - "Save history" toggle and retention (30 / 90 / 365 days / forever)
  - a newest-first list with search
  - Open (in the expanded window), Delete, and Clear all (with confirmation)

## Dependencies
- **Rust:** `rusqlite` 0.40 with `bundled`. It compiles SQLite in, so no system library is needed on either OS; the C compiler is already installed on Windows and in CI. Requirement BUB-9.1 names SQLite.
- **npm:** `markdown-it` 15 and `highlight.js` 11 (runtime); `vitest` 5 (dev). The constitution's CI runs vitest. `npm test` becomes `node --test … && vitest run`.

## Decisions

1. **The bubble is pre-created; constitution §8 was amended for it (decided 2026-10-12).** Measured on this PC:
   - Cursora with no window open uses **4 MB** private memory (32 MB working set).
   - With one webview window it uses **154 MB** private memory across 8 processes. That is WebView2 itself.

   So BUB-2.1 (create at startup) and the old §8 (idle ≤ 60 MB) couldn't both hold. You chose to keep the bubble always ready, and §8 now reads: Cursora's own process ≤ 60 MB, and ≤ 200 MB in total including the hidden bubble's WebView2 processes. Task 1 measures the hidden bubble and tries WebView2's low-memory mode while hidden; if the total comes out lower, we tighten the number.
   - *Rejected:* creating the bubble on the first hotkey press. That makes the first open of each session slow, roughly 0.5–1 s (an estimate).
   - *Rejected:* creating it on every open. Every open would take roughly 0.3–1 s, which breaks the 200 ms budget.
2. **Rust owns the conversation:** this is the simplest way for a second window (Expand, History) to show and continue it. *Rejected:* keeping it in React and passing it between windows, which needs a sync protocol.
3. **Events, not a per-request `Channel`:** both windows may need the deltas (Expand during a stream). The `offset` field makes a window that starts late catch up instead of losing text.
4. **Our own focus code:** Tauri's fallback taps Alt in the user's app (see Opening and focus).
5. **markdown-it over marked:** marked passes raw HTML through and needs a sanitizer (DOMPurify). markdown-it escapes HTML by itself.
6. **One history row per conversation:** turns are stored as JSON. Nothing queries single turns, so a turns table would be code without a user.
7. **Echo model in debug builds only:** it lets the streaming, Stop, cancel, Expand and history checks run before spec 04. Release builds never contain it.

## Requirement tweaks (approved, applied)
1. **BUB-7.2 moves to spec 04.** The quick model switcher needs configured models, which spec 04 creates. BUB-7.2 is retired and re-added as LLM-5.5.
2. **BUB-6.1 Regenerate is offered on the latest answer only.** Regenerating an older answer would leave the follow-ups after it answering a question that no longer exists.
3. **BUB-3.2 The context can be removed until the first question is sent.** After that, earlier answers depend on it.
4. **BUB-4.2 adds "Images in model output SHALL NOT be loaded; they show as text."** This follows from PRI-4.1 (no requests to hosts the user didn't configure).

## Known limits (documented, not fixed)
- **Windows 10** draws the bubble with square corners and a 1 px border. Rounded corners come from Windows 11.
- **The Echo model is for testing only.** The release build can't chat until spec 04.
- **`navigator.clipboard`** and the coordinate space of `monitor_from_point` are checked in task 1, before anything is built on them.

## Test strategy
| Criterion | Test |
|---|---|
| BUB-1.1 | `place`: plenty of room → `cursor + (16, 16)` |
| BUB-1.2 | Near the right and bottom edges and in each corner → flipped, then clamped inside the work area. Growing never changes the flip. |
| BUB-1.3 | Two monitors with scales 1.0 and 1.5 (one at negative coordinates) → uses the scale of the cursor's monitor |
| BUB-1.4 | No cursor → centered on the active monitor, else the primary one |
| BUB-2.6 | The worker with a fake model that streams until cancelled → partial answer kept, `cancelled`, saved |
| BUB-4.1 | `keyAction` (Enter / Shift+Enter / IME) in TS; the worker forwards deltas in order, one event per delta, in Rust |
| BUB-4.2 | `markdown.render`: `<script>` and `<img onerror>` escaped, `javascript:` not a link, images shown as text, code highlighted |
| BUB-4.4 | `errorText` has text for every LLM-1.4 kind; settings offered exactly for `auth`, `model_not_found` and `blocked_local_only` |
| BUB-5.1 | `assemble`: the context appears exactly once, on the oldest kept question, across several turns and after trimming |
| BUB-5.2 | Over budget → oldest pairs dropped, system/context/latest kept, `dropped` counted |
| BUB-7.1 | `header`: persona or "Personality off", model, and each badge |
| BUB-9.1 / 9.3 / 9.4 | `history` on a temp dir: save → list/search/get round-trip; with saving off no file is created; old rows pruned, "forever" keeps them |
| Manual | BUB-1.5, 2.1–2.5, 3.1–3.3, 4.3, 4.5, 5.3, 6.1, 8.1, 8.2, 9.2, plus the SEL items waiting on the bubble (SEL-1.4, 2.5, 4.5, 5.1, 6.1, 8.1) |

## Changes during implementation
- **2026-10-12, task 1:**
  - The hidden bubble measured about **170 MB** private memory across 8 processes (debug build, with the dev server).
  - WebView2's low-memory target saved only about 7 MB (165 MB). That isn't worth a new dependency plus unsafe code, so it isn't used.
  - `monitor_from_point` takes physical pixels (tao calls `MonitorFromPoint`), the same space as `cursor_position`, so placement stays in physical pixels throughout.
- **2026-10-12, build:**
  - The TypeScript helpers are in `src/conversation.ts`, not `chat.ts`. Windows treats `chat.ts` and `Chat.tsx` as the same file name.
  - History stores each conversation as one JSON `data` column, beside `title`, `app`, `model` and `search`. Search uses `instr` on a lowercased copy, so user input never needs LIKE escaping.
  - A conversation's `id` is the time it was opened, so it has no separate `started` field.
  - `tsconfig` `lib` is now ES2022, for `Array.at`.
  - The bundle grew from 242 KB to 505 KB (markdown-it plus highlight.js `common`). The hidden bubble loads it at startup, not when the hotkey is pressed.
- **2026-10-12, /ponytail-review fixes:**
  - **Async window commands:** every command that creates a window (`open_setup`, `open_settings`, `expand`, `history_open`) is now `#[tauri::command(async)]`. Creating a window inside a synchronous command deadlocks on Windows (wry#583). This also fixes spec 01's "Re-run setup".
  - **Mid-answer turns are saved:** when a conversation leaves its window during an answer (the expanded window closes, or it is replaced), the turn is marked Cancelled and saved there and then. Before this, its worker could no longer find it, so the turn never reached history.
  - **Expand:** it hides the bubble without handing focus back to the app, and moves the conversation under a single lock.
  - **Follow-ups:** they record the model that actually answers.
  - **Context escape:** it works in any letter case (`</CONTEXT>`).
- **2026-10-12, end-to-end run on Windows:**
  - The bubble's height now comes from CSS. Its content takes its natural height, up to `max-height: 520px`, and the window follows that one measured number. Before this, the header and context shrank to fit the window instead of growing it, so the bottom of the bubble was cut off near the screen corner.
  - `apply` offsets the position by the invisible frame Windows adds around a frameless window with a shadow: 9 px at 125 % scaling. The visible bubble now sits exactly at cursor + 16.
