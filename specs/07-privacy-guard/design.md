# 07: Privacy guard (design)

Status: **Approved** (2026-10-13). Implements [requirements.md](requirements.md). Spec 04 already built the network chokepoint (PRI-1.1). This spec adds the rest: switching to a local model, the app blocklist, redaction with its preview, and confirming before a cloud send.

## Overview
```
Ctrl+G ─→ capture(): foreground app ─→ blocklist rule matches? ── yes ─→ Notice::Blocked, nothing read (PRI-2.1)
                                                          └─ no ──→ the spec 02 capture chain, unchanged
ask / regenerate ─→ chat::request(): assemble() ─→ redact? (cloud, or "local too") ─→ messages + placeholder map
                     ├─ confirm on, cloud, not confirmed ─→ returns "confirm": the bubble opens "What will be sent" (PRI-3.7)
                     └─ stream ─→ Restorer: placeholders back to values, also across chunks (PRI-3.3) ─→ bubble, history
"What will be sent" ─→ send_preview(draft): the same chat::request(), so it shows the bytes that go out (PRI-3.6)
net.rs chokepoint ─→ local-only refuses cloud hosts (PRI-1.1, spec 04, unchanged)
```
- **Redaction happens where requests are built, not in the network layer.** Numbering values the same way across a conversation (PRI-3.1) needs the whole conversation, and putting values back (PRI-3.3) needs the map next to the stream. The network layer stays blind to content.
- **One request builder.** The ask, Retry, the skill form's Test, the developer prompt preview and the "What will be sent" panel all call `chat::request()`. So the preview can't drift from what is sent.

## Components

| File | Job |
|---|---|
| `src-tauri/src/privacy.rs` (new) | `PrivacySettings`, the detector table, `Redaction` (detect, number, apply), `Restorer` (stream), `validate()`, the confirm gate, `send_preview`, `privacy_detectors`. |
| `src-tauri/src/capture/mod.rs` | `Rule`, `blocked()`, `CaptureSettings.blocklist` with the PRI-2.3 defaults, `Notice::Blocked`, `identifies_apps()`. |
| `src-tauri/src/chat.rs` | `request()` (assemble + redact); `begin` restores the stream and passes the confirm gate; `ask` and `regenerate` take `confirmed`. |
| `src-tauri/src/prompt.rs` | `PLACEHOLDERS`: one line telling the model what the placeholders are, added only when something was hidden. |
| `src-tauri/src/llm/mod.rs` | `local_default()` (PRI-1.3); `availability()` says "No local model — set one up." |
| `src-tauri/src/lib.rs` | `update_settings`: `privacy::validate`, the PRI-1.3 switch; `app_info` gains `identifiesApps`; `model_problem`. |
| `src-tauri/src/tray.rs` | "Block Cursora in <app>"; the Local-only label and tooltip. |
| `src/Privacy.tsx` (new) | Settings → Privacy. |
| `src/Chat.tsx`, `src/conversation.ts` | The blocked notice, the Local-only badge, the "What will be sent" panel and the confirm flow. |

## Data

### Settings
```jsonc
{
  "localOnly": false,                       // unchanged (spec 04)
  "capture": {
    "blocklist": [ { "process": "KeePass.exe" }, { "title": "net banking" } ]   // PRI-2.2, 2.3
  },
  "privacy": {
    "redact": true,                         // PRI-3.1
    "redactLocal": false,                   // PRI-3.5
    "detectorsOff": ["phone"],              // PRI-3.4
    "patterns": [ { "label": "EMP", "regex": "EMP-\\d{6}" } ],   // PRI-3.4
    "confirmCloud": false                   // PRI-3.7
  }
}
```
- **The blocklist sits in `capture`**, next to `unsafeApps` and `copyShortcuts`: like them, it is a per-app rule for the capture. The Privacy tab edits it.
- `localOnly` stays where spec 04 put it, so nothing needs migrating.
- Older settings files get the defaults through `#[serde(default)]` (APP-4.3), so existing users get the default blocklist.
- **A custom pattern's label** is letters and digits, stored in capitals, since it becomes part of `[CUSTOM_<LABEL>_1]`.

### Rules (PRI-2.2)
`enum Rule { Process(String), Title(String) }`, written `{ "process": … }` or `{ "title": … }`.
- **Process:** the exe or process name, the same whole name ignoring case. `keepass.exe` matches `KeePass.exe`; `KeePass.exe.bak` does not.
- **Title:** a keyword found anywhere in the window title, ignoring case.

### Detectors (PRI-3.2)
One table holds each detector's id, placeholder tag, display name, pattern and check. The Privacy tab gets the names from `privacy_detectors`, so nothing is listed twice.

