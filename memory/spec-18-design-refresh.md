---
name: spec-18-design-refresh
description: "2026-10-27: spec 18 (DSN), a full design refresh: approved and built (today's green, Geist font); PR open; manual checklist and the real-app look (issue 63) still to do"
metadata:
  type: project
---

**Asked 2026-10-27:** "work as a product designer with 10+ years of experience and follow product design principles to update the whole design of Cursora."

**Decided by the user, 2026-10-27:**
- **Font: Geist + Geist Mono**, chosen from nine options. They ship in `src/fonts/` (OFL), never fetched at run time. Indian scripts fall back to Nirmala UI.
- **Palette: today's green (Forest), refined.** "current color palette is fine", after reviewing twenty (twelve one-colour, eight two-tone). The two-tone machinery (`--accent-2`, a gradient) was built in the prototype and then removed as unused. Token values for all twenty, and the recoloured logos, are only on the review page: https://claude.ai/artifact/EFj5FeQYgZmYNGNGipXsEQ
- **Approved** with "continue with the work"; also the icon actions, "Download a free model" as Recommended, and Fluent's look on Linux too.

**Where it stands:** built on branch `spec-18-design-refresh`; every automated check passed on 2026-10-27 (JS, trace, build, cargo test, test:linux). Issues 60-62 became R74-R76. **Still open:** issue 63, the redesign has only been seen in a browser preview with Tauri's calls faked, never in the real app (skipped because the laptop was low on memory), plus spec 18's manual checklist on Windows and Linux X11.

**How to apply:** next session, run the real app (`npm run tauri dev`) and go through tasks.md's manual checklist with the user, one plain step at a time ([[plain-step-by-step-instructions]]). Fix what looks wrong in WebView2 or WebKitGTK, then close issue 63. If the user wants another palette later, only the colour tokens and the logo's five colours change, and the icon files are regenerated from `icon.svg`. Related: [[product-direction]], [[no-emoji-in-ui]].
