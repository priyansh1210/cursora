# 26: Web search with sources (design)

Status: **Approved**¹ (2026-10-29). Implements [requirements.md](requirements.md). A system command; one POST to Tavily in the answer's worker; the results appended to the request.

## Overview
```
/web <question>   chat::ask → web::refusal (local-only, no key, nothing to search) → a local reply, or answer()
chat::begin       web::query (typed text, then the selection, up to 400 characters), redacted (PRI-3.1); confirm as a cloud request (PRI-3.7);
                  build() adds web::INSTRUCTIONS as the request's instructions
worker            web::search → Tavily POST /search (key from the keychain, net::json, local-only guard)
                  → web::block(results) appended to the last user message → the model → web::sources(results) appended to the answer
send_preview      Preview.web = the query, shown as "First, Tavily is searched for: …"
```

## Components
| File | Change |
|---|---|
| `src-tauri/src/web.rs` | New: `KEY`, `INSTRUCTIONS`, `query`, `refusal`, `request`, `results`, `block`, `sources`, `search`. |
| `src-tauri/src/skills.rs` | `WEB` in the system commands. |
| `src-tauri/src/chat.rs` | `/web` in `ask`; `begin` and `send_preview` find the query; `build` takes the web instructions; the worker searches first and adds the sources. |
| `src-tauri/src/prompt.rs` | `defuse` also breaks `</search_results`, and is shared with `web.rs`. |
| `src-tauri/src/llm/commands.rs` | `key_status`, so Settings can show a saved key's last 4 characters without the key. |
| `src/Models.tsx` | The Web search group: the key field, Save, Remove. |
| `src/Chat.tsx`, `src/conversation.ts` | "What will be sent" shows the query. |
| `specs/00-constitution.md` | The §3 amendment. |

## Decisions
1. **A system command, not a skill**: a skill is a prompt template; `/web` needs a network call before the model runs, and must be refused in local-only mode.
2. **The search runs in the worker**, after the turn is shown, so the bubble never waits on the network on the main thread. Stop takes effect when the search returns, at most 20 s later; the search itself doesn't watch for it.
3. **Room for the results is kept when the request is built**: `build()` keeps 1,200 tokens free for a web question, so older turns are dropped to make room (BUB-5.2) instead of the results overflowing a small context (found in the review).
4. **Results go in the user message**, defused and delimited like a selection (constitution §5); the instructions go in the system message, as a skill's do.
5. **The key goes in the `Authorization` header**, from the keychain, read in Rust only (constitution §3).
6. **Debug builds can point at a fake server** (`CURSORA_WEB_URL`), like the Echo model (`CURSORA_ECHO`), so the whole flow can be tried without a real key. Release builds always use Tavily.

## Test strategy
- Rust: the query from typed text or the selection, cut at 400 characters (WEB-1.2); Tavily's JSON to at most 5 results of at most 600 characters; the block defused and numbered; the sources as links; the request's address, header and body (WEB-1.2); an email in the query hidden (WEB-1.3); the refusals (WEB-1.4); a search against a local server that refuses the key gives an Auth error (WEB-1.5); `/web` in `/help` and reserved (WEB-1.6).
- By hand on Windows with the test copy and a fake search server: a `/web` question answered with [1] citations and the Sources list; "What will be sent"; local-only; no key. The user tries a real Tavily key.

¹ Approved under the user's go-ahead of 2026-10-28 (requirements.md).
