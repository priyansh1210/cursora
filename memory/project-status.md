---
name: project-status
description: "Where Cursora stands as of 2026-10-25: working toward a public release; PRs #21-#23 merged; test release v0.1.0-rc.1 built and its installers checked (draft release); next: install the .msi on the laptop"
metadata:
  type: project
---

**State on 2026-10-25:** `main` is at `f913fcc`, CI green. **#21** (spec 09 Optional) and **#22** (this `memory/` folder, `scripts/link-memory.ps1`, CLAUDE.md rule) are merged. **#23** (the stronger PC's setup in [[dev-machines]]) is merged too. The stronger PC is set up and passed everything: npm ci, trace, JS tests, build, and the WSL Rust tests (144) as user `cursora`. Version 0.1.0, README says pre-alpha; nothing is public yet.

**Goal: a public release, Windows first, Linux labelled beta.** The plan agreed on 2026-10-25, in order:
1. **Test release: built.** The user pushed `v0.1.0-rc.1` (on `f913fcc`) on 2026-10-25; run 40202245085 was green and made draft release 423458339 (.msi, NSIS setup.exe, .deb, .rpm, AppImage). The .msi, .deb and AppImage contain the engine (R60, was issue 9; APP-8.2 and LLM-3.1 ticked). New: issue 45 (Linux packages carry 3 copies of each engine library), issue 46 (setup.exe not inspected; per-user, so maybe ship only the .msi). **Still to do:** install the .msi on the laptop (quit the debug build first; checks APP-3.8, issue 42 and, with Settings -> Re-run setup, APP-6.1), and the .deb/AppImage in WSL on the stronger PC as user `cursora` (LNX-5.1). A browser download of the .msi also shows what SmartScreen says to new users (unsigned). The release PR for these notes: `release-rc1-check`.
2. **A cloud model answering end to end (issue 16).** Gemini still gives server errors; asking the user whether the saved key may be used for one direct test request is still open. Groq or OpenRouter free keys are the alternative.
3. **First-time setup start to finish (APP-6.1)** on Windows.
4. **One-click model download in Setup** (new spec): today the built-in engine needs a .gguf the user finds on Hugging Face. Setup already knows the RAM size.
5. **Auto-update** (new spec, `tauri-plugin-updater`); must be in the first public build.
6. **Windows code signing:** Azure Trusted Signing, SignPath (free for open source), a certificate, or the Microsoft Store. Not chosen yet.
7. **Open source or not; a LICENSE** (none yet; repo is private). The user said the `memory/` folder might be removed from the repo later, e.g. before it goes public.
8. **Demo GIF in the README and a download page.**
9. **Issue 44; VS Code and Firefox (issue 8).** Ctrl+G is VS Code's "Go to line".

Post-launch feature picks offered to the user (not approved, no specs): per-app disable (backlog), replace in place (backlog), native Ollama (backlog), smart routing, sticky answers, a GPU (Vulkan) llama.cpp build now that a GPU PC exists, a "Report a problem" button that opens a prefilled GitHub issue.

**Open issues (17):** 1, 2, 6, 7, 8, 10, 13, 16, 17, 33, 38, 40, 41, 42, 44, 45, 46. 1 needs a second monitor; 2, 10 and 33 need a real Linux desktop (the user chose WSL, so they stay open); 6, 7, 13 and 41 are watch items; 40 is optional.

**Other loose ends:** branch protection is refused for a private repo on the free plan (choices offered: GitHub Pro, going public, a local pre-push hook; not chosen). A real screenshot or GIF of the bubble and the snip overlay with the redesign hasn't been taken.

**How UI checks were done on the laptop:** the user's running Cursora is the debug build, theme Dark. Quit it with `src-tauri\target\debug\cursora.exe --quit` before `npx tauri build --debug --no-bundle` (the running exe is locked), then start it again. The 2026-10-16 session's scratchpad (`C:\Users\kabra\AppData\Local\Temp\claude\D--cursora\12b2a2d2-09c6-4da2-96f9-c23a9c3aaa31\scratchpad`) has `shot.ps1` (screenshot one Cursora window by name, optionally selecting a Settings tab through UI Automation) and `tour.ps1` (every Settings tab, Setup, and the chat window). Older X11 scripts are noted in [[after-restart-human-checks]].

**Why:** the user asked on 2026-10-25 to move on to the public release and to share memory between the two PCs.

**How to apply:** start the next session from the rest of step 1 (installing the rc1 installers), then step 2. Don't propose spec 09 ([[spec-09-voice-shelved]]).
