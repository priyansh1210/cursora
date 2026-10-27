# 20: Snip with any model (tasks)

Status: **In progress**.

- [x] 1. `snip/ocr.rs`: `available`, `read` (Windows and Linux), `tidy`, with tests. Covers OCR-1.2, OCR-1.5.
- [x] 2. The snip flow: `begin`, `Pending.read_text`, `prepare`, `finish`; `Snip.text`; the notices; `/snip` and "Snip an area instead". Covers OCR-1.1, OCR-1.4, OCR-1.6.
- [x] 3. The prompt block (OCR-1.2); the bubble's "Read as text" (OCR-1.3); spec 10 amended; the manual.
- [ ] 4. All checks; by hand on Windows; then `/ponytail-review`. 2026-10-28: trace, JS (20), build, cargo test (186, Windows, with real recognition of a fixture), test:linux (193, WSL, with Tesseract 5.3.4 reading the same fixture) pass. The real-app check found that a model that can't see was still sent the image, which llama-server refuses; fixed in `build()` with a test.

## Manual checklist
- [ ] OCR-1.3 A snip of text with the built-in model: "Read as text" with the count and Show more, and an answer about it. Windows ✓ (2026-10-28, test copy, Qwen 1.5B: a TypeScript error snipped in Notepad showed "Read as text · 51 characters" and Show more; the answer, 17 s later, explained it from the text. Windows read Notepad's slashed zero as Ø) Linux X11 ☐ (Tesseract runs in WSL; the app's snip overlay there is for the stronger PC)
- [ ] OCR-1.6 Linux without Tesseract: the notice says to install it; with it, OCR-1.3 works. Linux X11 ☐ (issue 65)
