# 11: Auto-update (UPD)

## Scope
- **In:** checking for a newer Cursora, telling the user, and downloading, verifying and installing it: the .msi on Windows, the AppImage on Linux. Publishing each release to a small public releases repo, while the code stays private.
- **Out:**
  - A beta or pre-release channel for users
  - Partial (delta) updates
  - macOS
  - Updating the .deb and .rpm in place: their users are pointed to the download page (UPD-2.6)
  - Updating models or skills

The check downloads one small file from GitHub. It sends no user content, settings or identifiers, and it can be turned off before it ever runs (UPD-1.5). Constitution §3 allows it by the amendment of 2026-10-26.

## Requirements

### UPD-1 Checking
User story: As a user, I want to know when a newer Cursora is out, without being interrupted or tracked.
- UPD-1.1 (manual) WHERE "Check for updates automatically" is on (the default) THE SYSTEM SHALL check once shortly after it starts, without delaying the start, the hotkey or the bubble, and then every 24 hours while it runs.
- UPD-1.2 (auto) THE check SHALL only download the release file (`latest.json`) of the public releases repo over HTTPS. It SHALL send nothing about the user: no content, settings or IDs, beyond what any download sends (the IP address, and a User-Agent with Cursora's version).
- UPD-1.3 (manual) Settings → About SHALL show the current version, a **Check for updates** button, and the last result: "Cursora is up to date", "Version X is available", or why the check failed.
- UPD-1.4 (manual) WHEN a check fails (offline, GitHub unreachable) THE SYSTEM SHALL say so only in About, and try again at the next check.
- UPD-1.5 (manual) THE setting SHALL be in Settings → General and on setup's last step (APP-6.1), ticked, so it can be turned off before Cursora ever checks.
- UPD-1.6 (auto) Drafts and pre-releases (such as `v0.1.0-rc.2`) SHALL never be offered; only a newer published release.

### UPD-2 Updating
User story: As a user, I want to update in one click, safely, without losing what I'm doing.
- UPD-2.1 (manual) WHEN a newer version is found THE SYSTEM SHALL say so without covering the user's work: an **Update to version X** item at the top of the tray menu, and the notice in About. No pop-up and no Windows notification.
- UPD-2.2 (manual) WHEN the user chooses to update THE SYSTEM SHALL show what's new (the release notes, written by hand in each release, UPD-3.2), download the update with its progress, verify it (UPD-2.3), and install it. On Windows, Windows asks once for administrator permission, since Cursora is installed for all users in Program Files.
- UPD-2.3 (auto) An update whose signature doesn't match the public key built into Cursora SHALL NOT be installed. The current version stays, and About says why.
- UPD-2.4 (manual) WHILE an answer is still coming THE SYSTEM SHALL wait for it to end (or for the user to stop it) before restarting. After installing, Cursora SHALL start again by itself, with its settings, history, API keys, "Start with system" and Run as administrator unchanged.
- UPD-2.5 (manual) The user SHALL be able to skip a version: it isn't offered again, but a later one is.
- UPD-2.6 (manual) On Linux the AppImage updates itself as above. The .deb and .rpm can't: there, the update item opens the release's download page instead.

### UPD-3 Publishing
User story: As the developer, I want each release built, signed and published the same way, and only when I say so.
- UPD-3.1 (manual) WHEN a `v*` tag is pushed THE SYSTEM (CI) SHALL build the installers and their signed update files, and put them with `latest.json` in a draft release of the public releases repo. The source code stays in the private repo.
- UPD-3.2 (manual) A release SHALL reach users only when the developer publishes its draft. Its notes are written by hand in `release-notes.md` with the version bump, and CI puts them into both the release and `latest.json`. (Amended 2026-10-26: `latest.json` is written during the build, so notes typed into the draft afterwards never reached Cursora.) A tag with a pre-release suffix (`-rc.N`) SHALL stay a pre-release.
- UPD-3.3 (manual) THE update signing key SHALL exist only as a GitHub Actions secret and in an offline backup the developer keeps. It is never committed. Without it, existing users can't be updated, so checking the backup is part of every release.
- UPD-3.4 (auto) THE version offered in `latest.json` SHALL be the app's own version (`tauri.conf.json`), and CI SHALL refuse to publish a full release whose tag isn't `v<that version>`.

## Decisions (2026-10-26, the user)
1. The check is **on by default**, with a ticked box on setup's last step (UPD-1.1, UPD-1.5). Constitution §3 is amended to allow it.
2. **The releases repo** is `cursora-releases`, public, on the developer's GitHub account. The developer creates it, and a fine-grained token that can only write to it. The token is kept as a secret of the private repo, and CI uses it to publish (UPD-3.1).
3. **Announcing:** the tray item and About only (UPD-2.1).
4. **Release notes** are written by hand, in `release-notes.md` with each version bump (UPD-3.2, amended 2026-10-26).

## Open questions
None.
