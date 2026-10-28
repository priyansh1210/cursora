# 21: Turn Cursora off per app (tasks)

Status: **In progress**.

- [x] 1. `Hotkeys.off_in`; `chord()` and the check in `on_press`; `pass_through`; `Platform::send_chord` on Windows, X11 and the fake, with tests. Covers OFF-1.1, OFF-1.2, OFF-1.3.
- [x] 2. The tray item and the list in Settings → General; the manual. Covers OFF-2.1, OFF-2.2.
- [ ] 3. All checks; by hand on Windows; then `/ponytail-review`. 2026-10-28: by hand on Windows below, with the test copy and Ctrl+H as its Ask key (the installed copy holds Ctrl+G).

## Manual checklist
- [ ] OFF-1.2 (2026-10-29, the stronger PC's X11 check: with xterm on the list, Ctrl+G opened no bubble but xterm never got it either; global-hotkey hadn't sent its ungrab yet, so the keys pressed again met the old grab. Fixed in `pass_through`; checked again below.) With the app on the list, its own action for the hotkey happens (Notepad's Go to line for Ctrl+G); elsewhere the bubble opens. Windows ✓ (2026-10-28, test copy with Ask on Ctrl+H: with Notepad.exe on the list, Ctrl+H in Notepad opened Notepad's Replace and no bubble, logged "off in Notepad.exe, passed on"; right after, Windows refused Ctrl+H to another program (1409, already registered), so Cursora had taken it back. With the list empty, Ctrl+H opened the bubble and not Replace) Linux X11 ☐ (issue 67)
- [ ] OFF-2.1 The tray item names the app the bubble last opened over, adds it with a tick, and removes it. Windows ✓ by the user (2026-10-29, test copy: "Turn Cursora off in Notepad" after a bubble over Notepad; with it ticked, Ctrl+G in Notepad opened no bubble; clicked again, the bubble opened) Linux X11 ☐
- [ ] OFF-2.2 General → Hotkeys lists the rules; edit, remove and Add work. Windows ✓ shows "Hotkeys off in these apps" with the Notepad.exe rule, × and Add (2026-10-28) ☐ editing by hand Linux X11 ☐
