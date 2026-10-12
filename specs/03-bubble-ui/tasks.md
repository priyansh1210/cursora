# 03: Bubble UI (tasks)

Status: **In progress**.

- [x] 1. Bubble window: created hidden at startup, frameless, rounded, on top, no taskbar/Alt+Tab, ignores close requests, CSP, `bubble` capability. Our own `activate` (no Alt tap). Checks: hidden-bubble RAM (try WebView2 low-memory mode), the `monitor_from_point` coordinate space, `navigator.clipboard`. Covers BUB-2.1, 2.2.
- [x] 2. `place()` (tests first), then `open`/`close`/`bubble_resize`, and the hotkey toggle with focus going back to the app. Covers BUB-1.1–1.4, 2.3, SEL-1.4.
- [x] 3. `chat.rs` + `prompt.rs`: types, `Model` seam, `Echo` (debug), `assemble()` (tests first), the turn worker (tests first), and the commands. Covers BUB-2.6, 4.1, 5.1, 5.2.
- [x] 4. `history.rs` + the `history` settings (tests first). Covers BUB-9.1, 9.3, 9.4.
- [x] 5. Frontend helpers with vitest: `markdown.ts`, `conversation.ts` (tests first). Covers BUB-4.1, 4.2, 4.4, 7.1.
- [x] 6. `Chat.tsx`: header, context, turns, input, actions, errors, no-model view, Expand window, `open_settings(tab)`. Covers BUB-2.4, 2.5, 3.1–3.3, 4.3, 4.5, 6.1, 8.1, 8.2.
- [x] 7. History tab. Covers BUB-9.2.
- [x] 8. End-to-end run on Windows and `/ponytail-review`. The review found 6 issues and the run found 2, all fixed (see design.md "Changes during implementation").

## Manual checklist
- [ ] BUB-1.5 Opens on the cursor's monitor, fully visible near every edge and corner. Windows: one monitor ✓ (auto-verified: bottom-right corner flips, stays inside the work area) ☐ two monitors X11 ☐
- [ ] BUB-2.1 Frameless, rounded (Windows 11), on top, not in the taskbar or Alt+Tab, pre-created. Windows ✓ (auto-verified: tool window + topmost styles, screenshots) X11 ☐
- [ ] BUB-2.2 The input has focus when the bubble opens. Windows ✓ (auto-verified: typed question landed in the input) X11 ☐
- [ ] BUB-2.3 Esc and clicking elsewhere keep it open; the hotkey closes it and focus returns to the app. Windows ✓ Esc, hotkey, focus (auto-verified) ☐ click elsewhere X11 ☐
- [ ] BUB-2.4 Drag by the header. Windows ☐ X11 ☐
- [ ] BUB-2.5 380 px wide, grows to 520 px then scrolls; Expand opens a resizable window. Windows ✓ grows, Expand (auto-verified) ☐ scrolls past 520 px X11 ☐
- [ ] BUB-3.1 Text preview with toggle and count, file chips, source app. Windows ✓ text (screenshots) ☐ file chips X11 ☐
- [ ] BUB-3.2 × removes the context before the first question. Windows ☐
- [ ] BUB-3.3 Truncation and left-out notices. Windows ☐
- [ ] BUB-4.3 Stop keeps the partial answer. Windows ☐
- [ ] BUB-4.5 "Thinking" until the first token. Windows ☐
- [ ] BUB-5.3 A new capture starts a new conversation; the old one is in History. Windows ☐
- [ ] BUB-6.1 Copy (Shift = plain text) and Regenerate. Windows ✓ Copy as Markdown (auto-verified) ☐ Shift+Copy ☐ Regenerate
- [ ] BUB-8.1 Everything reachable by keyboard, buttons labeled. Windows ☐ X11 ☐
- [ ] BUB-8.2 Follows the theme. Windows ☐ X11 ☐
- [ ] BUB-9.2 History tab: list, search, open, delete, clear. Windows ✓ list (screenshot) ☐ search, open, delete, clear
- Spec 02 items that needed the bubble: SEL-1.4 ✓ and SEL-2.5 ✓ (auto-verified: 108 ms on the copy path); still open: SEL-4.5, 5.1 (Desktop, folders), 6.1, 8.1.
