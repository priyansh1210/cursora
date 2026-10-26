# 15: One-click model download (tasks)

- [x] 1. `download.rs`: `MODELS`, `recommended`, `total_ram`, `fetch` and their tests. Covers MDL-1.2, 1.3, 1.5, 1.6, 1.7, 2.1.
- [x] 2. The commands, the `Download` state and `model-progress`.
- [x] 3. "Download a free model" in `AddProvider` (Setup and Settings → Models). Covers MDL-1.1, 1.4.
- [x] 4. Constitution §3 amendment; spec 04 and the backlog point here; manual and README.
- [x] 5. Check on Windows: download Small for real, cancel and continue, ask a question (2026-10-26: 190 s, SHA-256 confirmed with `certutil`, the engine answered). Then `/ponytail-review` (fixed: a late progress event could leave the bar stuck; a test for Cancel during silence; two unused fields).

## Manual checklist
- [ ] MDL-1.1 The three choices, Recommended, the folder and Change…, in Setup and Models. Windows ☑ (Models; Setup uses the same part) X11 ☑ (2026-10-26, WSL: Models and Setup's step 1 both; Medium marked Recommended on 31 GB)
- [ ] MDL-1.4 Progress and Cancel; Download again continues. Windows ☑ (cancelled at 178 MB, continued from 16%) X11 ☑ (2026-10-26, WSL: "0.9 of 4.7 GB" with Cancel; Large cancelled at 900 MB (19%), "Paused. Download again to continue where it stopped."; two seconds after Download again the part was 1.3 GB, so it continued)
- [x] Medium and Large downloaded and answering once each. 2026-10-26, stronger PC: in the Linux app (WSL, X11), each downloaded in Settings → Models, kept after its SHA-256 check (about 15 s for Medium in the debug build) and added as a Built-in engine source; each answered in the bubble ("Attendee brings signed contract.", "Bring signed contract to Friday meeting."). Also downloaded on Windows with curl from the pinned URLs into `C:\Users\FITT\Models\cursora`, SHA-256 matching `download.rs`, and answering through `llama-server.exe` in the spec 16 trial. Found: issue 56, the downloaded model doesn't become the default.