| Id | Tag | Pattern (candidates) | Check |
|---|---|---|---|
| `email` | `EMAIL` | `name@domain.tld` | — |
| `phone` | `PHONE` | `+` and 8–15 digits (E.164, with spaces, dots, dashes, brackets); Indian mobiles, `[6-9]` and 9 more digits, optionally after `+91` or `0`; `(NNN) NNN-NNNN` / `NNN-NNN-NNNN` | — |
| `card` | `CARD` | 13–19 digits, spaces and dashes allowed | Luhn |
| `iban` | `IBAN` | 2 letters, 2 digits, 11–30 letters or digits, spaces allowed | mod-97 |
| `apiKey` | `APIKEY` | `sk-`, `sk-ant-`, `AKIA`, `ghp_`, `github_pat_`, `xoxb-`/`xoxp-`, `AIza` with the usual lengths; JWTs (`eyJ….eyJ….…`) | — |
| `aadhaar` | `AADHAAR` | 12 digits not starting with 0 or 1, spaces allowed after every 4 | Verhoeff |
| `pan` | `PAN` | `[A-Z]{5}[0-9]{4}[A-Z]` | — |
| custom | `CUSTOM_<LABEL>` | the user's regex | — |

- **Overlaps:** when two matches overlap, the earlier detector in this order wins: custom, API key, email, IBAN, card, Aadhaar, PAN, phone. Phone comes last, so a card or Aadhaar number is never taken for a phone number.
- **Number patterns need word boundaries**, and the checks (Luhn, mod-97, Verhoeff) throw out most look-alikes, such as order ids.
- **Empty matches** of a custom pattern (`a*`) are skipped.

## Key mechanics

### Local-only (PRI-1)
- **PRI-1.1** is done (spec 04): `net.rs` refuses cloud hosts before connecting. Chat, model lists, Test connection and the health check all go through it. Speech (spec 09) will too.
- **PRI-1.3:** when `update_settings` sees local-only turn on and the default model is cloud, it sets the default to `llm::local_default()`: the first local or trusted-LAN provider's first chosen model. The tray, the Privacy tab and Settings all go through `update_settings`, so they behave the same.
  - With no local model, the default stays, and `availability()` says **"No local model — set one up."** The bubble shows that in its "No AI model is available" view, which has an Open setup button. The Privacy tab shows the same string via `model_problem`, so the wording lives in one place.
  - Turning local-only off doesn't switch back; the user picks again.
- **PRI-1.2:**
  - The pickers already disable cloud models with "(local-only)" (LLM-5.5), and the Models tab explains why.
  - The bubble's badge becomes **Local-only** while the mode is on (`badge()` gains the flag).
  - The tray item reads **Local-only** and stays checked, and the tooltip says "Cursora · Local-only".

### Blocklist (PRI-2)
- **`capture()` checks the rules right after reading the foreground app**, before the admin check. A match returns `Notice::Blocked` with no content. No accessibility read, no keys, no clipboard: the same place the admin check stops today. The window title is cleared from the capture, so a blocked app's title is neither shown nor saved in history.
- **The bubble opens anyway**, with "Cursora is blocked in KeePass, so nothing was read." and an empty input for a general question, like the admin-app notice. Nothing from the app can reach a model.
- **Tray "Block Cursora in <app>" (PRI-2.4):** while the tray menu is open, the foreground app is the taskbar. So "<app>" is the app the bubble last opened over. `on_ask` tells the tray, which rebuilds the menu only when that app changed. The item adds a process rule. It is hidden when the app is unknown or already blocked.
- **PRI-2.5:** `capture::identifies_apps()` is true on Windows and false in the Linux stub until spec 08, where it's true on X11 and false on Wayland. `app_info` passes it to the Privacy tab, which then warns: "Cursora can't tell which app is in front in this session, so the blocklist can't be enforced."

### Redaction (PRI-3)
- **When:** the request's model is cloud and `redact` is on, or the model is local or LAN and `redactLocal` is on (PRI-3.5). LAN counts as local, as everywhere since the 2026-10-11 amendment.
- **What:** every message of the request: the system message (a skill's `{{input}}` and the persona's own instructions are typed by the user), the captured text and files, the questions, and earlier answers.
- **Numbering (PRI-3.1):** values are numbered per type in the order they first appear in the whole conversation: the capture, then each turn's question and answer, then the system message. That order only grows, so a value keeps its placeholder for the rest of the conversation, even after BUB-5.2 drops early turns from the request. A reopened conversation numbers the same way.
- **The model is told:** when anything was hidden, `prompt::PLACEHOLDERS` goes at the end of the system message: "Some values were replaced with placeholders such as [EMAIL_1] before this was sent. Write a placeholder exactly as it is where you need that value, and don't guess what it stands for."
- **Restoring (PRI-3.3):** `Restorer::push(chunk)` returns the text that can be shown, with known placeholders swapped for their values. A trailing `[` followed only by capitals, digits and `_` (at most 64 characters) is held back until the next chunk. `finish()` flushes it when the stream ends, is stopped or fails. Unknown placeholders stay as written. The turn stores the restored text, so the bubble, Copy and history show the user's own values.
- **The map lives in memory for one request.** History keeps the original text, which never leaves the machine (BUB-9). Logs get counts only ("hid 2 values"), never values (APP-7.2).

