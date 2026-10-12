# 01: App shell (tasks)

Status: **In progress**. Code complete; the manual checklist is open.

- [x] 1. Scaffold Tauri v2 + React/TS, plus CI with `trace` (milestone 0). Covers APP-8.1 and APP-8.2.
- [x] 2. `settings.rs`: struct, load/save (atomic), backup on corruption. Tests first. Covers APP-4.1, APP-4.2, APP-4.3, APP-4.4.
- [x] 3. `cli.rs` + single-instance plugin + action handling. Covers APP-2.1, APP-2.2, APP-2.3, APP-2.4.
- [x] 4. No main window; tray menu, paused icon, Quit. Covers APP-1.1, APP-1.2, APP-1.3, APP-1.4.
- [x] 5. Commands `get_settings`/`set_settings`/`app_info`, `update_settings()`, event `settings-changed`, autostart. Covers APP-3.1, APP-4.1.
- [x] 6. Logging plugin + the log-hygiene test. Covers APP-7.1, APP-7.2.
- [x] 7. Settings window: tab frame, General tab (hotkeys, autostart, theme, pause, re-run setup), About tab. Covers APP-5.1, APP-5.2, APP-5.3, APP-6.3.
- [x] 8. Setup window with the Hotkeys and Start-with-system steps. Covers APP-3.2, part of APP-6.1.
- [x] 9. `/ponytail-review` of the spec-01 code. 3 findings, all fixed (see design.md "Changes during implementation").

## Manual checklist
"✓ (auto-verified)" marks a check run automatically with process and window inspection on Windows. The rest needs a human: clicking the tray, logging in again, Linux.
- [ ] APP-1.1 Starts with only a tray icon. Windows ✓ (auto-verified: no main window, about 35 MB) ✓ tray icon (auto-verified 2026-10-13) · X11 ☐ · Wayland ☐
- [ ] APP-1.2 The tray menu has every item. Windows ✓ (auto-verified 2026-10-13): every item; Personality and Local-only follow the setting; "Stop the model" unloads the built-in model X11 ☐ Wayland ☐
- [ ] APP-1.3 Pause swaps the icon (hotkey part verified after spec 02). Windows ✓ (auto-verified 2026-10-13): gray icon, check mark, Ctrl+G ignored X11 ☐
- [ ] APP-1.4 Quit exits cleanly with no process left. Windows ✓ (auto-verified 2026-10-13): from the tray with the engine running X11 ☐
- [ ] APP-2.2 A second `cursora --settings` opens Settings in the first instance. Windows ✓ (auto-verified) · X11 ☐
- [ ] APP-2.3 `cursora --settings` with no instance running starts the app and opens Settings. Windows ✓ `--settings` and `--ask` (auto-verified 2026-10-13) X11 ☐
- [ ] APP-2.4 Relaunching with no arguments opens Settings. Windows ✓ (auto-verified) · X11 ☐
- [ ] APP-3.1 "Start with system" survives a logout/login. Windows: per-user Run entry written ✓ (auto-verified) ☐ login · X11 ☐
- [x] APP-3.2 Setup asks about starting with the system (default yes). Windows ✓ (the user went through setup)
- [ ] APP-5.1 / 5.2 / 5.3 The tabs and their contents are present, and the folder buttons open the right folders. Windows ✓ (auto-verified 2026-10-13) X11 ☐
- [ ] APP-6.1 The full setup flow (after specs 03, 04 and 05). Windows ☐ X11 ☐
- [x] APP-6.3 Setup choices can be edited in Settings, and "Re-run setup" works. Windows ✓ (auto-verified 2026-10-13)
- [x] APP-7.1 More than 5 MB of logs leaves at most 5 files of about 1 MB. Windows ✓ (auto-verified 2026-10-13): 5 files (it kept 6 before the fix)
- [ ] APP-8.2 CI is green on both OSes; a `v*` tag produces draft-release installers. ☐ (needs a GitHub remote)
