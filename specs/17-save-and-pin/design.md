# 17: Save and pin answers (design)

Status: **Approved** (2026-10-26, PR #46). Implements [requirements.md](requirements.md). Rust owns the conversation (spec 03 design, decision 2), so both buttons send only which answer, and Rust copies or writes it. A pinned note is a small normal window with the answer. A saved answer is a Markdown file written by a new `keep.rs`.

## Overview
```
Answer (Chat.tsx), finished: [Copy] [Pin] [Save]
  Pin  → pin_answer(asked)  → chat::kept(): the turn, if finished (SAV-1.3)
                             → Pins state: "pin-<n>" → { question, answer }
                             → a new window "pin-<n>" beside the calling window
         Pin.tsx (in window "pin-<n>") → pinned() → the answer, Copy; the title bar's × closes it
  Save → save_answer(asked) → chat::kept() → keep::write(folder, ...) → the path
         → "Saved" and [Show in folder] → show_saved(path)
Settings → General → Saved answers: the folder, [Change…] (pick_folder), [Open] (open_folder "saved")
```

## Components
| File | Change |
|---|---|
| `src-tauri/src/keep.rs` (new) | `Pins` state; `file_name()`, `markdown()`, `write()`; commands `pin_answer`, `pinned`, `save_answer`, `show_saved`. |
| `src-tauri/src/chat.rs` | `kept(app, label, asked)`: the question, answer, app and model of a finished turn, or why not. |
| `src-tauri/src/settings.rs` | `saved_answers_folder: Option<PathBuf>`; None means `Documents/Cursora`. |
| `src-tauri/src/skills.rs` | `today()` becomes `local_time()` ("2026-10-26 21:14"); `{{date}}` takes its first 10 characters. |
| `src-tauri/src/llm/download.rs`, `src-tauri/src/lib.rs` | `model_folder` moves to `lib.rs` as `pick_folder`, for both folder pickers. `open_folder` gets `"saved"`. The new commands are registered, with `Pins` managed. |
| `src-tauri/capabilities/bubble.json` | `"pin-*"` joins the windows, so a note may open links and set its theme like the bubble. |
| `src/main.tsx` | A `pin-…` label shows `Pin`. |
| `src/Pin.tsx` (new) | The note: the answer rendered as in the bubble, and Copy. |
| `src/Chat.tsx` | Pin and Save on a finished answer. `CopyButton` and the link handler are exported for `Pin.tsx`. `notPut` becomes `failed`, for Replace, Insert and Save alike. |
| `src/Settings.tsx`, `src/Models.tsx` | The Saved answers row; `pick_folder` instead of `model_folder`. |
| `specs/03-bubble-ui/requirements.md` | BUB-6.1 and BUB-9.3 amended. |

## Key mechanics
- **Which answer:** each turn is named by `asked`, the time it was asked in milliseconds. It's unique within a conversation and doesn't move when the view adds a turn of its own. The view takes Rust's copy of the conversation when an answer is done (`chat-done`), so it holds Rust's `asked`, not the one it stamped on its own placeholder turn (a review finding: a 1 ms difference, or a Regenerate, made Pin and Save fail). `kept` finds that turn in the calling window's conversation and refuses one that is streaming, has an error, is empty, or is Cursora's own reply (`local`) (SAV-1.3). The buttons show under the same rule: status done or cancelled, not local.
- **A pinned note** (SAV-1.1, 1.2): `pin_answer` stores a copy of the question and answer under a new label `pin-<n>` (a counter), so later changes to the conversation don't reach it. Then it opens a window:
  - **Its window:** a normal window with a title bar (the question's first line, up to 60 characters), 360 × 260, at least 240 × 140, always on top, not focused, so typing stays where it was. The title bar gives moving, resizing and Close without new code, and follows the theme like the expanded window.
  - **Where it opens:** beside the calling window: to its right, or to its left when there's no room, each new note 24 px lower than the last.
  - **When it closes:** the window's `Destroyed` event removes its entry from `Pins`. Notes aren't saved anywhere, so they're gone after a restart, as asked.
  - **Inside:** `Pin.tsx` calls `pinned()` once, renders with the bubble's Markdown renderer (BUB-4.2, so no HTML and no images) and handles links the same way.

  `shortcut:` no limit on open notes, though each one is its own webview with its own memory. Add a limit if users pin dozens.
- **The saved file** (SAV-2.2):
  ```
  ---
  saved: "2026-10-26 21:14"
  app: "Notepad"
  model: "gemini-flash-lite-latest"
  ---

  > What is due on Friday?

  <the answer, as shown>
  ```
  The three values are written as JSON strings, which are also valid YAML, so Obsidian reads them as properties. Every line of the question gets `> `. The answer is the turn's text, which already has hidden values put back (PRI-3.3). The selection and files are never read.
- **The file name** (SAV-2.3): `"<date> <question>"`. Characters Windows or Linux refuse (`\ / : * ? " < > |` and control characters) become spaces, runs of spaces become one, and dots and spaces at the end go. The name is cut to 80 characters, never mid-character, then `.md` is added. A command such as `/explain` gives `2026-10-26 explain.md`. The date at the front means no name is ever a Windows device name (`CON`, `NUL`).
- **Never overwriting** (SAV-2.3): the file is opened with `create_new`. If the name is taken, it tries `… (2).md`, `… (3).md` and so on. Two saves at once can't both get the same name, because there's no check-then-write gap.
- **The folder** (SAV-2.4): `saved_answers_folder`, or `document_dir()/Cursora`, or `home_dir()/Documents/Cursora` when the system names no Documents folder. `create_dir_all` runs before each save. An error becomes "Couldn't save the answer in <folder>: <the system's reason>", shown under the answer, which stays in the bubble.
- **Show in folder:** `show_saved(path)` accepts any existing `.md` file, so it still works after the folder setting changes (revealing a file never opens it), then calls the opener plugin's `reveal_item_in_dir`, which selects it in Explorer or the Linux file manager.
- **History off** (SAV-2.6): `save_answer` never looks at the history settings and writes only the one file.

## Decisions
1. **Rust copies the answer, and the page sends only `asked`.** One source of truth, and a page can't save text that was never answered. *Rejected:* sending the text from the page.
2. **A note with a title bar, not a frameless one.** Moving, resizing and closing come free on Windows and Linux; frameless windows on Linux can't be resized by their edges. *Rejected:* one shared "pinned answers" window, which is less useful than notes placed where the user wants them.
3. **Notes not taskbar-hidden.** They stay on top, and the taskbar and Alt+Tab help find and close them.
4. **Plain Markdown with front matter** works with any editor, Obsidian included. *Rejected:* one file per day collecting answers, which needs appending and locking.
5. **`pick_folder` shared with spec 15.** One command for both pickers, one name per thing.

## Platform notes
- **Linux X11:** always on top works under Openbox and GNOME on Xorg. **Wayland:** GNOME doesn't let apps keep a window on top, so a note there is an ordinary window (best effort, constitution §7).
- **Show in folder on Linux** goes through the file manager's portal. Without it (WSL, some minimal desktops) the folder opens instead, with `xdg-open`, and a failure is said in plain words (R73).
- **Documents folder:** Tauri's `document_dir()` follows `XDG_DOCUMENTS_DIR` on Linux and the Known Folder on Windows, OneDrive-redirected or not.

## Test strategy
| Criterion | How |
|---|---|
| SAV-1.3 | `kept` refuses streaming, error, empty and local turns, and returns done and cancelled ones. A stored pin keeps its text after the conversation's turn changes. |
| SAV-2.2 | `markdown()` holds the date, app, model, the quoted question (two lines) and the answer, and not the selected text of the conversation it came from. |
| SAV-2.3 | `file_name()`: refused characters, trailing dots and spaces, an 80-character cut through Devanagari, `/explain`. `write()` twice with the same name gives `(2)`, and the first file is unchanged. |
| SAV-2.4 | With no setting, the default folder ends in `Documents/Cursora` and is made on the first save. A folder path that is a file gives the plain error. |
| SAV-2.6 | A save with "Save history" off writes the file, and only it. |
| Manual | SAV-1.1, 1.2, 1.4, 2.1, 2.5 on Windows and in WSL (X11). |
