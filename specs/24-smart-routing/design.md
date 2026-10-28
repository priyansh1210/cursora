# 24: Smart routing (design)

Status: **Approved**¹ (2026-10-28). Implements [requirements.md](requirements.md). One setting; the model choice in `chat::resolve`, and a second try in `chat::begin`.

## Overview
```
Settings → Models → Smart routing → Quick model  →  settings.models.quick: Option<ModelRef>
llm::quick(models)        the quick model, if it's still chosen and local or trusted LAN (ROU-1.4)
chat::resolve             skill's own model → conversation's own → quick model for a quick job (ROU-1.2, 1.3) → default
chat::begin               also builds the request for the quick model; the worker tries it when the first model
                          failed with nothing written (ROU-2.1, 2.2), and adds a note to the turn
llm::health               a failing default doesn't block the bubble while a quick model is set (ROU-2.3)
```

## Components
| File | Change |
|---|---|
| `src-tauri/src/llm/mod.rs` | `ModelSettings::quick`; `quick()`; `health()` lets the bubble open. |
| `src-tauri/src/chat.rs` | `QUICK_JOBS`, `QUICK_MAX_CHARS`, `quick_job()`; `resolve()` routes; `begin()` builds and runs the fallback; `falls_back()`; `reference()` becomes public for `llm::quick`. |
| `src/Models.tsx`, `src/useSettings.ts` | The Smart routing group; `ModelSelect` takes an `off` label; removing a provider clears its quick model too. |
| `user-manual.html` | A paragraph under Models. |

## Decisions
1. **Routing sits in `resolve()`**, so Send, Regenerate, Retry and "What will be sent" all name the same model, and a skill's own model and a snip's one-time model still win (ROU-1.2, 1.5).
2. **The fallback request is built before the worker starts**, from the same conversation, for the quick model's context size and redaction rules. The worker only decides whether to use it. Building it costs a few string copies per question; the alternative, rebuilding inside the worker, needs the settings and skill moved into the thread.
3. **"Nothing written" is read from the turn**: the answer is still empty. A partial answer followed by an error is not retried, since the user already saw part of it (ROU-2.2).
4. **The header keeps the default model.** `begin()` sets the conversation's model to the default for a routed job, and leaves it for a fallback, so the header's model and its Local or Cloud badge always say where the next typed question goes (BUB-7.1). The answer's note names the model that answered (ROU-1.5).
5. **The hotkey check passes when a quick model is set**, instead of checking the quick model too. The quick model is local, so the question either reaches it or fails with the engine's own error.
6. **Rejected: a `route: quick` field in skill files.** It would let users route their own skills, but the four built-ins cover the update. Recorded as a shortcut in the code.

## Test strategy
- Rust: `quick()` counts a removed or cloud model as off (ROU-1.4); `quick_job()` for the four skills, others, a long selection, files and a snip (ROU-1.2, 1.3); the answering order with a skill's and a conversation's own model (ROU-1.2); `falls_back()` for each error kind and a partial answer, and the note (ROU-2.1, 2.2).
- By hand on Windows, with the test copy: a quick model set, Fix on a short selection answered by it, a free question by the default; a cloud provider at an address that can't be reached, then a question answered by the quick model with the note (ROU-1.1, 1.5, 2.3).

¹ Approved under the user's go-ahead of 2026-10-28 (requirements.md).