### "What will be sent" (PRI-3.6, 3.8)
- A disclosure button under the conversation: **What will be sent ▸**. Open, it calls `send_preview(question)` with the current draft (debounced 250 ms) and shows:
  - who receives it, using the header's `badge()` ("To Cloud · OpenAI · gpt-4o")
  - the request as text (the same rendering as the developer preview), with each placeholder in a `<mark>`. Rust returns the placeholders it used, and TS marks those strings, so the pattern isn't written twice.
  - **PRI-3.8:** "Images are sent as they are: redaction covers text only." when a cloud request carries images. Spec 10 adds images; until then the warning is unit-tested only.
- A draft that is a Cursora command (`/help`, `/clear`, an unknown name…) shows "Cursora answers this itself; nothing is sent."

### Confirm before sending (PRI-3.7)
- **Rust decides.** `ask(question, confirmed)` and `regenerate(confirmed)` return `"sent"` or `"confirm"`. They return `"confirm"` and send nothing when `confirmCloud` is on, the request's model (the skill's own, if it has one) is cloud, and `confirmed` is false.
- On `"confirm"`, the bubble opens the panel for that question with a **Send to <provider>** button, which repeats the call with `confirmed: true`. Editing the draft or Esc cancels.
- The skill form's **Test** is itself the explicit action: it sends only the sample text and the template on screen.

### Settings → Privacy
1. **Local-only:** the switch, what it does, and `model_problem` when there is no local model (with "Open Models").
2. **Blocked apps:** the rules as rows ("Process name" or "Window title contains", value, Remove), an Add row, and the PRI-2.5 warning.
3. **Hide personal details:** "Before sending to cloud models" (`redact`), one checkbox per detector, "For local models too", and custom patterns (label, regex, Remove, Add). A bad regex is refused on save with the regex crate's message (PRI-3.4).
4. **Confirm before sending to cloud.**

## Commands and events

