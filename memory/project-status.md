---
name: project-status
description: "Where Cursora stands as of 2026-10-25: working toward a public release; PRs #21 (spec 09 Optional) and #22 (memory in the repo) merged; PR #23 (stronger PC setup) open; test release tag v0.1.0-rc.1 not pushed yet (needs the user)"
metadata:
  type: project
---

**State on 2026-10-25:** `main` is at `4467d77`, CI green. **#21** (spec 09 Optional) and **#22** (this `memory/` folder, `scripts/link-memory.ps1`, CLAUDE.md rule) are merged. Open, waiting for the user to merge: **#23** (the stronger PC's setup in [[dev-machines]]). The stronger PC is set up and passed everything: npm ci, trace, JS tests, build, and the WSL Rust tests (144) as user `cursora`. Version 0.1.0, README says pre-alpha; nothing has ever been released.

**Goal: a public release, Windows first, Linux labelled beta.** The plan agreed on 2026-10-25, in order:
1. **Test release (issue 9):** tag `main` as `v0.1.0-rc.1`; CI's release job builds a draft release (msi, nsis, deb, rpm, AppImage, with llama-server). The auto mode classifier refused Claude's tag push ("Create Public Surface"), so the user runs `git tag v0.1.0-rc.1 origin/main` and `git push origin v0.1.0-rc.1`, or allows it. Then: check the run, install the .msi on Windows (also closes APP-3.8, LLM-3.1, APP-8.2 and issue 42's precondition), and the .deb/AppImage in WSL on the stronger PC, as user `cursora` ([[dev-machines]]). As of 2026-10-25 no `v*` tag exists on origin.
2. **A cloud model answering end to end (issue 16).** Gemini still gives server errors; asking the user whether the saved key may be used for one direct test request is still open. Groq or OpenRouter free keys are the alternative.
3. **First-time setup start to finish (APP-6.1)** on Windows.
4. **One-click model download in Setup** (new spec): today the built-in engine needs a .gguf the user finds on Hugging Face. Setup already knows the RAM size.
5. **Auto-update** (new spec, `tauri-plugin-updater`); must be in the first public build.
6. **Windows code signing:** Azure Trusted Signing, SignPath (free for open source), a certificate, or the Microsoft Store. Not chosen yet.
7. **Open source or not; a LICENSE** (none yet; repo is private). The user said the `memory/` folder might be removed from the repo later, e.g. before it goes public.
8. **Demo GIF in the README and a download page.**
9. **Issue 44; VS Code and Firefox (issue 8).** Ctrl+G is VS Code's "Go to line".

Post-launch feature picks offered to the user (not approved, no specs): per-app disable (backlog), replace in place (backlog), native Ollama (backlog), smart routing, sticky answers, a GPU (Vulkan) llama.cpp build now that a GPU PC exists, a "Report a problem" button that opens a prefilled GitHub issue.

**Open issues (16):** 1, 2, 6, 7, 8, 9, 10, 13, 16, 17, 33, 38, 40, 41, 42, 44. 1 needs a second monitor; 2, 10 and 33 need a real Linux desktop (the user chose WSL, so they stay open); 6, 7, 13 and 41 are watch items; 40 is optional.

**Other loose ends:** branch protection is refused for a private repo on the free plan (choices offered: GitHub Pro, going public, a local pre-push hook; not chosen). A real screenshot or GIF of the bubble and the snip overlay with the redesign hasn't been taken.

**How UI checks were done on the laptop:** the user's running Cursora is the debug build, theme Dark. Quit it with `src-tauri\target\debug\cursora.exe --quit` before `npx tauri build --debug --no-bundle` (the running exe is locked), then start it again. The 2026-10-16 session's scratchpad (`C:\Users\kabra\AppData\Local\Temp\claude\D--cursora\12b2a2d2-09c6-4da2-96f9-c23a9c3aaa31\scratchpad`) has `shot.ps1` (screenshot one Cursora window by name, optionally selecting a Settings tab through UI Automation) and `tour.ps1` (every Settings tab, Setup, and the chat window). Older X11 scripts are noted in [[after-restart-human-checks]].

**Why:** the user asked on 2026-10-25 to move on to the public release and to share memory between the two PCs.

**How to apply:** start the next session from step 1 of the plan (check whether the tag was pushed and how the release run went). Don't propose spec 09 ([[spec-09-voice-shelved]]).
