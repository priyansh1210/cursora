---
name: working-rules-cursora
description: "The user's standing rules for Cursora work not written in CLAUDE.md (one name per thing, no macOS, reaper, Prettier, test input, the user merges PRs)"
metadata:
  node_type: memory
  type: feedback
  originSessionId: 13cf765a-4fc3-4f3c-83f4-17c71dfbf14b
  modified: 2026-10-14T03:46:23.458Z
---

- **One name per thing:** "make sure that we using same variable not using different variables for same thing" — reuse an existing type or constant (e.g. `bubble::Rect`, `ModelRef`) instead of adding a second one for the same concept.
- **No macOS:** "we are not working on macOS that is for the future" — Windows and Linux only.
- **Memory reaper:** if a long run (WSL e2e, builds) is killed for low memory, ask before restarting it.
- **Prettier:** the repo doesn't use Prettier; ignore its warnings.
- **Test automation never types into the user's own apps**; scripts guard the foreground window (see specs/issues.md test notes).
- **Never merge PRs yourself:** "from now no self merging the prs - i will merge prs" (2026-10-26). Open the PR, wait for CI, then tell the user it is ready; the user merges. The overnight permission for specs 13 to 15 has ended.

**Why:** stated by the user across sessions while building specs 01–10.

**How to apply:** check these before adding types, platform code, or automated UI checks. Related: [[no-ai-attribution-in-commits]], [[keep-big-files-off-c-drive]].
