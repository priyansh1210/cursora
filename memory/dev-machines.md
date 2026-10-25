---
name: dev-machines
description: "Two dev PCs share this memory through git: the laptop (PRIYANSH) does the Windows work; the stronger PC, reached over Chrome Remote Desktop, does the Linux work in WSL"
metadata:
  type: project
---

Since 2026-10-25 Cursora is developed on two PCs, each with its own Claude Code, sharing this memory through the repo's `memory/` folder (linked with `scripts/link-memory.ps1`; rules in CLAUDE.md).

- **The laptop** (hostname `PRIYANSH`, repo at `D:\cursora`): 8 GB RAM, i5-11300H, Windows 11 Home, C: nearly full ([[keep-big-files-off-c-drive]]). Does the Windows work and the Windows hands-on checks. Its WSL is slow and long runs get killed for low memory.
- **The stronger PC** (better CPU and GPU; the user reaches it with Chrome Remote Desktop): does the Linux work in WSL, chosen by the user on 2026-10-25 over a VirtualBox Ubuntu VM. So issue 2 (a real Linux desktop) stays open. Its hostname, specs and repo path aren't recorded yet: the Claude there should add them here.

**Why:** the laptop is short of RAM and disk; the user offered the stronger PC for heavy work.

**How to apply:** check `hostname` at the start of a session to know which PC this is, and read machine-specific notes (paths, scratchpads, installed tools) only for that PC. Put heavy builds and WSL runs on the stronger PC. Related: [[project-status]].
