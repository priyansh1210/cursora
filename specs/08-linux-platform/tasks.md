# 08: Linux platform (tasks)

Status: **In progress**.

- [x] 1. `session.rs`: `detect()`, `current()`, the XWayland switch at the start of `run()`; `app_info` takes `session` from it and gains `launcher` (shared with autostart); `identifies_apps()` per session (tests first). Covers LNX-1.1, part of 4.3.
- [x] 2. `capture()` policy: `Platform::primary()` and `selection_readable()`, `FILE_MANAGERS`, `copy_key(…, primary)`, `Notice::Unreadable`, `Method::Direct` (tests first, on the fake platform). Covers LNX-3.1, 3.2, 3.3, 3.4.
- [x] 3. `x11.rs` and `capture/linux.rs` on X11: `convert()` with INCR, `serve()`, the foreground app, PRIMARY, snapshot, restore, the XFixes sequence number, the clipboard read with `uri_paths()`, XTest, and waiting for every key but Ctrl. Tests against a real X server; CI and `wsl-test.sh` run the Linux tests under `xvfb-run` (tests first). Covers LNX-3.1, 3.2, 4.3, SEL-3.2.
- [x] 4. Wayland: `wl-paste --primary` with its timeout and cap, the unknown app, no hotkey registered, no cursor position, and `--ask` ignored while paused. Covers LNX-2.2, 3.4, 4.2, APP-1.3.
- [x] 5. `bubble/linux.rs`: `handle` through raw-window-handle, `activate` as a pager, `focus` again after the re-place, focus back on close. Closes issues 4 and 18.
- [x] 6. UI and packaging: `useAppInfo()`; About shows the session and, on Linux, the tray note; General on Wayland shows the shortcut steps; Advanced hides the Ctrl+C-unsafe list on Linux; the `unreadable` notice; the autostart file `cursora.desktop`; the .deb recommends `wl-clipboard`; README. Covers LNX-1.1, 2.2, 5.2, 5.3.
- [x] 7. Issue 20: the smallest layout change that lets AT-SPI read the bubble header, the Settings badges and the context row; Windows unchanged.
- [x] 8. End-to-end run in WSL (Xvfb + Openbox: a GTK editor, xterm, a file manager), the spec 01 and 02 Linux checks from issue 11; `/ponytail-review` (2026-10-14; its 2 findings fixed: LNX-3.5 `selection_owner()` for issue 30, the shortcut command's quoting for issue 31).

## Manual checklist
"WSL" is Xvfb + Openbox with the Linux debug build, run on 2026-10-13 and 14; its Wayland session is simulated (a Wayland session type, no compositor). Real Ubuntu, KDE and Fedora are issue 2.

- [x] LNX-2.1 On X11 the Ask hotkey works as on Windows (SEL-1). WSL ✓ Ctrl+G in mousepad, xterm and pcmanfm; nothing while paused. GNOME: issue 2.
- [ ] LNX-2.2 On Wayland, Settings → General shows the `cursora --ask` command with Copy and the GNOME and KDE steps; a shortcut bound that way opens the bubble. WSL ✓ the steps and the command; the quoting checked through bash (issue 31). A real shortcut: issue 2.
- [x] LNX-2.3 `cursora --ask` while Cursora runs starts the capture within 150 ms. WSL ✓ 57 to 81 ms (debug build).
- [ ] LNX-3.4 On Wayland, `wl-paste --primary` gives the selection; without wl-paste the bubble says "No selection could be read in this session." WSL ✓ the hint without wl-paste; a stand-in `wl-paste` is read; one that hangs is cut off (the capture took 304 ms and found nothing selected). A real `wl-paste` with a compositor: issue 2.
- [ ] LNX-4.1 On X11 the bubble opens at the cursor, on the right monitor with several. One monitor ✓ (spec 03's WSL run); several: issue 1.
- [ ] LNX-4.2 On Wayland the bubble opens centered on the primary monitor. WSL ✓ its centre at 960,539 on a 1920×1080 screen. A real Wayland desktop: issue 2.
- [ ] LNX-5.1 The CI-built .deb and .AppImage install and run on Ubuntu 22.04 and 24.04; the AppImage on Fedora. Not run (issues 2, 9).
- [ ] LNX-5.2 "Start with system" writes `~/.config/autostart/cursora.desktop`, and Cursora starts at login. WSL ✓ the file, with the app's path, written and removed again. A login: issue 10.
- [ ] LNX-5.3 The tray icon shows through AppIndicator; About explains the GNOME extension; without a tray, the hotkey and `cursora --settings` reach everything. WSL ✓ About's note; the menu through stalonetray (the XEmbed fallback). AppIndicator on GNOME: issue 2.
- [ ] LNX-6.1 On X11, text from Firefox, Chromium, GNOME Text Editor, LibreOffice Writer, VS Code, GNOME Terminal and Konsole (a running process isn't interrupted); files and folders from Nautilus. WSL ✓ mousepad; xterm with a running process, not interrupted; files from pcmanfm with the clipboard restored; a blocked app's selection isn't read from another app (LNX-3.5). The listed apps: issue 2.
- [ ] LNX-6.2 On Wayland (GNOME and KDE): the shortcut binding, `wl-paste`, the centered bubble and the blocklist warning. Not run (issue 2); the warning shows in WSL.
- Focus on X11 (issues 18 and 4): WSL ✓ 5 of 5, the bubble gets the keyboard and focus goes back on close. GNOME: issue 2.
- Screen readers (issue 20): WSL ✓ AT-SPI reads the bubble header, the source line and the Settings badges. Orca not run. Windows ✓ unchanged: the bubble, Personality and Models match the earlier screenshots, and UI Automation still reads the texts (2026-10-14).
