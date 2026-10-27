# 21: Turn Cursora off per app (OFF)

Status: **Approved**¹ (2026-10-28).

## Scope
- **In:** a list of apps where Cursora's hotkeys do nothing, so the keys reach the app as if Cursora weren't running: Ctrl+G stays "Go to line" in VS Code. Adding the app the bubble last opened over from the tray menu, and editing the list in Settings → General.
- **Out:**
  - Wayland, where Cursora has no hotkeys of its own (LNX-2): the desktop's shortcut runs `cursora --ask` in every app
  - Turning off the command line (`cursora --ask`): it was asked for by name
  - Other hotkeys per app: an app is in the list or not
  - A list that comes filled in: Ctrl+G clashes with different apps for different people

Update #5 from the list of 2026-10-28 ([improvements batch 2](../../memory/improvements-batch-2.md)); the backlog's "Disable Cursora per app".

## Requirements

### OFF-1 Hotkeys pass through
User story: As a VS Code user, I want Ctrl+G to go to a line in VS Code and to ask Cursora everywhere else.
- OFF-1.1 (auto) WHEN the Ask or Snip hotkey is pressed WHILE the app in front matches the off list THE SYSTEM SHALL open nothing, read nothing and grab nothing. The list's rules are the blocklist's (PRI-2.2): a process name, or words in the window title.
- OFF-1.2 (manual) In that case THE SYSTEM SHALL send the same key combination to the app, so it does what the keys do there, and then take the hotkey back for every other app. Keys the user is still holding stay held.
- OFF-1.3 (auto) THE keys SHALL be passed on only for a hotkey made of Ctrl, Alt, Shift or Win with a letter, digit, F1 to F24 or Space. Any other hotkey still does nothing in the app, and the log says why.

### OFF-2 The list
User story: As a user, I want to turn Cursora off in an app the moment its hotkey gets in the way.
- OFF-2.1 (manual) THE tray menu SHALL have **Turn Cursora off in <app>** for the app the bubble last opened over, beside "Block Cursora in <app>" (PRI-2.4). It adds that app's process name to the list, and a tick shows it's there; choosing it again removes it.
- OFF-2.2 (manual) Settings → General → Hotkeys SHALL show the list, with each rule editable and removable and **Add** for a new one, as the blocklist does (PRI-2.2). It's empty by default.

## Defaults taken under the go-ahead of 2026-10-28
1. **Off means both hotkeys**, Ask and Snip, pass through. The tray, Settings and `cursora --ask` still work there.
2. **The tray adds the app** the bubble last opened over: right after Ctrl+G opened the bubble by mistake, that's the app.
3. **The keys are sent again** after Cursora steps aside, since Windows and X11 had already kept them from the app.
4. **The list starts empty.**

## Open questions
None.

¹ Approved under the user's go-ahead of 2026-10-28 to design, build and merge updates 1 and 3 to 9, with the user reviewing everything afterwards. The choices made for the user are listed above.
