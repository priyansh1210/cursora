# 11: Auto-update (design)

Status: **Approved** (2026-10-26). Implements [requirements.md](requirements.md). Uses Tauri's updater plugin, which downloads an update, checks its signature, and runs the platform's installer. Cursora adds the schedule, the setting, the skip, the wait for a running answer, the tray item and About. CI publishes each release to the public `cursora-releases` repo.

## Overview
```
start ─→ 30 s later, then every 24 h, WHERE updates.check is on (UPD-1.1, 1.5)
      ─→ GET https://github.com/priyansh1210/cursora-releases/releases/latest/download/latest.json (UPD-1.2)
            GitHub's "latest" is the newest published release that isn't a pre-release, so drafts and rc tags never show (UPD-1.6)
      ─→ the plugin compares its version with ours; offer() also drops pre-release versions and the skipped one
      ─→ Available { version, notes } → tray item "Update to version X" + About (UPD-2.1); nothing pops up
            failure → kept for About only, retried at the next check (UPD-1.4)
user picks Update (tray or About)
      ─→ About shows the notes; an answer still coming in? wait until it ends or is stopped (UPD-2.4)
      ─→ AppImage or Windows: download with progress → signature checked against the built-in public key (UPD-2.3) → install
            Windows: msiexec /passive asks once for admin rights; Cursora exits while it runs, and the installer starts it again
            AppImage: the file is replaced, then app.restart()
         .deb / .rpm: open the release page instead (UPD-2.6)
```

## Components
| File | Role |
|---|---|
| `src-tauri/src/update.rs` (new) | The schedule, `check`, `offer()`, the state, `install`, the commands below. |
| `src-tauri/src/settings.rs` | `updates: UpdateSettings { check: bool (default true), skipped: Option<String> }`, with serde defaults so older settings files load. |
| `src-tauri/src/tray.rs` | "Update to version X" as the first item while an update is available (UPD-2.1). |
| `src-tauri/src/chat.rs` | `answering(app) -> bool`: any slot whose last turn is still `Streaming`. |
| `src/Settings.tsx` | General → Behavior: "Check for updates automatically". About: the version, **Check for updates**, the result, and, when available, the notes with **Update now** and **Skip this version** (UPD-1.3, 2.2, 2.5). |
| `src/Setup.tsx` | The last step, now "Startup and updates", gets "Check for updates automatically", ticked (UPD-1.5). |
| `src-tauri/tauri.conf.json` | `bundle.createUpdaterArtifacts: true`; `bundle.targets: ["msi", "deb", "rpm", "appimage"]` (no NSIS, decision 3); `plugins.updater: { pubkey, endpoints: [the URL above], windows: { installMode: "passive" } }`. |
| `.github/workflows/ci.yml` | The release job signs and publishes to `cursora-releases` (UPD-3). |
| `scripts/release-version.mjs` (new) | Refuses a tag that doesn't match the app's version (UPD-3.4). |

## Data
```rust
pub struct UpdateSettings { pub check: bool, pub skipped: Option<String> }  // in Settings, default { true, None }

enum Status {                    // update.rs, in a Mutex; sent to the UI as JSON
    Unknown,                     // not checked yet
    UpToDate { checked: u64 },
    Available { version: String, notes: String, checked: u64 },
    Failed { reason: String, checked: u64 },
    Downloading { done: u64, total: Option<u64> },
    Waiting,                     // Update chosen, an answer is still coming in (UPD-2.4)
}
```
The plugin's `Update` object from the last check is kept next to the status, so installing doesn't fetch `latest.json` again.

## Key mechanics

