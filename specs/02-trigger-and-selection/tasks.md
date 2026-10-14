# 02: Trigger and selection capture (tasks)

Status: **In progress**. Code complete; the manual checklist is open (most items need spec 03's bubble).

- [x] 1. Settings `capture` section with defaults. Covers SEL-4.3 (default list), feeds SEL-2.4, 3.3, 4.4, 5.3, 7.2.
- [x] 2. `capture/mod.rs`: types, `Platform` trait, `FakePlatform`, `capture()`, `copy_key()`, `truncate()`, `for_prompt()`. Tests first. Covers SEL-2.1–2.4, 3.1–3.4, 4.1–4.4, 6.2, 7.1, 7.2.
- [x] 3. `capture/files.rs`: `file_context()`. Tests first. Covers SEL-5.2, 5.3, 5.5, 5.6.
- [x] 4. `hotkey.rs`: `validate()` (tests first), `apply()`, wired into startup and `update_settings`. Covers SEL-1.1, 1.2, 1.3, APP-1.3.
- [x] 5. `capture/windows.rs` (`WinPlatform`) + `capture/other.rs` stub; hotkey → capture thread → `on_capture` logs metadata. Covers SEL-2.5, 3.5, 4.5, 5.1, 6.1, 8.1.
- [x] 6. UI: Ctrl+G note, plus General → Advanced (unsafe apps, per-app copy shortcuts, share window title). Covers SEL-1.5, SEL-4.3 (editable), SEL-4.4, SEL-7.2.
- [x] 7. `/ponytail-review` of the spec 02 code. 3 findings, all fixed (see design.md "Changes during implementation").

## Manual checklist
Most of these need spec 03's bubble to *see* the captured text. Until then, the log shows method, length and app.
- [x] SEL-1.1 The ask hotkey works in any app. Windows ✓ (auto-verified: injected Ctrl+G fired in a test window and in Explorer) ✓ real apps: Notepad, Chrome, Edge, Word, Excel, Windows Terminal, Explorer, Desktop (auto-verified 2026-10-13) · X11 ✓ mousepad, xterm, pcmanfm (auto-verified 2026-10-14 in WSL, spec 08; real Ubuntu: issue 2)
- [x] SEL-1.3 A hotkey owned by another app is refused, and the old one is kept. Windows ✓ (auto-verified 2026-10-13): Ctrl+Alt+K held by another program, message shown, Ctrl+G kept and still works
- [x] SEL-1.4 The hotkey closes an open bubble. Windows ✓ (auto-verified with spec 03's bubble)
- [x] SEL-1.5 The Ctrl+G note shows in Setup and in Settings. Windows ✓ (auto-verified 2026-10-13)
- [x] SEL-2.5 The bubble is visible within 200 ms on the copy path. Windows ✓ (auto-verified: 108 ms, capture 83 ms)
- [x] SEL-3.5 Copy X, select Y, hotkey, paste → X. Windows ✓ (auto-verified: clipboard "ORIGINAL" survived two copy-path captures) · X11 ✓ the copy path in pcmanfm gave the clipboard back, every target (auto-verified in WSL, spec 08)
- [x] SEL-4.5 Windows Terminal with a running process: not interrupted, text captured. Windows ✓ (auto-verified 2026-10-13): read through UI Automation, no keys sent, the process kept writing
- [x] SEL-5.1 Files and folders from Explorer and the Desktop. Windows: Explorer ✓ (auto-verified: 3 files) ✓ Desktop ✓ folders (auto-verified 2026-10-13)
- [x] SEL-6.1 Admin app → clear message, no keys sent. Windows ✓ (2026-10-14, with UAC on: an elevated Notepad, the user pressed Ctrl+G; the bubble showed the message, and the log showed method None, notice Elevated)
- [ ] SEL-8.1 The app compatibility matrix. Windows ✓ Notepad, Chrome, Edge, Edge PDF, Word, Excel, Windows Terminal, Explorer, Desktop (auto-verified 2026-10-13) ☐ Firefox, VS Code, an Electron app, IntelliJ (not installed on the test PC)
