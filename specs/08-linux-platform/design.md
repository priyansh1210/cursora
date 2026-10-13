# 08: Linux platform (design)

Status: **Approved** (2026-10-13). Implements [requirements.md](requirements.md). Specs 02 and 03 built the Windows side behind `Platform` (capture) and `bubble::os` (focus); this spec fills in the Linux side. It also closes issues 3, 4 and 18, and takes on issue 20.

## Overview
```
startup ─→ session::current(): x11 | wayland | unknown (LNX-1.1)
           └─ wayland with XWayland (DISPLAY set): GDK_BACKEND=x11, so the bubble can be placed, kept on top and focused

X11      Ctrl+G (global-hotkey, XGrabKey) ─→ capture(LinuxPlatform with an X11 connection)
           foreground: _NET_ACTIVE_WINDOW → _NET_WM_PID → /proc → process name; _NET_WM_NAME title (LNX-4.3)
           file manager? ─ yes → XTest Ctrl+C → CLIPBOARD text/uri-list → files; clipboard served back (LNX-3.2)
                         └ no  → PRIMARY; no keys, ever (LNX-3.1, 3.3)
         bubble at the cursor; focus by _NET_ACTIVE_WINDOW as a pager; back to the app on close (issues 18, 4)

Wayland  desktop shortcut → `cursora --ask` → single instance → capture(LinuxPlatform without X11)
           foreground unknown (LNX-4.3, PRI-2.5); text from `wl-paste --primary`; no keys
           no wl-paste → general chat + "No selection could be read in this session." (LNX-3.4)
         bubble centered on the primary monitor (LNX-4.2, BUB-1.4)
```
- **The decisions stay in the shared `capture()`**, tested with the fake platform. Linux adds two facts that the platform reports: it reads PRIMARY, so keys are only for file managers; and whether it can read a selection at all.
- **On Linux, keys are only ever sent to file managers.** Every X11 app in the compatibility matrix (LNX-6.1) puts its selection in PRIMARY. So a copy fallback would only run when nothing is selected. There it can copy a whole line (VS Code copies the current line) or interrupt a terminal. Terminals are therefore safe by construction, not by a list that can miss one (LNX-3.3).

## Components

