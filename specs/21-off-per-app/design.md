# 21: Turn Cursora off per app (design)

Status: **Approved**¹ (2026-10-28). Implements [requirements.md](requirements.md). The check comes first in the hotkey's handler, before the bubble, the capture or the snip, and reuses the blocklist's rules. Passing the keys on means stepping aside: release the hotkey, press the keys again, then take the hotkey back.

## Overview
```
hotkey::on_press(shortcut)
  front = OsPlatform::foreground()
  capture::blocked(&s.hotkeys.off_in, &front)?        (OFF-1.1, the blocklist's matching)
    yes → pass_through(shortcut) on its own thread:
            chord(shortcut)? → unregister → Platform::send_chord → 100 ms → register    (OFF-1.2, 1.3)
            no chord → log why, nothing sent
    no  → as before: the snip key snips, the other asks
tray: "Turn Cursora off in <app>" (LAST_APP) ⇄ s.hotkeys.off_in                           (OFF-2.1)
Settings → General → Hotkeys: the rules, as in Privacy                                    (OFF-2.2)
```

## Components
| File | Change |
|---|---|
| `src-tauri/src/settings.rs` | `Hotkeys.off_in: Vec<Rule>`, empty by default. |
| `src-tauri/src/hotkey.rs` | `on_press` checks the list first. `Chord` and `chord()`: the hotkey as keys that can be pressed again, or None. `pass_through()`. |
| `src-tauri/src/capture/mod.rs` | `Platform::send_chord(&Chord)`; the fake records it. |
| `src-tauri/src/capture/windows.rs` | `send_chord`: `SendInput`, pressing only the modifiers that aren't already down (`GetAsyncKeyState`) and releasing only those. |
| `src-tauri/src/capture/linux.rs` | `send_chord`: XTest, the same, with the keymap from `QueryKeymap`. |
| `src-tauri/src/tray.rs` | The check item beside "Block Cursora in <app>". |
| `src/Privacy.tsx` | `RuleRow` and `AddRule` exported. |
| `src/Settings.tsx`, `src/useSettings.ts` | The list in General → Hotkeys. |

## Passing the keys on
Windows' `RegisterHotKey` and X11's key grab take the key press away from the app, and a key pressed again while the hotkey is held would only fire the hotkey again. So `pass_through`:
1. unregisters that one hotkey through the plugin. On X11, global-hotkey 0.8 only queues the ungrab, so a dummy registration (Ctrl+Alt+Shift+Win+F24, undone at once) makes it send the queue first; without it, the keys pressed again met the old grab and were lost (found in the X11 check of 2026-10-29, issue 67)
2. presses it again: the modifiers that aren't still held, then the key down and up, then releases only the modifiers it pressed, so a Ctrl the user still holds stays held
3. waits 100 ms, since injected keys reach the app's queue after the call returns and a hotkey registered too soon would take them again
4. registers the hotkey again

It runs on its own thread, so the 100 ms never holds up the main thread. During those 100 ms the hotkey does nothing anywhere, which can't be noticed.

`chord()` turns the plugin's `Shortcut` into modifiers (Ctrl, Alt, Shift, Win) and one key: `KeyA` to `KeyZ`, `Digit0` to `Digit9`, `F1` to `F24`, `Space`. Those cover what a hotkey is usually made of; mapping every key on both platforms isn't worth it (OFF-1.3).

## Decisions
1. **Constitution §6** says Cursora never sends a key combination that's destructive in the target app. The keys sent here are the ones the user just pressed in that app, so it does what they meant.
2. **The blocklist's rules and matching**, not a new kind: `capture::blocked` and the Privacy page's row editor.
3. **Not a low-level keyboard hook**, which could let the key through without taking it first: that means a new hook on Windows and no equivalent on X11.

## Test strategy
- Rust: an app on the list gets the keys and no bubble, and one that isn't doesn't (OFF-1.1); `chord()` for Ctrl+G, Ctrl+Shift+F9 and an arrow key (OFF-1.3); the fake platform records the chord.
- By hand on Windows: Notepad on the list, Ctrl+G opens Notepad's Go to line; off the list, the bubble opens. The tray item adds and removes Notepad; Settings shows the rule. On X11: the stronger PC (issue 2) unless WSL can show it.

¹ Approved under the user's go-ahead of 2026-10-28 (requirements.md).
