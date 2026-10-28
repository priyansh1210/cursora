---
name: improvements-batch-2
description: "2026-10-28 go-ahead: build updates 1 and 3-9 (voice input, snip OCR, GPU engine, per-app off, profile, web search, smart routing, hover to define), each specced, built, tested, PR'd and merged by Claude; the user reviews all at the end; ponytail throughout"
metadata:
  type: project
---

**The user's words (2026-10-28):** "we will not do the 2nd one [whole-document questions], others are good so lets go for 1 to 9 all leave 2. I want you to start working on them, design them, build them and raise PR and then merge them, do all the tests, and I will be watching you ... and reviewing as well, and later when everything is finished I will review everything again even if the PRs are merged. I MUST WANT YOU TO USE Ponytail skill throughout the whole work."

**The eight updates** (from the list of 10 given that day; #2 dropped, #10 usage tracker not included):
1. Voice input only (speech-to-text into the bubble, local Whisper); no spoken answers, not spec 09 ([[spec-09-voice-shelved]])
3. Snip with any model (OCR when the model can't read images)
4. Faster local answers with a graphics card (GPU build of the engine)
5. Turn Cursora off per app (the hotkey passes through, e.g. Ctrl+G in VS Code)
6. Personal profile (a few lines about the user in every prompt)
7. Web search with sources (works with local models too; off in local-only mode; needs a §3 amendment)
8. Smart routing (quick jobs to the local model, the rest to the default; local fallback offline)
9. Hover to define (the word under the pointer, no selection)

**How to apply:** this go-ahead covers approving each spec (mark them Approved with a "Defaults taken under the go-ahead of 2026-10-28" list, like specs 13-15's ¹) and merging the PRs, which overrides the usual "the user merges" rule ([[working-rules-cursora]]) for these eight only. Use `/ponytail` (full) for every change and `/ponytail-review` before marking a spec Done. Run all checks before each push; Linux X11 hand checks go to the stronger PC ([[dev-machines]]) and are logged as open. The user is watching: keep them posted in short lines.

**Linux checks (the user, 2026-10-28):** "for these updates you can do the Linux tests here, and if there are Linux tests which here are not possible we will do them on that system later, cause as of now I don't have access to that system." So run Linux checks in the laptop's WSL (`Ubuntu-24.04`, user `priyansh`, no sudo), and log what can't run there as open items for the stronger PC.

**Progress:** #1 voice input = spec 19 (VIN), merged 2026-10-28 (PR #56). #3 snip with any model = spec 20 (OCR), merged (PR #57). #5 per-app off = spec 21 (OFF), merged (PR #58). #6 profile = spec 22 (PRF), merged (PR #59). #4 GPU = spec 23 (GPU), merged (PR #60); the driver check runs in a child (`cursora --gpu`) to keep Cursora under 60 MB. **Paused here by the user (2026-10-28): "after completing this update don't start the next one".** Wait for their word before #8 routing, #9 hover to define, #7 web search. Open checks for the user: issue 64 (a Hindi sentence with Accurate; the microphone blocked in Windows' privacy settings); Linux app checks on the stronger PC: issues 64, 65, 67, 68; the tray item "Turn Cursora off in <app>" (issue 67). Issue 66: an unreadable history file is only logged.
