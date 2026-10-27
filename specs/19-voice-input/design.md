# 19: Voice input (design)

Status: **Approved**¹ (2026-10-28). Implements [requirements.md](requirements.md). The microphone is read in Rust with `cpal`, and whisper.cpp (through `whisper-rs`) is compiled into Cursora, so nothing new runs as a separate program. The speech model comes through spec 15's download.

## Overview
```
Bubble (Chat.tsx): [mic] [send]
  no voice_model      → open_settings("General")                                      (VIN-1.6)
  mic                 → voice_start   → a thread owns the cpal input stream, samples to memory, 60 s cap
  stop (or 60 s)      → voice_stop    → the thread ends → to_16k_mono → whisper (spawn_blocking)
                                     → clean() → the text, added after the draft       (VIN-1.2)
  bubble hides        → voice::cancel → the thread ends, the samples are dropped        (VIN-1.5)
  nothing stops it    → after 65 s the thread closes the microphone and drops the samples (VIN-1.1)
Settings → General → Voice input: DownloadModel kind="speech" → model_download("fast"|"accurate") → voice_model
```

## Components
| File | Change |
|---|---|
| `src-tauri/src/voice.rs` (new) | `Voice` state (the running recording). `to_16k_mono()`, `audio_ctx()`, `clean()`, `transcribe()`, `cancel()`. Commands `voice_start` and `voice_stop`. |
| `src-tauri/src/llm/download.rs` | `SPEECH`: the two pinned whisper models. `model_choices` returns them too, and `model_download` finds an id in either list. The part file is `<file>.part` for any extension. |
| `src-tauri/src/settings.rs` | `voice_model: Option<String>`, the speech model's path. |
| `src-tauri/src/bubble/mod.rs` | Hiding the bubble cancels a recording. |
| `src-tauri/src/lib.rs` | Registers the commands and manages `Voice`. |
| `src/Chat.tsx` | The mic button and its states; `appendDictation()` (in `conversation.ts`). |
| `src/Models.tsx` | `DownloadModel` takes `kind`: the chat or speech choices, sizes in GB or MB, and its own note. |
| `src/Settings.tsx` | The Voice input group in General. |
| `src/icons.tsx` | The mic path. |
| `.cargo/config.toml` (new) | How whisper.cpp is compiled (decision 4). |
| `.github/workflows/ci.yml` | `libasound2-dev` on Linux; `WHISPER_DONT_GENERATE_BINDINGS` on Linux. |
| `scripts/wsl-test.sh` | Sources `~/.cursora-env` when it exists (build tools without sudo). |
| `specs/00-constitution.md` | §3 amendment: speech models too. |

## Recording
`voice_start` spawns a thread that opens the default input device with its own default config, since every device supports that. The stream's callback appends samples to a `Vec<f32>` behind a mutex, dropping any past 60 s × the device rate × channels, so memory stays bounded even if Stop never comes. The thread waits on a channel for Stop or Cancel, for 65 seconds at most. Stop returns the samples. Cancel drops them, and so does running out of time, so the microphone never stays open: the expanded window has the mic too, and closing it sends nothing. `voice_start` drops any recording still in the slot before it starts. The thread owns the stream because a `cpal::Stream` isn't `Send` on every platform. Errors from opening or starting the stream come back through the same channel before the thread waits, so `voice_start` can report them (VIN-1.4).

`to_16k_mono` averages the channels and resamples by linear interpolation to the 16 kHz whisper expects. Speech needs nothing better.

## Turning speech into text
`transcribe` loads the model for each recording and drops it after, so the 60 to 190 MB are held only while it works (constitution §8). The parameters:
- greedy decoding, language `auto` (VIN-2.3)
- every processor thread (8 was faster than 4 on the laptop)
- `audio_ctx = min(1500, seconds × 50 + 128)`: whisper otherwise reads a 30-second window, so a 5-second question took 8.6 s instead of 5.3 s with Accurate
- `suppress_nst`, no timestamps, no printing

`clean` joins the segments, removes whole bracketed or parenthesised marks (`[BLANK_AUDIO]`, `(music)`, `[Music]`), and collapses spaces. An empty result is "Didn't catch that. Try again, a little closer to the microphone."

## Decisions
1. **Whisper compiled in, not a separate program.** whisper.cpp publishes ready-made programs for Windows but not Linux, and `whisper-rs` builds the same code on both. Rejected: `whisper-server` as a child process like llama-server.
2. **`cpal` in Rust, not the webview's microphone.** WebView2 shows its own permission prompt, and WebKitGTK needs its media settings turned on and permission requests answered. `cpal` behaves the same in both, and the recording never enters the page.
3. **The mic shows even before setup**, and opens Settings then (VIN-1.6). A hidden feature is never found (constitution §4 allows "disable themselves … and explain why").
4. **Build settings for whisper.cpp in `.cargo/config.toml`:**
   - `GGML_NATIVE=OFF` with AVX, AVX2, FMA and F16C on, so the build runs on any x86-64 PC from about 2013 on, not just the build machine.
   - `CMAKE_C_FLAGS_RELEASE` and `CMAKE_CXX_FLAGS_RELEASE` = `-O2 -DNDEBUG`. The `cmake` crate replaces CMake's release flags with its own, without optimization, and a 5-second question then took 72 s instead of 9 s. Both MSVC and gcc accept `-O2`.
5. **Bindings:** on Windows, `whisper-rs-sys` generates its bindings with libclang, since its bundled ones were made on Linux, where `long` is 8 bytes. GitHub's Windows runners have LLVM. On Linux the bundled ones are used (`WHISPER_DONT_GENERATE_BINDINGS=1`), so no libclang is needed.

## Developer setup
- **Windows:** CMake (Visual Studio Build Tools has one) and libclang (LLVM, or `pip install libclang` with `LIBCLANG_PATH` pointing at its `clang/native` folder). Set `CMAKE` to cmake.exe if it isn't on the PATH.
- **Linux:** `cmake`, `libasound2-dev` and `WHISPER_DONT_GENERATE_BINDINGS=1`. Without sudo, `~/.cursora-env` can add them from a home folder (issues.md, test environment notes).

## Errors
| Case | Shown |
|---|---|
| No input device | "No microphone was found." |
| The device fails to open or start | "Cursora couldn't use the microphone: <reason>. On Windows, check Settings → Privacy & security → Microphone → Let desktop apps access your microphone." |
| The model file is missing | "The speech model is missing. Download it again in Settings → General → Voice input." |
| Nothing heard | "Didn't catch that. Try again, a little closer to the microphone." |

## Test strategy
- Rust: `to_16k_mono`, `audio_ctx` and `clean` (VIN-1.2); the source of `voice.rs` writes no files and opens no connections (VIN-1.3); a recording thread is stopped and its samples dropped by cancel (VIN-1.5), and when nothing stops it in time (VIN-1.1); the speech models are pinned (VIN-2.2).
- TS: `appendDictation` (VIN-1.2).
- By hand on Windows: dictation with both models, English and Hindi; no microphone; the bubble closing mid-recording. In WSL: the build and the tests. A microphone on Linux needs a real desktop (issue 2).

¹ Approved under the user's go-ahead of 2026-10-28 (requirements.md).
