# 27: Fill forms (tasks)

Status: **In progress**.

- [x] 1. `fill/mod.rs` with its tests; `my_details` in the settings; `/fill` as a system command. Covers FIL-1.2, FIL-2.2, FIL-2.3, FIL-2.6, FIL-3.2, FIL-3.4.
- [x] 2. `fill/windows.rs`: reading the boxes, focus, the frame, typing. Covers FIL-2.2, FIL-3.1, FIL-3.3.
- [x] 3. `/fill` in `ask` and "What will be sent"; the plan on the turn; the `fill` command. Covers FIL-2.3, FIL-2.4, FIL-2.5, FIL-3.1.
- [x] 4. My details in Settings → Personality; Fill this form and the plan in the bubble. Covers FIL-1.1, FIL-2.1, FIL-2.5.
- [x] 5. All checks; by hand on Windows; then `/ponytail-review` (2026-10-30: its findings fixed, R89 to R92).

## Manual checklist
- [ ] FIL-1.1 My details in Personality: the 17 names, editing, removing, adding, the note, saved. Windows: shown ✓ (2026-10-30, the test copy: the 17 rows under About you, the note, Remove on each row, Add a detail; the saved details filled the forms below); editing, removing and adding by hand ☐ Linux X11 ☐
- [ ] FIL-2.1 Fill this form in the bubble with nothing selected in a form; not with a selection. Windows ✓ (2026-10-30: offered in Edge, Chrome and a dialog box with nothing selected; not offered with text selected in Notepad, where `/fill` says to click into the form without selecting) Linux X11: n/a until Linux is built (`/fill` says so) ☐
- [ ] FIL-2.5 The plan: rows with values and tick boxes, left boxes, already filled; with a model that fails. Windows: rows, tick boxes and left boxes ✓ (2026-10-30); a failing model is unit-tested only ☐ (issue 74)
- [ ] FIL-3.1 Fill types each ticked value letter by letter in a green frame: a test form in Edge and Chrome, a Microsoft Form, a Google Form, a dialog box. Windows: Edge, Chrome and a dialog box ✓ (2026-10-30, the page saw one input event per letter; hidden boxes left alone); a Microsoft Form and a Google Form ☐ (issue 74)
- [x] FIL-3.3 Esc stops it; clicking another window stops it. Windows ✓ (2026-10-30, Edge: Esc mid-typing stopped it and the page never saw the Esc; a click on the taskbar stopped it after the mobile number)
