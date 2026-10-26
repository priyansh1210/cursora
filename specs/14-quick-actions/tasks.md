# 14: Quick actions (tasks)

- [x] 1. `quick.rs`: `kind`, `set`, `actions` and their tests; `QuickSettings`. Covers QAC-1.3, 2.1, 2.2, 3.1.
- [x] 2. `/reply` and `/error`; SKL-3.1 amended. Covers QAC-4.1, 4.2.
- [x] 3. `Conversation.quick`; the bubble's button row and Alt+N. Covers QAC-1.1, 1.2.
- [x] 4. Settings → Skills: the setting and Pin. Covers QAC-3.1, 3.2.
- [x] 5. Check in the bubble on Windows (2026-10-26, test copy and Echo), then `/ponytail-review` (three small fixes applied: typed text goes with the command, two dead table entries, a shorter label).

## Manual checklist
- [ ] QAC-1.1 Six buttons over a selection before the first question; Fix runs /fix and offers Replace. Windows ☑ (test window) X11 ☐
- [ ] QAC-1.2 Alt+1 to Alt+6 press them; tooltips show the keys. Windows ☑ (Alt+1, then Replace) X11 ☐
- [ ] QAC-3.2 The setting hides them; Pin puts a skill first. Windows ☑ X11 ☐
- [ ] Real apps: Windows Terminal (Explain error first), VS Code (Explain, Find bugs), Outlook or Slack (Reply first).
