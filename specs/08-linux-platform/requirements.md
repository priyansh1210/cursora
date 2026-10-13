# 08: Linux platform (LNX)

## Scope
- **In:**
  - Linux implementations of the hotkey, selection capture, cursor position, active-app detection, tray and autostart
  - Packaging
  - **X11 is fully supported. Wayland is best-effort**, and every fallback is documented here.
- **Out:** distribution-specific packages other than `.deb` and `.AppImage`.

## Requirements

### LNX-1 Session detection
- LNX-1.1 (auto) THE SYSTEM SHALL detect the session type (X11 or Wayland) from `XDG_SESSION_TYPE`, `WAYLAND_DISPLAY` and `DISPLAY`, and choose implementations to match. Settings → About SHALL show the detected type.

### LNX-2 Hotkey
- LNX-2.1 (manual) On X11 THE global hotkeys SHALL work as they do on Windows (SEL-1).
- LNX-2.2 (manual) On Wayland THE Settings window SHALL show copy-paste-ready steps to bind `cursora --ask` as a GNOME or KDE custom shortcut. (Amended 2026-10-13: the XDG Desktop Portal *GlobalShortcuts* interface moved to the backlog; the `cursora --snip` line arrives with spec 10.)
- LNX-2.3 (manual) WHEN `cursora --ask` runs while Cursora is running THE capture flow SHALL start within 150 ms (APP-2.2).

### LNX-3 Selection capture
- LNX-3.1 (auto) On X11 THE first text strategy SHALL be reading the **PRIMARY** selection, which needs no simulated keys.
- LNX-3.2 (auto) WHEN the foreground app is a known file manager (Nautilus, Dolphin, Thunar, Nemo, Caja, PCManFM) THE SYSTEM SHALL use the files strategy:
  1. simulate Ctrl+C via XTest
  2. read `text/uri-list` or `x-special/gnome-copied-files`
  3. restore the clipboard (SEL-3)
- LNX-3.3 (auto) THE SYSTEM SHALL never simulate Ctrl+C in terminal apps (gnome-terminal, konsole, xterm, alacritty, kitty, wezterm, tilix, terminator, xfce4-terminal, ptyxis). PRIMARY covers terminals.
- LNX-3.4 (manual) On Wayland THE SYSTEM SHALL read PRIMARY via `wl-paste --primary` when it is available and supported. Otherwise the bubble SHALL open in general-chat mode with the hint "No selection could be read in this session."
- LNX-3.5 (auto) On X11 WHEN the PRIMARY selection is owned by an app that matches a blocklist rule THE SYSTEM SHALL NOT read it, whichever app is in front; the capture goes on as if nothing were selected (PRI-2.1). (Added 2026-10-14, issue 30: X11 has one PRIMARY for every app, so text selected in a password manager could be read from another app.)

### LNX-4 Cursor position and active app
- LNX-4.1 (manual) On X11 THE bubble SHALL open at the cursor (XQueryPointer) and be correct on multi-monitor setups (XRandR work areas).
- LNX-4.2 (manual) On Wayland THE bubble SHALL open centered on the active monitor (BUB-1.4).
- LNX-4.3 (auto) On X11 THE active app SHALL be resolved from `_NET_ACTIVE_WINDOW` → `_NET_WM_PID` → process name, plus `WM_CLASS` and `_NET_WM_NAME` for the title. On Wayland, where this is generally unavailable, the active app SHALL be reported as unknown (PRI-2.5).

### LNX-5 Packaging and desktop integration
- LNX-5.1 (manual) CI SHALL produce x86_64 `.deb` and `.AppImage` builds that install and run on:
  - Ubuntu 22.04 and 24.04
  - Fedora (via AppImage)
- LNX-5.2 (manual) Autostart SHALL use `~/.config/autostart/cursora.desktop` (APP-3.1).
- LNX-5.3 (manual) THE tray SHALL use AppIndicator. On GNOME, where there is no tray by default, Settings and the README SHALL explain the AppIndicator extension. Cursora SHALL stay fully usable through the hotkey and `cursora --settings`.

### LNX-6 Compatibility matrix
- LNX-6.1 (manual) On X11, text capture SHALL work in:
  - Firefox, Chromium
  - GNOME Text Editor, LibreOffice Writer, VS Code
  - GNOME Terminal and Konsole, while a process is running, which must not be interrupted

  File capture SHALL work in Nautilus (files and folders).
- LNX-6.2 (manual) On Wayland (GNOME and KDE), the documented fallbacks SHALL behave as specified: shortcut binding, `wl-paste`, centered bubble, and the blocklist warning.

## Open questions
- _(none.)_

## Resolved
- Possibly stale PRIMARY text is shown as removable context (BUB-3.2), which is enough for v1 (confirmed).
