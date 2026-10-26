# 14: Quick actions (design)

Status: **Approved** (2026-10-26, under the user's go-ahead to build ideas #1 to #5 while away, choices listed in requirements.md for review). Implements [requirements.md](requirements.md). Rust works out the buttons when the bubble opens: the app's kind, its set, pinned skills first, without skills that can't run. They travel with the conversation, and a button sends its command exactly as if typed.

## Overview
```
capture (text selected) → chat::start
   → quick::kind(app, unsafe_apps): Terminal | Code | Mail | Other (QAC-2.1)
   → quick::actions(kind, pinned, skills, vision): pinned first, then the kind's set (QAC-2.2, 3.1),
       without unknown skills or vision skills the model can't serve (QAC-1.3), no duplicates, at most 6
   → Conversation.quick
bubble, no question yet → a row of buttons above the input; click or Alt+N → send(command) (QAC-1.1, 1.2)
```

## Components
| File | Change |
|---|---|
| `src-tauri/src/quick.rs` (new) | `Kind`, `kind()`, `QuickAction { label, command }`, `set(kind)`, `actions()`. |
| `src-tauri/src/settings.rs` | `quick: QuickSettings { on: bool (true), pinned: Vec<String> }`. |
| `src-tauri/src/chat.rs` | `Conversation.quick: Vec<QuickAction>`, set in `start()` for text when `quick.on`; not read back from History, like `write_back`. |
| `src-tauri/skills/reply.md`, `error.md` (new) | The two built-ins (QAC-4); `BUILT_IN` grows to 11; SKL-3.1 amended. |
| `src/Chat.tsx` | The button row (a `toolbar` with labelled buttons) before the first question; Alt+1 to Alt+6 in the bubble. |
| `src/SkillsTab.tsx` | "Show quick actions in the bubble" and a **Pin**/**Unpin** button on each skill card. |

## Key mechanics
- **`kind()`** looks the exe up case-insensitively, in this order. Editors come before terminals because the default `unsafe_apps` lists `Code.exe`, `Cursor.exe` and JetBrains IDEs for their built-in terminals.
  1. **Code:** `code.exe`, `code`, `cursor.exe`, `devenv.exe`, `idea64.exe`, `pycharm64.exe`, `webstorm64.exe`, `clion64.exe`, `rider64.exe`, `notepad++.exe`, `sublime_text.exe`, `zed.exe`.
  2. **Mail and chat:** `outlook.exe`, `olk.exe`, `thunderbird.exe`, `ms-teams.exe`, `teams.exe`, `slack.exe`, `discord.exe`, `whatsapp.exe`, `telegram.exe`, `signal.exe`, and their Linux names.
  3. **Terminal:** a short list (`WindowsTerminal.exe`, `alacritty.exe`, `kitty`, `gnome-terminal-server`, `konsole`, `xterm`, …), then the user's `unsafe_apps`, so their own additions count.
  4. **Other:** everything else.

  `shortcut:` a fixed table; apps it doesn't know are "Other". Add names as users report them.
- **A button's label** is the set's label. For a pinned skill, it's the skill's name with its first letter in capitals (`eli5` → "Eli5"), and its command is `/<name>`.
- **QAC-1.3:** a command's skill is its first word without the `/`. It must be among the loaded skills, and a skill with `requires: vision` needs a model that can see.
- **The row** shows only while `conv.turns` is empty and `conv.quick` isn't (requirements default 1). Each button's `title` says "Alt+N".
- **A click or Alt+N** sends `<command> <input text>`; `send` trims it, and the text is the skill's `{{input}}`.
- **Alt+N:** a `keydown` handler on the bubble's root runs when `altKey` and `1`–`6` match a button that is shown. It calls `preventDefault` so no character is typed.
- **Pinning** is a settings change: `quick.pinned` gains or loses the skill's name. A pinned name whose skill is later deleted is skipped (QAC-1.3).

## Decisions
1. **Worked out in Rust when the bubble opens**, like `write_back`. The view gets the result, and the rules are tested without a browser.
2. **A button is just a command**, so skills, replacement and redaction all work as when typed.
3. **No per-app settings page** (requirements Out). Pinning covers personal favourites.

## Test strategy
| Criterion | How |
|---|---|
| QAC-1.3 | `actions()` drops unknown skills and vision skills without vision. |
| QAC-2.1 | `kind()` for each kind, in other letter cases, a user-added unsafe app, and an unknown app. |
| QAC-2.2 | `set()` for each kind matches the table. |
| QAC-3.1 | Pinned skills first, no duplicates (a pinned `fix`), at most 6. |
| QAC-4.1, 4.2 | `reply` (chat) and `error` load as valid built-ins. |
| Manual | QAC-1.1, 1.2, 3.2 in the bubble (test window), the Pin button and the setting. |