### Checking (UPD-1)
- `start(app)` runs at startup: a tokio task that sleeps 30 s (so the start, the hotkey and the bubble aren't slowed), then loops: if `settings.updates.check`, `check()`; sleep 24 h.
- `check()` calls `app.updater()?.check().await`. The plugin uses its own HTTPS client, and sends nothing but the request for that one URL.
- `offer(current, candidate, skipped) -> bool`: true only if the candidate is a valid semver newer than current, has no pre-release part, and isn't the skipped version. The plugin already compares versions; `offer` adds the last two rules (UPD-1.6, 2.5).
- Every result goes to the state, the tray is rebuilt (only the update item changes), and `update-status` is emitted to the Settings window.
- **Check for updates** in About runs `check()` at once, whatever the setting.

### Installing (UPD-2)
- `update_install`: if `chat::answering(app)`, the status becomes `Waiting`, and a 1 s poll goes on once the answer has ended or been stopped. Stopping is still the user's choice.
- On Windows and in an AppImage: `update.download_and_install(on_chunk, on_finished)`. Progress goes to About as `Downloading`. The plugin checks the minisign signature against `pubkey` before installing anything. A mismatch returns an error, the status becomes `Failed { reason }`, and nothing is installed (UPD-2.3).
- **Windows:** the plugin runs the .msi with `/passive`. Since the .msi installs for all users, Windows shows its admin prompt once. The plugin exits Cursora while the installer runs, and the installer starts it again. The settings, history and keys stay in AppData and the keychain. The Run entry and the Run as administrator task name the same `Program Files` path, so they keep working (UPD-2.4).
- **AppImage:** the plugin replaces the file, then `app.restart()`.
- **.deb and .rpm:** no download. `update_install` opens `https://github.com/priyansh1210/cursora-releases/releases/tag/v<version>` with the opener (UPD-2.6). Detected by the AppImage env being absent on Linux (`app.env().appimage`, as `launcher()` already does).
- `update_skip(version)` saves `updates.skipped`, so the tray item and About's notice go away until a newer version (UPD-2.5).

### Publishing (UPD-3)
In the release job, for each OS:
1. `node scripts/release-version.mjs "$GITHUB_REF_NAME"` fails unless the tag is `v<version>` or `v<version>-<suffix>`, where `<version>` is `tauri.conf.json`'s (UPD-3.4).
2. `tauri-apps/tauri-action` with:
   - `owner: priyansh1210`, `repo: cursora-releases`, `releaseCommitish: main`. Without that last one, the action tags the private repo's commit, which doesn't exist in `cursora-releases`.
   - `releaseDraft: true`, and `prerelease: ${{ contains(github.ref_name, '-') }}` (UPD-3.2).
   - `env: GITHUB_TOKEN: ${{ secrets.RELEASES_TOKEN }}`, `TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}`.
   - `uploadUpdaterJson` stays on (the default). Each OS's job adds its platform to the release's `latest.json` (`windows-x86_64` from the .msi, `linux-x86_64` from the AppImage).
3. The developer checks the draft and publishes it. Only then does `releases/latest` point to it. The notes come from `release-notes.md` through tauri-action's `releaseBody`, so they're in `latest.json` too.

**Developer setup, once** (UPD-3.3):
1. In a normal PowerShell window (it asks for a password, so not through Claude): `npx tauri signer generate -w "$env:USERPROFILE\.tauri\cursora-updater.key"`. Choose a strong password.
2. Back up the key file and its password together, offline (a password manager, or a USB stick kept apart).
3. Add two secrets to the `cursora` repo: `TAURI_SIGNING_PRIVATE_KEY` (the content of `cursora-updater.key`) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
4. The public key (`cursora-updater.key.pub`) goes into `tauri.conf.json` → `plugins.updater.pubkey`. It's public, so Claude can copy it in.

**Release checklist** (each version):
1. Bump the version in `tauri.conf.json`, `Cargo.toml`, `Cargo.lock`, `package.json` and `package-lock.json`, and write `release-notes.md` (what's new, in plain words).
2. Merge, then tag `v<version>` (or `v<version>-rc.N` for a test build).
3. Check the draft in `cursora-releases`: every file has its `.sig`, and `latest.json` has the notes.
4. Confirm the signing key backup is still readable.
5. Publish.
6. Once a year, replace `RELEASES_TOKEN`. It was created without an expiry date (GitHub wouldn't keep one), so this is the safety net.

## Commands and events
| Command | Returns | Does |
|---|---|---|
| `update_status` | `Status` | The current state, for About when it opens. |
| `update_check` | `Status` | Checks now (About's button). |
| `update_install` | `()` | Waits for an answer if needed, then downloads and installs, or opens the release page on .deb/.rpm. |
| `update_skip(version)` | `()` | Saves `updates.skipped`. |

Event `update-status` (to the Settings window) carries every change, including download progress.

## Changes to earlier specs' code
- **Tray (APP-1.2):** a new first item while an update is available. Its text names the version.
- **Setup (APP-6.1):** one more checkbox on the last step.
- **Packaging:** no NSIS `setup.exe` any more (decision 3). This closes issue 47.

## Dependencies
- `tauri-plugin-updater = "2"`: Tauri's own; signature checking with minisign, and the Windows and AppImage installers.
- `minisign-verify` (dev only): the plugin's own signature library, for UPD-2.3's test.
- No JS package: the webview only calls Cursora's own commands, so no updater permission is given to any window.

## Decisions
1. **Tauri's updater plugin**, not our own download code: it already checks signatures and drives the .msi and AppImage installs. Rejected: a hand-written downloader (we'd own the signature checking).
2. **A static `latest.json` in the public releases repo**, reached through GitHub's "latest release" link: no server of our own. GitHub skips drafts and pre-releases for us. Rejected: GitHub's API (rate limits per IP, and it needs parsing).
3. **Windows ships only the .msi.** Run as administrator works only from Program Files (APP-3.8). The NSIS `setup.exe` installs per user, and was never inspected (issue 47). One installer also means one entry in `latest.json`. Rejected: shipping both (two Windows installers to choose from, and in the setup.exe Run as administrator is greyed out).
4. **Everything runs in Rust**, and the webview gets plain commands: no updater permissions in any window's capabilities.
5. **No notification**, only the tray and About (the user's choice, 2026-10-26).
6. **30 s after start, then every 24 h**, not at once: the start stays quick, and an offline start doesn't log a failure straight away.
7. **`requireSignedVersion` on from the first release**: it protects against downgrades, and there are no older releases that lack a signed version (see "Changes during implementation").

## Known limits
- `shortcut:` users of builds without the updater (rc.1, rc.2) never get updates. Fine: nobody outside has them.
- `shortcut:` no retry before the next day after a failed check; About's button retries at once. Upgrade if users report missed updates.
- `shortcut:` the release page link for .deb/.rpm assumes the tag is `v<version>`, as UPD-3.4 enforces.

## Changes during implementation
- **`offer()` checks only the pre-release suffix and the skipped version.** The plugin already offers nothing that isn't newer, so the `semver` dependency isn't needed.
- **`requireSignedVersion: true`** (decision 7, new): with it, each signature names the version it was made for. Then a tampered `latest.json` can't pair a new version number with an older, genuine release, forcing a downgrade. `tauri build` writes the version into signatures since CLI 2.12.0, and the project uses 2.12.2.
- **No check before setup is done** (UPD-1.5): a new user may spend more than the first 30 s in setup. The loop waits a minute at a time until setup is finished, so the box on its last page always comes first.
- `Status` has no check time. `Available` carries `installable`, so About's button can say "Open the download page" on .deb and .rpm.
- **The tray item starts the update at once**, and opens About to show the notes and the progress.
- **Notes come from `release-notes.md`** (UPD-3.2 amended). v0.1.0's `latest.json` had empty notes: tauri-action writes it while building, so text added to the draft afterwards never reaches it.
- **CI runs once per change:** on PRs, on `main` and on `v*` tags. Branch pushes no longer run it too, since minutes are limited in a private repo.
- Setup's last page is now titled "Startup and updates".
- **UPD-2.3's test** runs the plugin's own verification steps (`minisign-verify`, its dependency): the plugin's published throwaway signature passes with its own key, and is refused by the key in `tauri.conf.json`. Tauri's mock app (feature `test`) can't be used: the test program then won't start on Windows (`STATUS_ENTRYPOINT_NOT_FOUND`).

## Test strategy
| Criterion | How |
|---|---|
| UPD-1.2 | Rust test reading `tauri.conf.json`: exactly one endpoint, HTTPS, the `cursora-releases` latest-release URL, and a non-empty `pubkey`. |
| UPD-1.6 | Rust tests of `offer()`: newer is offered; older, equal, pre-release (`0.2.0-rc.1`) and skipped are not. |
| UPD-2.3 | Rust test of the plugin's verify steps (see "Changes during implementation"): another key's signature is refused by the configured key, which is checked to be a real minisign key. |
| UPD-3.4 | `scripts/release-version.test.mjs`: `v0.2.0` and `v0.2.0-rc.1` pass for 0.2.0; `v0.1.9`, `0.2.0` and `v0.2.0.1` fail. |
| Manual | UPD-1.1, 1.3 to 1.5, 2.1, 2.2, 2.4 to 2.6, 3.1 to 3.3: with two releases (an older installed .msi, a newer published one): the tray item, About, the notes, the admin prompt, the restart with settings intact, the wait during a streaming answer, a skip, the setup checkbox, off means no request (checked with a firewall log or proxy), offline means About says so; the AppImage in WSL; the .deb shows the link. |
