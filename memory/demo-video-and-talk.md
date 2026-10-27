---
name: demo-video-and-talk
description: Where the 91-second demo video, its recording/editing scripts, the voice-over script and the class presentation script live (made 2026-10-26)
metadata:
  type: reference
---

- **Demo video:** `D:\cursora-demo\Cursora-demo.mp4`, 91 s, 1080p30, captions, no sound yet. Seven scenes (ask, fix, hindi, snip, skills, pin, privacy) plus a title card (7 s) and an end card (9 s) timed to the voice-over. It shows specs 16 and 17, which released v0.1.1 lacks, so make a new release before posting it publicly (tags need the user).
- **How it was made (all in `D:\cursora-demo`):** `scenes.ps1 -Scene <name> -Record` drives Cursora's release build and a Chrome window with its own profile (`chrome-profile`, local pages in `pages/`), recording with ffmpeg gdigrab without the pointer; `overlay.py` draws the pointer back from the logged positions (gdigrab draws the I-beam as a white box); `quiet.py` finds the waits to cut; `make_video.py` trims, speeds up, captions and joins. `demo-settings.py` swaps in demo settings and restores them by file copy only (removing the provider in Cursora would delete the user's Gemini key). ffmpeg is in `D:\tools\ffmpeg`.
- **Voice-over:** two voices, Priyansh and Nipun (a teammate), 20 timed lines: https://claude.ai/artifact/GLku1dVuUsa5dued5xdRG8 . Recordings, once the user makes them, go in `D:\cursora-demo\voice\` to be mixed in.
- **Class talk:** Design Thinking HUM_4342 (mentor Dr. Vandana Jha, MIT Bengaluru), deck `D:\PRIYANSH!!!!\DOWNLOADS\Design_Thinking_Presentation.pptx` (21 slides). The 15-minute two-voice script with the video in it, delivery tips and likely questions: https://claude.ai/artifact/KpZBXdV4D6WEP6tJu7r7zW . The user shares it with Nipun.
- The user's `forthereport.md` (features only, for their report) is untracked in the repo root: never commit it. See [[project-status]].
