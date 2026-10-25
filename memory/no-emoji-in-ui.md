---
name: no-emoji-in-ui
description: "The user finds emoji unprofessional: no emoji anywhere in Cursora's UI; use line icons (src/icons.tsx) or plain words"
metadata:
  type: feedback
---

Don't use emoji in Cursora's interface: no emoji in buttons, badges, chips, tray labels or status messages. Use the line icons in `src/icons.tsx` or plain words instead.

**Why:** on 2026-10-16, during the grey and green redesign, the user said "dont use emojis they look very unprofessional". Emoji also showed as boxes on Linux without an emoji font (issue 14, now R59).

**How to apply:** when adding UI, reach for `Icon`/`PATHS` from `src/icons.tsx` or text. Related: [[project-status]].
