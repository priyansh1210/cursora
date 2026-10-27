# 22: Personal profile (tasks)

Status: **In progress**.

- [x] 1. `about_you`; `prompt::system` and `assemble` with it, with tests. Covers PRF-1.2, PRF-1.3, PRF-1.4.
- [x] 2. About you in Settings → Personality; the manual. Covers PRF-1.1.
- [ ] 3. All checks; by hand on Windows; then `/ponytail-review`. 2026-10-28: the review found that typing and closing Settings with its × lost the text (checked in the real app); fixed for every field that saves on blur by blurring on close, then checked again: saved, and the window closed.

## Manual checklist
- [ ] PRF-1.1 About you in Personality: the placeholder, the line about where it goes, 600 characters at most, saved. Windows ✓ (2026-10-28, test copy: the box under the personality switch with the saved text and the note under it; the first build's box had no inner padding, fixed by putting it in a row) ☐ typing and saving by hand Linux X11 ☐
- [ ] PRF-1.4 "What will be sent" shows it, with an email hidden for a cloud model. Windows ✓ the prompt preview in Personality → Developer shows "About the user, in their own words" (2026-10-28) ☐ the bubble's "What will be sent" with a cloud model (the test copy has none; the redaction is unit-tested) Linux X11 ☐
