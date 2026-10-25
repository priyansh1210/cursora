# 11: Auto-update (tasks)

- [x] 1. `offer()` and its tests, the `updates` setting with defaults, `chat::answering`. Covers UPD-1.6, UPD-2.5.
- [x] 2. `update.rs`: the plugin, the schedule, `check`, the state, the commands and event, `install` (wait, download, restart, or the release page on .deb/.rpm). Covers UPD-1.1, 1.4, 2.2 to 2.4, 2.6.
- [x] 3. The tray item, About (version, Check for updates, notes, Update now, Skip), General's checkbox, setup's last step. Covers UPD-1.3, 1.5, 2.1, 2.5.
- [ ] 4. `tauri.conf.json`: updater artifacts, the .msi only on Windows, `plugins.updater` with the public key and the one endpoint; its test. Covers UPD-1.2.
- [ ] 5. A bad signature is refused: the test. Covers UPD-2.3.
- [x] 6. `scripts/release-version.mjs` and its test; the release job signs and publishes a draft to `cursora-releases`. Covers UPD-3.1 to 3.4.
- [ ] 7. Two real releases end to end, then `/ponytail-review`.

## Manual checklist
- [ ] UPD-1.1 Checks 30 s after start, then daily; the start, hotkey and bubble aren't slowed. Windows ☐ Linux ☐
- [ ] UPD-1.3 About shows the version, Check for updates and the result. Windows ☐ Linux ☐
- [ ] UPD-1.4 Offline, the check fails quietly; About says why. Windows ☐ Linux ☐
- [ ] UPD-1.5 The setting in General and ticked on setup's last step; off means no request. Windows ☐ Linux ☐
- [ ] UPD-2.1 The tray item and About's notice, nothing else. Windows ☐ Linux ☐
- [ ] UPD-2.2 Notes, progress, the admin prompt, the install. Windows ☐ Linux (AppImage) ☐
- [ ] UPD-2.4 Waits for a streaming answer; restarts with settings, history, keys, Start with system and Run as administrator intact. Windows ☐ Linux ☐
- [ ] UPD-2.5 A skipped version isn't offered again; a later one is. Windows ☐
- [ ] UPD-2.6 The .deb opens the release page. Linux ☐
- [ ] UPD-3.1 / 3.2 A tag makes a draft in `cursora-releases`; an rc tag stays a pre-release. ☐
- [ ] UPD-3.3 The signing key is only in the secrets and the offline backup. ☐
