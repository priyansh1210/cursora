# 01: App shell (tasks)

Status: **In progress**. Code complete, including Run as administrator (tasks 10–13, added 2026-10-14); the manual checklist is open.

- [x] 1. Scaffold Tauri v2 + React/TS, plus CI with `trace` (milestone 0). Covers APP-8.1 and APP-8.2.
- [x] 2. `settings.rs`: struct, load/save (atomic), backup on corruption. Tests first. Covers APP-4.1, APP-4.2, APP-4.3, APP-4.4.
- [x] 3. `cli.rs` + single-instance plugin + action handling. Covers APP-2.1, APP-2.2, APP-2.3, APP-2.4.
- [x] 4. No main window; tray menu, paused icon, Quit. Covers APP-1.1, APP-1.2, APP-1.3, APP-1.4.
- [x] 5. Commands `get_settings`/`set_settings`/`app_info`, `update_settings()`, event `settings-changed`, autostart. Covers APP-3.1, APP-4.1.
- [x] 6. Logging plugin + the log-hygiene test. Covers APP-7.1, APP-7.2.
- [x] 7. Settings window: tab frame, General tab (hotkeys, autostart, theme, pause, re-run setup), About tab. Covers APP-5.1, APP-5.2, APP-5.3, APP-6.3.
- [x] 8. Setup window with the Hotkeys and Start-with-system steps. Covers APP-3.2, part of APP-6.1.
- [x] 9. `/ponytail-review` of the spec-01 code. 3 findings, all fixed (see design.md "Changes during implementation").
- [x] 10. `admin.rs`: `startup()` and `task_xml()`, tests first. Covers APP-3.6, APP-3.7.
- [x] 11. The Windows side: the start-up step in `run()` (run the task, else the UAC prompt, then exit), `--after <pid>`, creating and deleting the task, `canRunAsAdmin`. Covers APP-3.3.
- [x] 12. The `runAsAdmin` setting and its General checkbox: turning on asks Windows and restarts, turning off says when it applies. The SEL-6.1 message names the setting. Covers APP-3.4, APP-3.5, APP-5.2, SEL-6.1.
- [x] 13. `/ponytail-review` of tasks 10–12. 3 findings, all fixed (see design.md "Changes during implementation").

## Manual checklist
"✓ (auto-verified)" marks a check run automatically with process and window inspection on Windows. The rest needs a human: clicking the tray, logging in again, Linux.
- [ ] APP-1.1 Starts with only a tray icon. Windows ✓ (auto-verified: no main window, about 35 MB) ✓ tray icon (auto-verified 2026-10-13) · X11 ☐ · Wayland ☐
- [ ] APP-1.2 The tray menu has every item. Windows ✓ (auto-verified 2026-10-13): every item; Personality and Local-only follow the setting; "Stop the model" unloads the built-in model X11 ✓ every item; Open Settings, Pause and "Stop the model" work (auto-verified 2026-10-14 in WSL, stalonetray as the tray host; real Ubuntu: issue 2) Wayland ☐
- [ ] APP-1.3 Pause swaps the icon (hotkey part verified after spec 02). Windows ✓ (auto-verified 2026-10-13): gray icon, check mark, Ctrl+G ignored X11 ✓ paused and resumed from the tray, Ctrl+G ignored while paused (auto-verified 2026-10-14 in WSL; the icon wasn't looked at) ☐ the icon
- [x] APP-1.4 Quit exits cleanly with no process left. Windows ✓ (auto-verified 2026-10-13): from the tray with the engine running X11 ✓ from the tray and with `cursora --quit`, no process left (auto-verified 2026-10-14 in WSL)
- [x] APP-2.2 A second `cursora --settings` opens Settings in the first instance. Windows ✓ (auto-verified) · X11 ✓ (auto-verified in WSL: every Linux Settings check from spec 04 on opens it this way)
- [ ] APP-2.3 `cursora --settings` with no instance running starts the app and opens Settings. Windows ✓ `--settings` and `--ask` (auto-verified 2026-10-13) X11 ☐
- [ ] APP-2.4 Relaunching with no arguments opens Settings. Windows ✓ (auto-verified) · X11 ☐
- [ ] APP-3.1 "Start with system" survives a logout/login. Windows: per-user Run entry written ✓ (auto-verified) · login ✓ (2026-10-14, after a restart: started 86 s after boot from the Run entry, not elevated, only the tray icon plus the debug build's console, issue 13) · X11 ☐
- [x] APP-3.2 Setup asks about starting with the system (default yes). Windows ✓ (the user went through setup)
- [ ] APP-3.3 With Run as administrator on, Cursora starts as administrator at login and when opened again, with no UAC prompt; with the task deleted by hand, it asks once instead. Windows ✓ opened again (2026-10-15: a normal launch ran the task and exited with code 0, no prompt; the running admin copy opened Settings) ✓ reads admin apps (an elevated Notepad: `Direct, 156 chars`) ✓ login (2026-10-15, after a restart: the Run entry's normal copy ran the task and exited; one copy, started by Task Scheduler 83 s after boot, elevated, no prompt, no console left open) ☐ task deleted by hand (not run: the user skipped it, issue 42)
- [x] APP-3.4 Turning it on asks once and restarts as administrator; No leaves it off with "Windows didn't allow it." Windows ✓ (2026-10-15, the user: No showed the message and left the box off; Yes restarted Cursora elevated with Settings reopened and the box ticked, and the task was created)
- [x] APP-3.5 Turning it off removes the task; the next start is a normal app. Windows ✓ (2026-10-15, the user: unticking showed "Cursora runs as a normal app from its next start.", and the task was gone; the next start was a normal app)
- [ ] APP-5.1 / 5.2 / 5.3 The tabs and their contents are present, and the folder buttons open the right folders. Windows ✓ (auto-verified 2026-10-13) X11 ☐
- [ ] APP-6.1 The full setup flow (after specs 03, 04 and 05). Windows ☐ X11 ☐
- [x] APP-6.3 Setup choices can be edited in Settings, and "Re-run setup" works. Windows ✓ (auto-verified 2026-10-13)
- [x] APP-7.1 More than 5 MB of logs leaves at most 5 files of about 1 MB. Windows ✓ (auto-verified 2026-10-13): 5 files (it kept 6 before the fix)
- [ ] APP-8.2 CI is green on both OSes; a `v*` tag produces draft-release installers. ☐ (needs a GitHub remote)