| Command | Use |
|---|---|
| `ask(question, confirmed)` → `"sent"` \| `"confirm"` | Was `ask(question)`. |
| `regenerate(confirmed)` → `"sent"` \| `"confirm"` | Was `regenerate()`. |
| `send_preview(question)` | `{ to: ModelInfo, text, placeholders, imageWarning, local }` for the panel. |
| `privacy_detectors()` | `[{ id, name }]` for the Privacy tab. |
| `model_problem()` | Why no model can answer, or null (`availability()`'s error). |
| `app_info()` | Gains `identifiesApps`. |

No new events: settings changes already emit `settings-changed`, which the badge and the tab follow.

## Changes to earlier specs' code
- **Spec 02:** `capture()` checks the blocklist first; `Notice::Blocked`; `CaptureSettings.blocklist`.
- **Spec 03:** the blocked notice, the badge, the panel, and `ask`/`regenerate` with `confirmed`.
- **Spec 04:**
  - `availability()`'s message when there is no local model.
  - The PRI-1.3 switch in `update_settings`.
  - **Echo can pose as a cloud model:** `CURSORA_ECHO=cloud`, debug builds only. Then redaction, the confirm flow and the badge can be tested end to end without API keys (issue 16). Echo never touches the network either way.
- **Spec 05:** the developer prompt preview goes through `chat::request()`, so it shows redaction for a cloud model (its design said redaction would be added there in spec 07). The prompt gains the `PLACEHOLDERS` line and a snapshot for it.
- **Spec 06:** the skill form's Test goes through `chat::request()`; its answer is restored in one go.

## Dependencies
- **`regex = "1"`.** It is already compiled as part of Tauri's dependency tree (1.13 in `Cargo.lock`), so nothing new is downloaded. It is needed for the user's own patterns (PRI-3.4). It runs in linear time, so a user's pattern can't hang a request the way a backtracking engine can.

## Decisions
1. **Redact where requests are built, not in `net.rs`** (see Overview). Alternative rejected: redacting in `ProviderModel::stream`. It sees only one request, so a value's number could shift once BUB-5.2 drops early turns.
2. **Number from the whole conversation**, so placeholders never change within a conversation, instead of storing a map that would need saving with history.
3. **The tray blocks the app the bubble last opened over**, because the real foreground app is the taskbar while the menu is open.
4. **A blocked app still opens the bubble**, with the notice and a general chat, like admin apps. The notice is the "near the cursor" message PRI-2.1 asks for.
5. **Rust decides when to confirm**, so the rule exists once, and a view can't skip it.
6. **The blocklist lives in `CaptureSettings`**, next to the other per-app capture rules.

## Known limits
- `// shortcut:` detection is by pattern. Names, street addresses and anything not in the table go out as they are. The panel shows exactly what goes. Upgrade: a local NER model, if users ask for names.
- `// shortcut:` phone numbers: E.164 with `+`, Indian mobiles, and the `(NNN) NNN-NNNN` style. Other national formats without `+` are missed. Upgrade: add formats as users report them.
- `// shortcut:` a placeholder-shaped string already in the user's own text (`[EMAIL_1]`) is shown as the hidden value in the answer if one has that placeholder. Upgrade: if it is ever seen.
- Window-title rules see the title only at the moment of the hotkey.
- On Linux the blocklist can't be enforced until spec 08 identifies apps; the Privacy tab says so (PRI-2.5).

## Test strategy

| Criterion | Test |
|---|---|
| PRI-1.1 | Done in spec 04: with local-only on, a cloud URL is refused with `blocked_local_only` before connecting. |
| PRI-1.3 | `local_default()` picks the first local or LAN model when the default is cloud and leaves a local default alone; with no local model, `availability()` gives "No local model — set one up." |
| PRI-2.1 | A blocked app on the fake platform: the only call is `foreground`, the notice is `Blocked`, the title is cleared. |
| PRI-2.2 | Process names match whole and ignoring case; title keywords match anywhere, ignoring case; neither matches the other's field. |
| PRI-2.3 | The default blocklist has every process name the requirement lists. |
| PRI-3.1 | A conversation with an email in the selection, a question and an earlier answer: every message has `[EMAIL_1]`, none has the address; a second address is `[EMAIL_2]`; the numbers stay the same when an early turn is dropped; the system message ends with `PLACEHOLDERS` (snapshot). |
| PRI-3.2 | A table per detector: values that must be found, and look-alikes that must not (a card that fails Luhn, an IBAN that fails mod-97, an Aadhaar that fails Verhoeff, a short `sk-`, a date, an order number). |
| PRI-3.3 | A text with placeholders, streamed split at every position, comes out the same as restoring it whole; an unknown placeholder stays; `finish()` flushes a held `[`. |
| PRI-3.4 | A detector switched off leaves its values; a custom pattern labelled `emp` gives `[CUSTOM_EMP_1]`; an invalid regex and a bad label are refused with the reason. |
| PRI-3.5 | Local and LAN requests are unchanged unless `redactLocal` is on; cloud ones are redacted unless `redact` is off. |
| PRI-3.7 | The gate: confirm on + cloud + not confirmed → `confirm`; local, confirmed, or confirm off → `sent`. |
| PRI-3.8 | A cloud preview with an image warns; a local one, or one without images, doesn't. |

Manual: PRI-1.2, 2.4, 2.5, 3.6, and 4.1 (a network monitor: on Windows, the remote addresses of Cursora, its WebView2 processes and llama-server polled through a session; in WSL, `ss -tnp`). End-to-end runs also cover 2.1 (a Notepad rule), 3.1/3.3 (Echo as a cloud model) and 3.7.

## Changes during implementation
- **2026-10-13, build:**
  - **`send_preview` lives in `chat.rs`**, next to `begin`, since it needs the conversation. "Cursora answers this itself" is a preview with no receiver (`to: null`) instead of a `local` flag.
  - **"Before sending to cloud models" is the main switch:** with it off, "For local models too" does nothing, and the Privacy tab greys out everything under it.
  - **A request is built on a copy of the conversation and stored only once it is built.** A custom pattern that can't compile (only possible in a hand-edited settings file) stops the send with its error and changes nothing; nothing is ever sent unredacted because of it.
  - **One new-question constructor, `Turn::asking()`**, for the ask, the preview and the sample conversation.
  - **A custom label is 1–20 letters or digits**, so a stream holds back at most 48 characters while a placeholder may be arriving.
- **2026-10-13, end-to-end run:**
  - **Each rule's input is labelled with its kind and value** ("Process name: KeePass.exe"). With the kind alone, every process row had the same name for screen readers.
  - **The panel opens scrolled to the end**, where the selection and the question are; the system message comes first and is the same every time.
  - **The panel is padded like the rest of the bubble**; it was flush with the left edge.
  - **Badges never wrap:** in a crowded header, "🔒 Local-only" broke onto two lines.
- **2026-10-13, review:**
  - **The tray menu is never rebuilt while the settings lock is held**, and `seen` runs after the bubble opens. A tray update waits for the main thread, which may be waiting for that lock.
  - **"What will be sent" refreshes when the settings change**, so a model switch shows the new receiver before Send.
  - **The pattern form saves directly**, so a refused pattern shows its reason once, next to the form.
