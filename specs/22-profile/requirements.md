# 22: Personal profile (PRF)

Status: **Approved**¹ (2026-10-28).

## Scope
- **In:** a few lines about the user, written once in Settings, which every answer takes into account: who they are, what they work on, how they like things explained.
- **Out:**
  - Memory that learns by itself from conversations
  - Several profiles (work, study) to switch between
  - A profile per persona: the profile is about the user, the persona about Cursora
  - Asking for it in the first-time setup

Update #6 from the list of 2026-10-28 ([improvements batch 2](../../memory/improvements-batch-2.md)).

## Requirements

### PRF-1 About you
User story: As a computer science student, I want Cursora to know that once, so I don't have to say it in every question.
- PRF-1.1 (manual) Settings → Personality SHALL have **About you**: a box for up to 600 characters, empty by default, with an example as its placeholder and a line saying it goes with every question, to cloud models too.
- PRF-1.2 (auto) WHILE About you isn't empty THE system message SHALL include it as the user's own words, after the persona and before a skill's instructions, with personality on or off: it describes the user, not the persona (PER-2.1).
- PRF-1.3 (auto) At most 600 characters SHALL be sent, even from a settings file edited by hand.
- PRF-1.4 (auto) Personal details in it SHALL be hidden from cloud models like the rest of the request (PRI-3.1), and "What will be sent" SHALL show it (PRI-3.6).

## Defaults taken under the go-ahead of 2026-10-28
1. **One box of free text, up to 600 characters**: enough for a few lines, short enough to cost few tokens with every question.
2. **Sent with personality off too**, since it's about the user, not Cursora's character.
3. **In Settings → Personality**, beside the personas, where how Cursora answers is set.

## Open questions
None.

¹ Approved under the user's go-ahead of 2026-10-28 to design, build and merge updates 1 and 3 to 9, with the user reviewing everything afterwards. The choices made for the user are listed above.
