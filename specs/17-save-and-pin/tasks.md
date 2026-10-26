# 17: Save and pin answers (tasks)

- [x] 1. `chat::kept` (a finished turn by `asked`) and `keep.rs`: `file_name`, `markdown`, `write`, the default folder, with tests. Covers SAV-1.3, 2.2, 2.3, 2.4, 2.6.
- [x] 2. `skills::today()` becomes `local_time()`; `model_folder` becomes the shared `pick_folder`; `open_folder("saved")`; the setting `saved_answers_folder`.
- [x] 3. The commands `pin_answer`, `pinned`, `save_answer`, `show_saved`; the `Pins` state; the note window beside the calling window; `pin-*` in the bubble capability.
- [x] 4. `Pin.tsx`; Pin and Save in `Chat.tsx`; the Saved answers row in Settings → General; the manual and README. Covers SAV-1.1, 1.2, 1.4, 2.1, 2.5.
- [ ] 5. Check on Windows (test build); then `/ponytail-review`. Windows done 2026-10-26 (below). The review found two, both fixed and checked: Pin and Save could fail on a fresh or regenerated answer (R72), and Show in folder refused a file after the folder setting changed.

## Manual checklist
On X11, [x11-check.sh](x11-check.sh) runs the checklist in WSL on the stronger PC and prints PASS or FAIL for each item.

- [ ] SAV-1.1 Pin on a finished answer, in the bubble and the expanded window; the note beside the bubble, on top, with its question as the title, Copy and Close. Windows ✓ (2026-10-26, test build: from the bubble and from the expanded window, also after Regenerate; the note opened just right of the bubble, titled with the question) X11 ☐
- [ ] SAV-1.2 The note stays when the bubble closes and a new conversation starts; two notes at once; gone after Quit. Windows ✓ (2026-10-26: both, the second 24 px lower; none after Quit) X11 ☐
- [ ] SAV-1.4 The note in light and dark; its answer and buttons read by a screen reader. Windows ✓ light and dark, title bar too; the buttons have names in UI Automation ☐ a real screen reader X11 ☐
- [ ] SAV-2.1 Save, then Saved and Show in folder, which selects the file. Windows ✓ (2026-10-26: `2026-10-26 When is the report due.md` with the time, app, model, quoted question and answer, not the selection; a second save made `… (2).md`; Explorer opened with the file selected, also after the folder setting changed) X11 ☐
- [ ] SAV-2.5 Settings → General: the folder, Change… and Open. Windows ✓ (2026-10-26: `C:\Users\<user>\Documents\Cursora` shown by default; Change… set another; Open showed it) X11 ☐