| File | Job |
|---|---|
| `src-tauri/src/session.rs` (new) | `Session` (`windows`, `x11`, `wayland`, `unknown`), `detect()` (pure), `current()` (read once), and the XWayland switch. |
| `src-tauri/src/x11.rs` (new, Linux) | A small X11 client over `x11rb`, shared by capture and the bubble: connect and atoms, read a property, the active window, `activate()`, `convert()` (ask a selection's owner for one target, INCR included) and `serve()` (own a selection and hand out saved targets until another app takes it). |
| `src-tauri/src/capture/linux.rs` (replaces `other.rs`) | `LinuxPlatform`: the X11 version of every `Platform` method, and the Wayland one (`wl-paste`; everything else empty). `uri_paths()` (pure). |
| `src-tauri/src/capture/mod.rs` | `FILE_MANAGERS`; `Platform::primary()` and `selection_readable()`; `copy_key(…, primary)`; `Notice::Unreadable`; `Method::Uia` becomes `Method::Direct`; `identifies_apps()` per session. |
| `src-tauri/src/bubble/linux.rs` (replaces `other.rs`) | `foreground`, `handle` (the bubble's X window, through raw-window-handle), `activate`, `focus`. |
| `src-tauri/src/hotkey.rs` | No hotkey on Wayland; no cursor position on Wayland. |
| `src-tauri/src/lib.rs` | The XWayland switch first thing in `run()`; `app_info` takes `session` from `session::current()` and gains `launcher`; the autostart file is `cursora.desktop`; `--ask` is ignored while paused. |
| `src/useSettings.ts`, `src/Settings.tsx`, `src/Privacy.tsx` | `useAppInfo()`, shared by General, About and Privacy (one `AppInfo` type instead of two). General on Wayland: the shortcut steps instead of the hotkey fields. About on Linux: the tray note. Advanced hides the Ctrl+C-unsafe list on Linux, where it changes nothing. |
| `src/Chat.tsx`, `src/conversation.ts` | The `unreadable` notice. |
| `src/styles.css` | Issue 20: the rows whose text AT-SPI can't see. |
| `src-tauri/tauri.conf.json` | The .deb recommends `wl-clipboard`. |
| `README.md` | Linux: the tray on GNOME, the Wayland shortcut, wl-clipboard. |
| `.github/workflows/ci.yml`, `scripts/wsl-test.sh` | Linux tests run under `xvfb-run`, so the X11 code is tested against a real X server. |

## Data

### `app_info`
```jsonc
{
  "session": "x11",                 // "windows" | "x11" | "wayland" | "unknown" (LNX-1.1); About shows "Linux (X11)"
  "identifiesApps": true,           // windows and x11 (PRI-2.5)
  "launcher": "/usr/bin/cursora"    // new: the AppImage file when run from one; also what autostart starts
}
```

### Capture
- `Notice::Unreadable`: the bubble says "No selection could be read in this session." and opens as a general chat (LNX-3.4).
- `Method::Direct` (was `Uia`): read without keys, by UI Automation on Windows and PRIMARY or `wl-paste` on Linux. It's only logged.

No new settings.

## Key mechanics

### Session (LNX-1.1)
`detect(xdg_session_type, wayland_display, display)`:
1. `XDG_SESSION_TYPE` is `x11` or `wayland`: that.
2. Otherwise (`tty`, unset, …): `WAYLAND_DISPLAY` set means wayland; `DISPLAY` set means x11.
3. Otherwise unknown. Nothing can be read then (`Notice::Unreadable`), and registering the hotkey fails with its usual message.

**XWayland.** On native Wayland a GTK app can't place its windows, keep them on top or keep them off the taskbar, and GNOME won't focus a window that appears without an activation token. So in a Wayland session with XWayland (`DISPLAY` set), `run()` sets `GDK_BACKEND=x11` before GTK starts, unless the user has set `GDK_BACKEND`. Cursora's own windows are then X11 windows, which the compositor manages like any other. Capture still treats the session as Wayland, because through XWayland Cursora only sees X11 apps.

### The X11 client (`x11.rs`)
- `x11rb`'s connection is pure Rust, so there is no libxcb or libXtst to install. Each user opens its own short connection: a capture, or a focus call. Only `serve()` keeps one, on its own thread.
- `convert(selection, target, deadline)` asks the selection's owner to put `target` in a property of our own hidden window, waits for SelectionNotify, and reads the property. A large value arrives in INCR chunks, which it collects. It gives up at the deadline. Returns the type, format and bytes.
- `serve(selection, items)` starts a thread with its own connection and window. The thread takes the selection and answers each SelectionRequest, for TARGETS and every saved target, with exactly the saved type, format and bytes. It ends on SelectionClear, that is when anything else is copied.
- `activate(window)` sends `_NET_ACTIVE_WINDOW` to the root with source 2 (pager). Window managers honor a pager's request without the user-timestamp check that blocks ordinary focus requests (issue 18).

### Capture on X11
- **Foreground (LNX-4.3).** `_NET_ACTIVE_WINDOW` on the root, then its `_NET_WM_PID`, then the file name of `/proc/<pid>/exe`, or `/proc/<pid>/comm` when that can't be read. That is `exe` and `name`. A window without `_NET_WM_PID` uses the first part of `WM_CLASS`. The title is `_NET_WM_NAME`, else `WM_NAME`. `window` is the X window id, for focus on close (BUB-2.3).
- **Text (LNX-3.1).** `accessible_selection` is `convert(PRIMARY, UTF8_STRING)`, within 100 ms. No keys.
- **A blocked app's selection (LNX-3.5).** `Platform::selection_owner()` (default none) is, on X11, the app behind PRIMARY's owner window: its pid as for the foreground, then the process name. `capture()` skips PRIMARY when that app is blocked, as it does in file managers. The owner window is a hidden one with no title, so only process rules can match it.
- **Files (LNX-3.2).** `capture()` skips PRIMARY for `FILE_MANAGERS` and runs the spec 02 copy chain:
  1. Wait until every key except Ctrl is up (below).
  2. Snapshot: TARGETS, then every target except TARGETS, MULTIPLE, TIMESTAMP, SAVE_TARGETS and DELETE, within 300 ms in all. If any part fails, there is no snapshot, and the clipboard is not touched (SEL-3.1).
  3. The clipboard sequence number counts XFixes owner-change events for CLIPBOARD.
  4. Ctrl+C through XTest.
  5. Read `text/uri-list`, else `x-special/gnome-copied-files`, else `UTF8_STRING` text. `uri_paths()` turns `file://` URIs into paths: percent-decoded, with comments, the copy/cut line and other schemes skipped.
  6. Restore: `serve(CLIPBOARD, snapshot)`. If the clipboard was empty before, it gets no owner again.
- **Every key except Ctrl up (SEL-3.2 on X11).** While the user holds a grabbed key, X sends every key event to the app that grabbed it. Ctrl+C sent while the G of Ctrl+G is still down would go to Cursora's own hotkey grab instead of the file manager. So on X11 `extra_modifiers_released` waits, up to the same 300 ms, until QueryKeymap shows nothing but Ctrl down. Ctrl is part of every copy key anyway.
- `FILE_MANAGERS`: `nautilus`, `dolphin`, `thunar`, `nemo`, `caja`, `pcmanfm`, `pcmanfm-qt` (process names).
- `copy_key(exe, cfg, primary)`: the user's per-app shortcut first (SEL-4.4). Then, where the platform reads PRIMARY, file managers get Ctrl+C and every other app `never`. Otherwise it's unchanged.

### Capture on Wayland (LNX-3.4)
- `foreground` is unknown. `identifies_apps()` is false, so the Privacy tab warns (PRI-2.5).
- `accessible_selection` runs `wl-paste --primary --no-newline --type text`. It is killed after 300 ms, and its output is capped at `maxChars × 4` bytes.
- `selection_readable` says whether `wl-paste` is on the PATH, checked when the platform is created. If it isn't, the notice is `Unreadable`.
- No keys and no clipboard: XTest would only reach X11 apps. File managers therefore give nothing.

### Hotkey and `cursora --ask` (LNX-2)
- **X11:** unchanged. `tauri-plugin-global-shortcut` grabs the keys with XGrabKey (LNX-2.1).
- **Wayland:** no hotkey is registered. Through XWayland it would fire only while an X11 app has focus. Settings → General shows this instead:
  > Wayland doesn't let apps listen for hotkeys. Add a shortcut in your desktop's settings that runs:
  > `/usr/bin/cursora --ask` **Copy**
  > - GNOME: Settings → Keyboard → View and Customize Shortcuts → Custom Shortcuts → +
  > - KDE: System Settings → Keyboard → Shortcuts → Add New → Command or Script
  >
  > Selections are read with `wl-paste`, from the wl-clipboard package.

  The command is `launcher`, in single quotes (a `'` written as `'\''`) if it holds anything but letters, digits and `/._+-` (issue 31).
- `--ask` reaches the running instance through single-instance, which uses D-Bus on Linux (LNX-2.3). While paused it is ignored (tweak 2).

### Bubble (LNX-4)
- **X11:** the cursor position comes from Tauri (XQueryPointer under GDK), and the monitors and their work areas from GDK (XRandR and `_NET_WORKAREA`). Both have been used since spec 03, so LNX-4.1 needs no new code, only its check.
- **Wayland:** `on_ask` passes no cursor, because XWayland's pointer position is stale while a Wayland app has focus. The bubble is then centered on the primary monitor (BUB-1.4, LNX-4.2).
- **Focus (issue 18):** `focus()` is `set_focus()` plus `activate(bubble)`. It runs again after the existing 150 ms re-place, since a window manager ignores activating a window it hasn't mapped yet.
- **Close (issue 4):** `activate(previous)`.
- `handle()` is the bubble's X window id, from raw-window-handle (0 on native Wayland).

### Desktop integration (LNX-5)
- **Autostart (LNX-5.2):** `auto-launch` names the file after the app name, which is "Cursora". On Linux it gets the crate name, so the file is `~/.config/autostart/cursora.desktop`. Windows keeps "Cursora", so existing Run entries are still found.
- **Tray (LNX-5.3):** it already goes through libayatana-appindicator. About on Linux adds:
  > No Cursora icon in the top bar? GNOME needs the AppIndicator extension (Ubuntu includes it). Everything works without it: the hotkey asks, and `cursora --settings` opens this window.
- **Packaging (LNX-5.1):** CI already builds the .deb and the .AppImage on ubuntu-22.04 (glibc 2.35), so Ubuntu 24.04 and Fedora can run them. The .deb recommends `wl-clipboard`.

### Screen readers (issue 20)
WebKitGTK leaves the text of `display: flex` items out of AT-SPI. Affected: the bubble header, the badges in Settings and the context row. The task finds the smallest layout that exposes the text (a grid, or inline-block children), checks it with the AT-SPI script (and Orca if it can run), and confirms that Windows is unchanged.

## Commands and events
- `app_info` gains `launcher`; its `session` comes from `session::current()`. Nothing else changes.

## Changes to earlier specs' code
- **Spec 01:** `app_info`; the autostart file name on Linux; `--ask` while paused.
- **Spec 02:** `primary()`, `selection_readable()`, `FILE_MANAGERS` and `copy_key` in `capture()`; `Method::Direct`; `Notice::Unreadable`; `identifies_apps()` per session.
- **Spec 03:** `bubble/linux.rs`; no cursor on Wayland; focus again after the re-place.
- **Spec 07:** PRI-2.5 now holds on X11, and the tray's "Block Cursora in <app>" works there without changes. The blocklist also covers PRIMARY's owner (LNX-3.5).

## Dependencies
- **`x11rb` 0.13**, Linux only, with the `xfixes`, `xtest` and `res` features. It is already compiled on Linux for `global-hotkey`; the features only add generated protocol code.
- **`raw-window-handle` 0.6**, Linux only. Already compiled for Tauri.
- **Tests:** `xvfb` in CI and in WSL.
- **At run time on Wayland:** `wl-clipboard`. The .deb recommends it, and Settings says so.

## Decisions
1. **XWayland in Wayland sessions** (see Session). *Rejected:* native Wayland, where the compositor places the bubble, it isn't kept on top, it may not get focus, and it shows in the taskbar.
2. **PRIMARY only; keys only for file managers** (see Overview).
3. **Our own small X11 client on `x11rb`.** *Rejected:* GTK's clipboard. It runs on the main thread, and every wait on the main thread has been a freeze risk (the tray and the settings lock in spec 07). `x11rb` is already compiled.
4. **Restore by serving the saved copy.** On X11 the clipboard isn't stored anywhere: the app that copied hands it out on request. To put the user's clipboard back, Cursora has to hold it and answer for it until something else is copied.
5. **A real X server in the tests.** The selection protocol is the riskiest code here. The tests own selections with `serve()` and read them back through the platform, under `xvfb-run` in CI and in WSL.

## Requirement tweaks (approved, applied)
1. **LNX-2.2: the GlobalShortcuts portal moves to the backlog.** Settings shows the binding steps on every Wayland desktop.
   - GNOME added the portal's global shortcuts only in GNOME 48. Ubuntu 22.04 and 24.04 LTS don't have it, so the steps are needed anyway.
   - It's about 100 lines of D-Bus code (a session, the desktop's bind dialog, the Activated signal, registering the app id) that no machine here can run, so it would ship untested.
   - The steps work on every GNOME and KDE version.

   The `cursora --snip` line is added by spec 10, once `--snip` does something.
2. **APP-1.3: while paused, `cursora --ask` does nothing either.** On Wayland the desktop shortcut is the hotkey, and Cursora can't unregister it.
3. **SEL-3.2 on X11:** wait until every key except Ctrl is up (see Capture on X11).

## Known limits
- `// shortcut:` the restore serves without INCR. If one clipboard item is larger than the X server's request size (16 MB with BIG-REQUESTS), the snapshot refuses and no file copy happens. Upgrade: if users copy huge images and then ask about files.
- The restored clipboard lives in Cursora. Quitting before anything else is copied empties it, unless a clipboard manager kept it.
- Apps that don't set PRIMARY (some Wine and Java apps) give nothing, and the bubble opens as a general chat. A per-app copy shortcut in Settings → Advanced still turns the copy on for them.
- PRIMARY can be stale: text selected earlier in another app. The requirements accept this (removable context, BUB-3.2).
- `// shortcut:` a blocked selection (LNX-3.5) is recognized by process rules only: a title rule can't match the owner's untitled window, and a clipboard manager that takes PRIMARY over hides the app it came from. Upgrade: remember the window in front when PRIMARY changes owner, if users block web pages by title.
- **Wayland:**
  - No files.
  - No app identification, so no blocklist.
  - The bubble opens on the primary monitor.
  - Text can be blurry under fractional scaling (XWayland).
  - On GNOME, which lacks the data-control protocol, `wl-paste` may flash a tiny window to read the selection.
  - A compositor without primary-selection support shows "Nothing was selected" instead of the unreadable hint.
- The display name is the process name (`gnome-terminal-server`), as Windows uses the exe name.

## Test strategy

| Criterion | Test |
|---|---|
| LNX-1.1 | `detect()` table: each `XDG_SESSION_TYPE`, the `WAYLAND_DISPLAY` and `DISPLAY` fallbacks, and nothing set gives unknown. |
| LNX-3.1 | Fake platform with PRIMARY: the text comes back with no snapshot and no key. Real X server: a selection owned with `serve(PRIMARY)` is read by `LinuxPlatform`. |
| LNX-3.2 | Fake: `nautilus` skips PRIMARY, sends Ctrl+C, returns the files and restores the clipboard. `uri_paths()`: uri-list and gnome-copied-files, percent-encoding, comments, CRLF, non-file URIs. Real X server: a uri-list on CLIPBOARD is read as files; a snapshot, then another owner, then the restore gives back every target byte for byte; the sequence number moves when the owner changes. |
| LNX-3.3 | Fake: every terminal in the list, with and without PRIMARY text: no key is sent, and `copy_key` is `never`. |
| LNX-3.5 | Fake: PRIMARY owned by `bitwarden` with an editor in front: not read, no key, "nothing selected"; owned by an unblocked app: read. Real X server: `selection_owner()` names the test binary while it serves PRIMARY. |
| LNX-4.3 | Real X server: a window whose `_NET_WM_PID` is the test's own pid and whose `_NET_WM_NAME` is set, made the root's `_NET_ACTIVE_WINDOW`: `exe` is the test binary's name and the title is the one set. Without the pid, the `WM_CLASS` name. Wayland: without X11 the app is unknown, and `identifies_apps` is false for wayland and unknown. |

The real-X-server tests are Linux-only and fail with "run under xvfb-run" when `DISPLAY` is missing, instead of being skipped silently.

Manual: LNX-2.1, 2.2, 2.3, 3.4, 4.1, 4.2, 5.1–5.3, 6.1 and 6.2.
- **In WSL** (Xvfb + Openbox, as before): 2.1, 2.3, 3.1–3.3 with real apps (a GTK editor, xterm, a file manager), 4.1 on one monitor, 4.3, the bubble's focus and the focus back, and the Linux checks from specs 01 and 02 still open in issue 11.
- **Wayland:** in WSL only if a headless compositor (weston) runs there without showing windows on the Windows desktop.
- **Real Ubuntu** (22.04 and 24.04, GNOME on X11 and on Wayland), KDE and Fedora are not available here, so they stay open (issue 2).

## Changes during implementation
- **2026-10-13, build:**
  - **The process comes from the X server first, then `_NET_WM_PID`.** The X server knows the pid of every local client (the X-Resource extension). `_NET_WM_PID` is only the app's own claim, and inside a Flatpak sandbox it counts in the sandbox's own numbering, so a password manager installed as a Flatpak could have slipped past the blocklist. `x11rb` gains the `res` feature.
  - **Text is read as `UTF8_STRING` only**, with no `STRING` fallback: every app in the matrix offers it, xterm included.
  - **The clipboard snapshot leaves out a target that the owner offers but won't give**, instead of failing, since no app could paste it either. It still fails on a timeout or an item too big to hand back.
  - **The X11 tests run only with `CURSORA_XVFB=1`**, which `scripts/wsl-test.sh` and CI set when they give the tests their own X server: the tests take over the clipboard and press keys. Without it they fail with that instruction instead of being skipped.
- **2026-10-13, issue 20:** 30 layouts were tried in a WebKitGTK page. A `<span>` that is a flex or grid item is dropped from AT-SPI; a span that is itself a flex container, bare text, `<strong>`, `<label>` and `<p>` are kept. The fix is CSS only: text-only spans in `.row`, the chat header's spans and `.badge` are `inline-flex`. Spans that mix text and elements are left alone, because inside a flex container the space between the text and the element disappears (seen on the Personality switch).
- **2026-10-14, review:** a blocked app's PRIMARY could be read from another app (issue 30), so LNX-3.5 was added and `capture()` asks for the selection's owner. The Wayland shortcut command quotes any path with characters a shell treats specially, an apostrophe included (issue 31).
