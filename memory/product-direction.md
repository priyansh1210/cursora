---
name: product-direction
description: "2026-10-26: Claude's honest assessment of Cursora (well built, idea not unique, crowded market, setup/trust are the blockers) and the user's direction: make it far more useful before launching; Store work paused; Hugging Face considered for later; the 10 agreed improvement ideas"
metadata:
  type: project
---

**The assessment (asked for "100% honest", 2026-10-26):**
- **Good:** engineering quality well above indie apps: specs, ~140 tests, privacy guard (redaction incl. Aadhaar/PAN, blocklist, local-only), clipboard and terminal safety, built-in offline engine, signed auto-update. It does more than rivals: a conversation at the cursor, files, snip, skills, personas, history, local and cloud models.
- **Not unique:** select text + hotkey + AI is crowded, often free. Microsoft Copilot and Click to Do, the ChatGPT app (Alt+Space). Free open-source Windows apps: QuillBar, ReWrite AI, CapyBro, AI Shortcuts, Writing Tools. Already in the Microsoft Store: Rewire Text, Rewrait. Browser extensions (Monica, Sider, Merlin) with millions of users.
- **Downloads:** organically ~none. A good launch might bring a few hundred to a couple of thousand, with fewer staying (a guess).
- **What loses users:**
  1. setup (finding a model or a key)
  2. trust (unknown, unsigned, closed source, reads every app, looks like spyware to cautious users and antivirus)
  3. good-enough free alternatives
  4. weak small local models
- **What would change the odds:**
  1. effortless first 5 minutes (one-click model, offline, no account)
  2. reconsider open source (trust, free SignPath signing, discovery)
  3. replace in place
  4. one target audience (e.g. Indian students/professionals, developers, privacy-sensitive jobs)
  5. a 15-second demo video
- **For the user:** a strong portfolio piece whatever the downloads.

**The user's direction (2026-10-26):** "we will work on improving it a lot", meaning make it much more useful (rewrite and edit the selection, and more) before launching. **Store implementation paused**: spec 12 requirements approved and design drafted (PR #39), not built. The user asked about **publishing on Hugging Face instead of the Microsoft Store, later**. Claude's answer: Hugging Face doesn't remove Windows' SmartScreen warning, which depends on the file's signature, not where it's downloaded. Hugging Face is great for hosting the models of a one-click download and for reaching AI enthusiasts (a Space, posts), but it isn't a desktop app store.

**The 10 improvements offered (priority order suggested):**
1. Replace in place
2. Quick actions without typing
3. One-click model download
4. Insert at the cursor (write without a selection)
5. App-aware suggestions
6. Personal profile and memory
7. Indian languages
8. Whole-document questions
9. Web search with sources
10. Save and pin answers

Quick wins beside them: per-app disable (Ctrl+G stays "Go to line" in VS Code), and OCR so snip works with any model. All are in specs/backlog.md.

**How to apply:** next work is these improvements, one spec at a time (requirements first, the user approves), starting where the user picks. Don't push the Store build until the user brings it back. Keep the assessment honest when asked again; don't oversell. Related: [[project-status]], [[spec-09-voice-shelved]] (voice stays out of these lists).
