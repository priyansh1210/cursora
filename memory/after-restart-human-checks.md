---
name: after-restart-human-checks
description: "Run as administrator status: PRs #13 and #14 merged; Program Files only (APP-3.8), so the dev build no longer runs as admin; old task left to the user"
metadata:
  node_type: memory
  type: project
  originSessionId: 9ecfc2a0-9602-47cf-8ec9-2ec65e07b35e
  modified: 2026-10-15T06:21:26.103Z
---

Run as administrator (APP-3.3–3.7) was merged in PR #13 on 2026-10-15. Issue 39's fix, PR #14, was merged the same day. Under it, the setting only works when the exe is in Program Files (APP-3.8): the user picked that over also making NSIS per-machine.

The dev build in `D:\cursora` now ignores the ticked box and runs as a normal app; Settings greys the box out and gives the reason. Testing Run as administrator, including issue 42's "task deleted by hand", now needs the .msi installed in Program Files. The APP-3.8 .msi check is open on spec 01's checklist.

The user pointed out that the old build starting as admin was correct, not a bug: the setting was on. Don't call a ticked setting doing its job a "hole".

**Why:** the user wants Cursora to stay admin across restarts until they turn it off, with Windows asking only once, but only where that can't be hijacked.

**How to apply:** the dev PC still has the old task `Cursora (administrator)`. The user didn't say whether to delete it: leave it to them and don't delete it unasked. The new build never runs it. Related: [[project-status]], [[plain-step-by-step-instructions]], [[github-pr-without-gh]].
