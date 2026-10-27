# 22: Personal profile (design)

Status: **Approved**¹ (2026-10-28). Implements [requirements.md](requirements.md). One setting, one more part in the system message.

## Overview
```
Settings → Personality → About you (textarea, maxLength 600)  →  settings.about_you
chat::request → assemble(…, about) → prompt::system(persona, language, about, skill)
  BASE · persona block (or the language line) · "About the user, in their own words: …" · skill · UNTRUSTED
request() → redaction over every message, the system one included (PRI-3.1)
```

## Components
| File | Change |
|---|---|
| `src-tauri/src/settings.rs` | `about_you: String`, empty by default. |
| `src-tauri/src/prompt.rs` | `system()` takes `about`; `ABOUT_MAX` = 600 characters. |
| `src-tauri/src/chat.rs` | `assemble()` passes `settings.about_you` on; its tests pass `""`. |
| `src/Personality.tsx`, `src/useSettings.ts` | The About you group. |
| `src/Settings.tsx`, `src-tauri/capabilities/default.json` | On close, the focused field is blurred first, so it saves (decision 4). |

## Decisions
1. **Not delimited as untrusted.** It's the user's own words, like a persona's custom instructions (PER-1.3), not text from the screen.
2. **Cut at 600 characters in Rust too**, so a hand-edited settings file can't send a page of text with every question (PRF-1.3).
3. **After the persona, before the skill**, so a skill's instructions still come last and win.
4. **Saved when the box loses focus, and Settings takes the focus away when it's asked to close.** The review found that typing and then closing Settings with its × lost the text: closing doesn't move the focus, so no field that saves on blur saved, the blocklist's and off list's rules included. One close handler in Settings fixes them all; it needs `core:window:allow-destroy`, since the window then closes itself.

## Test strategy
- Rust: the system message with About you, with personality on and off, and without it when blank (PRF-1.2); cut at 600 characters (PRF-1.3); an email in it hidden for a cloud model (PRF-1.4).
- By hand on Windows: write it, ask, and see it in "What will be sent".

¹ Approved under the user's go-ahead of 2026-10-28 (requirements.md).
