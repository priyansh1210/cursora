---
name: spec-09-voice-shelved
description: "Spec 09 (voice in/out) is Optional, by the user's decision (shelved 2026-10-15, made Optional 2026-10-25); don't propose it as remaining work"
metadata:
  type: project
---

On 2026-10-15 the user said spec 09 (voice) won't be worked on for now: "narrator is not that useful". On 2026-10-25 they asked to make it an optional spec, and PR #21 added an **Optional** status to specs/README.md (approved, not part of the release, built only if asked for); `npm run trace` treats it as not started.

**Why:** the user doesn't see enough value in spoken answers yet, and wants to focus on the public release.

**How to apply:** don't list spec 09 as remaining work or offer to start it. Build it only if the user brings it up. If they do, the design's first step still applies: check the versions of `whisper-rs`, `cpal` and `tts` again.
