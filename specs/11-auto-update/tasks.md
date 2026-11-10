# 11: Auto-update (tasks)

- [x] 1. `offer()` and its tests, the `updates` setting with defaults, `chat::answering`. Covers UPD-1.6, UPD-2.5.
- [x] 2. `update.rs`: the plugin, the schedule, `check`, the state, the commands and event, `install` (wait, download, restart, or the release page on .deb/.rpm). Covers UPD-1.1, 1.4, 2.2 to 2.4, 2.6.
- [x] 3. The tray item, About (version, Check for updates, notes, Update now, Skip), General's checkbox, setup's last step. Covers UPD-1.3, 1.5, 2.1, 2.5.
- [x] 4. `tauri.conf.json`: updater artifacts, the .msi only on Windows, `plugins.updater` with the public key and the one endpoint; its test. Covers UPD-1.2.
- [x] 5. A bad signature is refused: the test. Covers UPD-2.3.
- [x] 6. `scripts/release-version.mjs` and its test; the release job signs and publishes a draft to `cursora-releases`. Covers UPD-3.1 to 3.4.
- [ ] 7. Two real releases end to end, then `/ponytail-review`. The releases are done (2026-10-26: v0.1.0 then v0.1.1, below); the review is still to do. Done 2026-11-10: one finding fixed, R104 (a double click installed twice).

## Manual checklist
- [ ] UPD-1.1 Checks 30 s after start, then daily; the start, hotkey and bubble aren't slowed. Windows ☐ Linux ☐
- [ ] UPD-1.3 About shows the version, Check for updates and the result. Windows ✓ (2026-10-26: "Couldn't reach the update server…" before anything was published; "Version 0.1.1 is available." with its notes once it was) Linux ☐
- [ ] UPD-1.4 Offline, the check fails quietly; About says why. Windows ✓ by the user (2026-10-29, the installed 0.2.0: Wi-Fi off, About → Check for updates failed quietly with the reason) Linux ☐
- [ ] UPD-1.5 The setting in General and ticked on setup's last step; off means no request. Windows ✓ the box in General (screenshot) ☐ setup's last page ☐ off means no request Linux ☐
- [ ] UPD-2.1 The tray item and About's notice, nothing else. Windows ✓ About's notice ☐ the tray item (not confirmed) Linux ☐
- [ ] UPD-2.2 Notes, progress, the admin prompt, the install. Windows ✓ (2026-10-26, the laptop: the installed 0.1.0 showed 0.1.1's notes, Update now asked for admin permission, installed, and restarted as 0.1.1) Linux (AppImage) ☐
- [ ] UPD-2.4 Waits for a streaming answer; restarts with settings, history, keys, Start with system and Run as administrator intact. Windows ✓ intact after 0.1.0 → 0.1.1 (settings, both providers and the default model, history, the quoted Run entry, the admin task on the same path) ☐ waiting for an answer Linux ☐
- [ ] UPD-2.5 A skipped version isn't offered again; a later one is. Windows ☐
- [ ] UPD-2.6 The .deb opens the release page. Linux ☐
- [x] UPD-3.1 / 3.2 A tag makes a draft in `cursora-releases`; an rc tag stays a pre-release. ✓ v0.1.0 and v0.1.1 each made one draft there, with the .msi, .deb, .rpm, AppImage, a `.sig` for each, and `latest.json` (Windows → .msi, Linux → AppImage); every signature names its version; v0.1.1's notes came from `release-notes.md`; published by hand, then `releases/latest/download/latest.json` served them signed out. (An rc tag's pre-release flag is set by `prerelease: contains(ref_name, '-')`, not yet run.)
- [ ] UPD-3.3 The signing key is only in the secrets and the offline backup. ✓ the secrets (names checked through the API; the private file was never read here) ☐ the offline backup, which only the user can confirm
