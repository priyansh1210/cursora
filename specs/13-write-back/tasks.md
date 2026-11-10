# 13: Write back into apps (tasks)

- [x] 1. `capture::write_back` (the rule) and `capture::paste` with the Fake platform's tests; `text_snapshot` and `send_paste` on Windows and X11. Covers WRT-3.1, WRT-4.1.
- [x] 2. `plain_text` and its test; `Conversation.write_back`; the `write_back` command with the foreground check (`bubble::bring_back`). Covers WRT-1.1 to 1.3, 2.1, 4.2.
- [x] 3. `output: replace` in skills (file, form and the skill form's checkbox), the replacement rule, the four built-ins. Covers WRT-5.1, WRT-5.2.
- [x] 4. Replace and Insert in the bubble's actions; why it didn't happen shows under the answer, scrolled into view. Covers WRT-1.1, WRT-2.1.
- [ ] 5. Check on Windows (done with a test window, below) and on X11 in WSL on the stronger PC (done 2026-10-26, below; it found R66 and R67, both fixed), then `/ponytail-review`. `/ponytail-review` done 2026-11-10: two findings fixed, R101 (code blocks) and R102 (a second paste).

## Manual checklist
- [ ] WRT-1.1 Replace puts the answer over the selection; Ctrl+Z undoes it. Windows ✓ (2026-10-26, a test window's text box, via a test build with the Echo model: "teh quick brown fox…" became the answer as plain text, with line breaks and dashes kept and Markdown marks gone; the clipboard came back unchanged; the bubble closed) ☐ Notepad, Word, a Chrome text box ☐ Ctrl+Z X11 ✓ (2026-10-26, WSL, mousepad, Echo: at first nothing was put in, "couldn't bring mousepad back to the front" (R66); after the fix the answer replaced the selection as plain text, Markdown marks gone and dashes and code kept, the clipboard came back unchanged, the bubble closed, and one Ctrl+Z restored the original. Also with a Hindi answer, LNG-3.4)
- [ ] WRT-1.3 With the app gone, nothing is pasted and the bubble says so. Windows ✓ (2026-10-26: the test window closed before Replace; "Cursora couldn't bring wrt-app back to the front, so nothing was put in. Copy the answer instead." in the bubble, the clipboard unchanged) X11 ✓ (2026-10-26, WSL: a second mousepad killed before Replace; the bubble said so, nothing went into the other mousepad window, the clipboard unchanged)
- [ ] WRT-2.1 Insert in an email or chat box with nothing selected. Windows ✓ (2026-10-26, the test window holding "Dear team," with the cursor after it: the answer went in there) ☐ a real email or chat app X11 ✓ (2026-10-26, WSL: mousepad holding "Dear team," with nothing selected; Insert put the answer after it, the clipboard unchanged)
- [ ] WRT-3.1 No Replace in Windows Terminal; Copy only. Windows ✓ by the user (2026-10-29, the installed 0.2.0: a word selected in a terminal, asked about; Copy under the answer, no Replace) X11 ✓ (2026-10-26, WSL, xterm: Replace was offered at first (R67); after the fix, Copy only)

## Notes
- The test copy ran with its own identifier and product name (`--config '{"identifier":"com.cursora.dev","productName":"Cursora Dev"}'`) and settings with Start with system off. With the same product name, a test copy would delete the installed copy's "Cursora" Run entry; with the same identifier, it would only hand its commands to the installed copy.
- A window hosted by `powershell.exe` gets no Replace, correctly: PowerShell is on the "Ctrl+C may stop a process" list. Test windows need their own exe.
