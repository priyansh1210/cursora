# 13: Write back into apps (design)

Status: **Approved** (2026-10-26, under the user's go-ahead to build ideas #1 to #5 while away, choices listed in requirements.md for review). Implements [requirements.md](requirements.md). It's the capture run backwards: the clipboard is saved, the answer goes onto it, the app gets Ctrl+V, and the clipboard is put back. All of it uses the `Platform` trait the capture already has.

## Overview
```
answer done → the view shows Replace / Insert when conv.write_back is Some (WRT-1.1, 2.1, 3.1)
click → write_back(turn) command
      → plain_text(answer) (WRT-1.2)
      → hide the bubble (as closing it: focus may go back on its own) and activate the source window
      → wait up to 1 s until that window is in front, else Err "Cursora couldn't bring <app> back…" (WRT-1.3, 4.2)
      → on a capture-style thread: capture::paste(&platform, text):
            modifiers released? clipboard snapshot (None → Err, WRT-4.1) → clipboard = text → Ctrl+V
            → 500 ms for the app to read it → clipboard restored
```

## Components
| File | Change |
|---|---|
| `src-tauri/src/capture/mod.rs` | `Platform` gets `text_snapshot(&str) -> Snapshot` and `send_paste()`. New `paste<P: Platform>(p, text) -> Result<(), &'static str>`. New `write_back(content, app, cfg, session) -> Option<WriteBack>`, the pure rule of WRT-3.1. |
| `src-tauri/src/capture/windows.rs` | `text_snapshot`: `[(CF_UNICODETEXT, UTF-16 with its terminating 0)]`. `send_paste`: Ctrl+V through `SendInput`, as `send_copy`. |
| `src-tauri/src/capture/linux.rs` | `text_snapshot`: `UTF8_STRING` and `STRING` items, served by `x11::serve` like a restored clipboard. `send_paste`: Ctrl+V through XTest, as `send_copy`. |
| `src-tauri/src/chat.rs` | `Conversation.write_back: Option<WriteBack>` (`"replace"` or `"insert"`), set in `start()`, never saved to History (`#[serde(skip_deserializing)]`, so a reopened one has none). New command `write_back(window, index)`. `plain_text(&str)`. |
| `src-tauri/src/bubble/mod.rs` | `hide_for_write_back()`: hides without cancelling or activating (`close()` without the activate), since the command activates the app itself. |
| `src-tauri/src/skills.rs`, `src-tauri/src/prompt.rs` | `output: replace` in the front matter → `Skill.replace`. The skill's instructions then end with the replacement rule (WRT-5.1). |
| `src-tauri/skills/{fix,rewrite,simplify,translate}.md` | `output: replace`; `/fix`'s "then a short list of what changed" goes (WRT-5.2). |
| `src/Chat.tsx`, `src/conversation.ts` | The first action of a finished answer: **Replace** or **Insert** when `conv.writeBack` is set. Errors show in the bubble like other errors. |

## Key mechanics

### Which mode (WRT-3.1)
```rust
pub enum WriteBack { Replace, Insert }
pub fn write_back(content: &Content, app: &SourceApp, cfg: &CaptureSettings, session: Session) -> Option<WriteBack>
```
It's `None` for any of these:
- a Wayland session
- an app with no window (`window == 0`)
- an app that is blocked or elevated
- an app on `cfg.unsafe_apps` (case-insensitive, like `copy_key`)
- files or a snip as the context

`Replace` for text, `Insert` for none. `chat::start` stores it on the conversation. `#[serde(skip_deserializing)]` keeps it from coming back from History (the window is gone by then), while the view still receives it.

### The paste (WRT-1.1, 4.1)
`capture::paste` mirrors `copy_selection`:
1. Wait up to 300 ms for Shift, Alt and Win to be released. A held Alt would turn Ctrl+V into Ctrl+Alt+V, and Cursora's own hotkey may still be down.
2. `clipboard_snapshot()`, or `Err` if it can't be saved exactly.
3. `clipboard_restore(text_snapshot(text))`, which sets the clipboard to the text.
4. `send_paste()`.
5. Sleep 500 ms.
6. `clipboard_restore(snapshot)`.

`shortcut:` the 500 ms is a guess at how long an app takes to read a paste; a very busy app could read the restored clipboard instead. Upgrade to delayed rendering (the app asks for the data, and we answer) if anyone sees the old clipboard pasted.

### Bringing the app back (WRT-1.3, 4.2)
The command runs async:
1. `bubble::hide_for_write_back`.
2. `os::activate(window)`.
3. Poll `os::foreground() == window` every 20 ms, for up to 1 s.
4. On success, a `std::thread` with a fresh `capture::OsPlatform` does the paste. Windows COM and the X11 connection belong to the thread, as in the capture.
5. On failure, the bubble shows again with the error. The conversation is still there, since hiding didn't clear the slot.

### Plain text (WRT-1.2)
`plain_text` works line by line:
- code fences are dropped, and their contents kept
- `#`s at the start of a line go
- `**`, `__` and backticks are removed
- `[text](url)` becomes `text (url)`
- a leading `> ` goes

Single `*` and `_` stay, since they're too often real text (file_names, 2*3).

### Replacement skills (WRT-5)
`Skill.replace` comes from `output: replace`; any other value of `output` is a SKL-1.3 error naming the line. The skill's rendered instructions end with: "Reply with the new text only: no introduction, no quotes around it, no explanation."

## Decisions
1. **Paste, not typed keys** (requirements default 1).
2. **Only the window the bubble opened over**, checked right before pasting.
3. **No diff view.** The answer is the new text, and the app still shows the old one until Replace.
4. **The rule is one pure function** (`capture::write_back`), so it's tested without an OS.

## Test strategy
| Criterion | How |
|---|---|
| WRT-1.2 | `plain_text` on fences, headings, emphasis, links, quotes; dashes and line breaks kept. |
| WRT-3.1 | `capture::write_back` for text, none, files, snip, blocked, elevated, an unsafe app in other letter case, Wayland, no window. |
| WRT-4.1 | `capture::paste` with the Fake platform: the order is snapshot → text → Ctrl+V → restore; a clipboard that can't be saved means no paste and an `Err`. |
| WRT-4.2 | The foreground check is a pure helper, `came_back(poll)`: it gives up after the deadline when another window stays in front. |
| WRT-5.1, 5.2 | Skill parsing of `output: replace` (and an error for another value); the four built-ins are replacement skills, and the rule ends their instructions. |
| Manual | WRT-1.1, 1.3, 2.1 on Windows: Notepad, Word, a Chrome text box and Outlook or Gmail compose for Replace and Insert; a page's text in Chrome (read-only, so nothing changes, and Copy remains); Windows Terminal (no Replace). On X11: mousepad. |
