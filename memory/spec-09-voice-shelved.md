---
name: spec-09-voice-shelved
description: "Spec 09 (voice in/out) stays Optional and unused; 2026-10-28 the user wants voice INPUT only (dictating questions), as its own new spec, with no spoken answers"
metadata:
  type: project
---

On 2026-10-15 the user said spec 09 (voice) won't be worked on for now: "narrator is not that useful". On 2026-10-25 they asked to make it an optional spec, and PR #21 added an **Optional** status to specs/README.md (approved, not part of the release, built only if asked for); `npm run trace` treats it as not started.

**2026-10-28:** asked for 10 more updates "including voice input", the user said: "leave that voice spec and I just want a voice input, I don't want voice output as of now."

**Why:** the user doesn't see enough value in spoken answers, but does want to dictate questions.

**How to apply:** voice input (speech-to-text into the bubble) is a fair item to propose and build, as a new spec of its own, not by reviving spec 09. Leave text-to-speech, voices per persona and spec 09 itself alone unless the user brings them up. Spec 09's design notes can still inform it: check the versions of `whisper-rs` and `cpal` again before designing.
