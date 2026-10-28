# 25: Hover to define (design)

Status: **Approved**¹ (2026-10-28). Implements [requirements.md](requirements.md). A third hotkey; the snip's screen grab and spec 20's text recognition, read word by word.

## Overview
```
hotkey::on_press   Define key → snip::define(app)     (the off list passes it through first, OFF-1.1)
snip::define       blocklist? → hide the bubble → os::grab() → crop 800×120 around the pointer
                   → ocr::words(crop) → word_at(words, pointer) → bubble at the pointer, Content::Text(word)
                   → "snip-ask" "/define" (the view sends it as if typed)
ocr::words         Windows: OcrResult lines → words with BoundingRect. Linux: `tesseract stdin stdout tsv`.
```

## Components
| File | Change |
|---|---|
| `src-tauri/src/settings.rs` | `Hotkeys::define`, "Ctrl+Alt+G" by default. |
| `src-tauri/src/hotkey.rs` | Validated with the others; registered after Ask and Snip, so its failure costs neither; `on_press` sends it to `snip::define`. A clash on a Snip or Define key the change didn't touch is only logged (decision 6). |
| `src-tauri/src/lib.rs` | Saving re-registers the hotkeys when any of them changed, not only Ask. |
| `src-tauri/src/snip/mod.rs` | `crop()` out of `prepare()`; `define()`; `word_at()`. Shares `SNIPPING`, so a define and a snip never grab at once. |
| `src-tauri/src/snip/ocr.rs` | `words()`: each word's text and box (`bubble::Rect`); `tsv()` parses Tesseract's table. |
| `src-tauri/src/capture/mod.rs` | `Method::Screen`: read from the screen, so no Replace or Insert (HOV-1.6). |
| `src/HotkeyInput.tsx`, `src/useSettings.ts` | The Define field. |
| `src/Chat.tsx` | `snip-ask` also asks for a text context (the defined word). |
| `user-manual.html` | A paragraph under the hotkeys. |

## Decisions
1. **Text recognition on the screen** rather than UI Automation's `RangeFromPoint`: one way on both systems and in every app, using what spec 20 already ships. It reads what is drawn, so it can misread small or stylised text; the word is shown in the bubble, so a misreading is visible.
2. **A band of 800 × 120 pixels** around the pointer: wide enough for a long word at 200% scaling, a few lines high, and small enough to read in well under a second.
3. **The word is asked about by the view**, through the snip's existing `snip-ask` event, not by Rust: confirming before a cloud request (PRI-3.7) lives in the view.
4. **`Method::Screen`** marks the capture, and `write_back()` returns None for it. Without it, Replace would paste the definition at the app's caret over nothing that was selected.
5. **Rejected: a separate tooltip window.** A new window type, theme and lifetime for what the bubble already does.
6. **A clash on an unchanged key doesn't block a change** (found in the review). Ctrl+Alt+G is new for everyone who updates, and another app may hold it. Registering stops with an error only for a key the change touched; the start still says it (SEL-1.3). The same rule now covers Snip, which until now also needed a restart after it was changed in Settings.

## Test strategy
- Rust: `word_at` picks the word under the point, with slack, and none between lines (HOV-1.2); `tsv()` parses Tesseract's table (HOV-1.2); the OCR fixture is read word by word, and the word at a point is found (HOV-1.2, where text recognition exists: Windows, WSL); `crop()` clips to the screen; the hotkey is validated and clashes are refused (HOV-1.1); `write_back` is None for `Method::Screen` (HOV-1.6); the blocklist stops a define before any grab (HOV-1.5); no word gives the notice (HOV-1.4).
- By hand on Windows with the test copy: point at a word in Notepad and press; then another word; a word in a picture or a PDF if one is at hand.

¹ Approved under the user's go-ahead of 2026-10-28 (requirements.md).
