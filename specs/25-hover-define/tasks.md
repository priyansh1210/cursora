# 25: Hover to define (tasks)

Status: **In progress**.

- [x] 1. `Hotkeys::define`: validated, registered, sent to `snip::define`; the Define field in Settings and setup. Covers HOV-1.1.
- [x] 2. `ocr::words` on Windows and Linux, `tsv()`, `crop()`, `word_at()`, with tests. Covers HOV-1.2.
- [x] 3. `snip::define`: the blocklist, the grab, the word, the bubble and `/define`; `Method::Screen` without Replace or Insert; the notices. Covers HOV-1.3, HOV-1.4, HOV-1.5, HOV-1.6.
- [x] 4. The manual; all checks; by hand on Windows; then `/ponytail-review`. 2026-10-29: the review found that a changed Define key (and Snip, since spec 10) wasn't registered until a restart, since saving re-registered only when Ask or Pause changed; and that a Define key held by another app made every later change of Ask or Snip fail with an error about Define. Now any key change re-registers, and a clash on a key the change didn't touch is only logged. Checked in the test copy with the Define key held by another process: the start said so, Ask was changed and saved, and a new Define key worked at once.

## Manual checklist
- [ ] HOV-1.1 The Define field in Settings → General and in setup, Ctrl+Alt+G by default. Windows ✓ Settings (2026-10-29, test copy: Define under Ask and Snip with its line) ✓ setup, by the user (2026-10-29: Re-run setup, step 3 shows Ask, Snip and Define with the line) Linux X11 ☐
- [ ] HOV-1.3 Point at a word in Notepad and press: the bubble at the pointer defines it; then another word. Windows ✓ by the user in a web page ("mountain") and a PDF ("course"), 2026-10-29 ✓ (2026-10-29, test copy with the key set to Ctrl+Alt+Shift+F11: the bubble 239 ms after the key, "From Notepad · 9 characters: ephemeral", `/define` asked by itself and answered, no Replace; pressed again over "blossoms" with the bubble open, that word was defined) Linux X11 ✓ (2026-10-29, the stronger PC's WSL, `scripts/x11-issues-check.sh` `after`: "ephemeral" in an xterm, found by Tesseract on the screen, defined with the word as the context and /define asked; then "blossoms")
- [ ] HOV-1.4 Press over empty space: the bubble says no word was found. Windows ✓ (2026-10-29: the notice, with "Ask anything" ready) Linux X11 ✓ (2026-10-29, the stronger PC's WSL, `scripts/x11-issues-check.sh` `before`, without tesseract-ocr: "Install tesseract-ocr so Cursora can read the word under the pointer.")
