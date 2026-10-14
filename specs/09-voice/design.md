# 09: Voice in/out (design)

Status: **Approved** (2026-10-14), **not started**: the user chose to build it later. When it starts: check the versions of `whisper-rs`, `cpal` and `tts` again (the model hashes are pinned and don't change), then write `tasks.md`. Implements [requirements.md](requirements.md). The push-to-talk hotkey already has its setting (spec 01); its field in Setup and Settings → General is hidden until this spec is built (issue 37).

## Overview
```
mic button / push-to-talk held ─→ dictate_start: the default microphone, kept in memory as 16 kHz mono (VOC-1.7)
    the bubble: a level meter and the time (VOC-1.2); stops at maxSeconds, on the button, on release, or when hidden
dictate_stop ─→ route() (VOC-1.5)
    this computer: whisper.cpp in process (whisper-rs)
        model not downloaded → the bubble asks first, shows size and progress; SHA-256 checked (VOC-1.4)
    a provider:    WAV built in memory → POST {base}/audio/transcriptions, through net's guard (PRI-1.1)
        "Confirm before sending to cloud" on → the bubble asks first (PRI-3.7)
    the audio is dropped here, whatever happened (VOC-1.7)
 ─→ the text goes into the input; sent only if "Send after dictation" is on (VOC-1.3)

answer streams in the bubble ─→ speech.ts: markdown stripped, split into sentences (VOC-2.2)
    Speak, or Auto-speak ─→ speak(sentence, persona) ─→ pick_voice() (VOC-2.4) ─→ the speech thread (tts crate)
    Stop, or the bubble hidden ─→ silence at once (VOC-2.3)
```

## Components

| File | Job |
|---|---|
| `src-tauri/src/voice/mod.rs` (new) | The commands; the state: a recording or a held one, the speech thread, the voice notes already shown; `stop_all()` for closing the bubble. |
| `src-tauri/src/voice/record.rs` (new) | `cpal` input at the device's own format, mixed down to mono and resampled to 16 kHz in memory; the level and the time every 50 ms; the stop at `maxSeconds`; `mic_problem()` (VOC-1.6). |
| `src-tauri/src/voice/stt.rs` (new) | `route()` (pure); local transcription with `whisper-rs`; cloud transcription; `wav()` (16-bit PCM in memory). |
| `src-tauri/src/voice/models.rs` (new) | The whisper models (name, URL at a pinned revision, size, SHA-256); `installed()`; `download()` with progress and the hash check. |
| `src-tauri/src/voice/speak.rs` (new) | The speech thread, which owns `tts::Tts` (its Windows objects can't move between threads): speak, stop, voices, rate and volume. `pick_voice()` (pure). |
| `src-tauri/src/llm/net.rs` | `upload()`: a multipart POST through the same guard and client as chat. `download()`: a user-started download to a file (PRI-4.1), streamed, no provider guard. |
| `src/HotkeyInput.tsx` | The Push-to-talk field comes back in Setup and General → Hotkeys, with its × to clear it (removed while this spec waited, issue 37). |
| `src-tauri/src/hotkey.rs` | Registers push-to-talk when it's set, Cursora isn't paused and the session isn't Wayland. Pressed: Ask (if the bubble is closed), then record. Released: stop. |
| `src-tauri/src/bubble/mod.rs` | Closing the bubble calls `voice::stop_all()`: speech stops, a recording or held audio is discarded. |
| `src-tauri/src/persona.rs` | `Persona.voice`: a voice, rate and volume of its own (VOC-2.5), validated. |
| `src-tauri/src/settings.rs` | `VoiceSettings`. |
| `src/speech.ts` (new) | `speakable()`, the streaming `Sentences` splitter (VOC-2.2) and `afterDictation()` (VOC-1.3). Pure, tested with vitest. |
| `src/Chat.tsx` | The mic button, meter, time and states; the model offer and its progress; the cloud confirm; **Speak**/**Stop** on each answer; Auto-speak while streaming; the voice note. |
| `src/VoiceTab.tsx` (new), `src/Settings.tsx` | Settings → Voice. |
| `src/Personality.tsx` | The persona's voice, rate and volume. |
| `.cargo/config.toml`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `.github/workflows/ci.yml` | Dependencies, build settings, the .deb dependency, CI packages. |

## Data

### Settings
```jsonc
"voice": {
  "whisperModel": "base",       // "tiny" | "base" | "small", all multilingual (VOC-1.4)
  "sttProvider": null,          // null: this computer; else the id of an OpenAI-compatible provider (VOC-1.5)
  "sttModel": "whisper-1",      // that provider's transcription model (Groq: "whisper-large-v3-turbo")
  "maxSeconds": 60,             // 5 to 300 (VOC-1.2)
  "sendAfterDictation": false,  // VOC-1.3
  "autoSpeak": false,           // VOC-2.1
  "defaultVoice": null,         // a voice id; null: the system's default
  "rate": 1.0,                  // 0.5 to 2.0, times the engine's normal rate
  "volume": 1.0                 // 0.0 to 1.0
}
```
`hotkeys.pushToTalk` already exists (null: none).

### Persona
```jsonc
"voice": { "id": null, "rate": null, "volume": null }   // each null: the Voice tab's setting (VOC-2.5)
```
Older persona files have no `voice` and load with it empty. Rate and volume are checked against the same ranges as the settings.

### Whisper models
| Model | Size | SHA-256 |
|---|---|---|
| `tiny` | 77,691,713 bytes (78 MB) | `be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21` |
| `base` | 147,951,465 bytes (148 MB) | `60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe` |
| `small` | 487,601,967 bytes (488 MB) | `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b` |

From `https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-<model>.bin` (a fixed revision, so the hash can't change under us). Stored as `<app data>/whisper/ggml-<model>.bin`.

### `voice_status`
```jsonc
{
  "mic": null,              // or why the mic button is disabled (VOC-1.6)
  "speech": null,           // or why Speak is hidden (VOC-2.6)
  "models": { "tiny": false, "base": true, "small": false },   // downloaded
  "voices": [{ "id": "…", "name": "Microsoft Zira", "language": "en-US", "gender": "female" }]   // gender null on Linux
}
```

## Key mechanics

### Recording (VOC-1.1, 1.2, 1.6, 1.7)
- The default input device, at its own sample rate and format (WASAPI in shared mode only offers the mix format). Each block is mixed down to mono and resampled to 16 kHz into a `Vec<f32>`. Nothing is written anywhere.
- Every 50 ms the bubble gets `{ level, ms }`: the RMS of the last 50 ms (scaled to 0–1) and the time recorded. At `maxSeconds` the recording stops by itself, as if the button were pressed.
- Starting twice is ignored (Windows repeats the hotkey's press while it's held).
- `mic_problem()`, asked when the bubble opens:
  - no input device: "No microphone was found."
  - Windows: microphone access off in Privacy settings (read from the registry's `CapabilityAccessManager\ConsentStore\microphone`, the global switch and the one for desktop apps): "Windows doesn't let desktop apps use the microphone. Turn it on in Settings → Privacy & security → Microphone."
  - opening the stream fails: its error, in one sentence.

### Transcribing (VOC-1.3 to 1.5)
- **`route(settings)`**: `sttProvider` null, removed, or a cloud provider while local-only is on → this computer (VOC-1.5). A local or trusted-LAN provider (e.g. a local Whisper server) stays allowed in local-only.
- **This computer:** if the model isn't downloaded, `dictate_stop` returns `needsModel { model, bytes }` and keeps the audio. The bubble offers the download ("Speech-to-text runs on this computer. It needs the base model, 148 MB, from huggingface.co.") with Download and Cancel. After the download it transcribes the held audio. Otherwise: load the model, `whisper_full` on a blocking thread (greedy, language detected, no timestamps, `min(4, cores)` threads), join the segments, drop the model.
- **A provider:** with "Confirm before sending to cloud" on and a cloud receiver, `dictate_stop` returns `needsConfirm { receiver, seconds }` and keeps the audio. The bubble asks "Send 7 s of audio to Groq to turn it into text?" (PRI-3.7). Then `net::upload`: `POST {baseUrl}/audio/transcriptions`, multipart `file` (`speech.wav`, built in memory), `model`, `response_format=json`, the provider's key from the keychain → `text`.
- `dictate_finish(go)` continues (`true`) or discards (`false`) held audio. Closing the bubble discards it too.
- The audio is dropped as soon as transcription returns or fails (VOC-1.7). The log gets the length and the time, never the text.
- Empty text: "Didn't catch that. Try again."
- `afterDictation(input, text, send)`: the text is added to the input (after a space if the input isn't empty) and focused; it's sent only when "Send after dictation" is on (VOC-1.3).

### The model download (VOC-1.4)
- Only from the bubble's offer or the Voice tab's **Download**, both of which name the size; never automatically.
- `net::download` streams to `ggml-<model>.bin.part`, hashing as it goes, and sends progress `{ done, total }` to the page. The file is renamed only if the size and the SHA-256 match. Otherwise the `.part` file is deleted and the bubble says "The download was damaged, so it was deleted. Try again." Cancel deletes it too.
- It's allowed while local-only is on: it's a download the user started (PRI-4.1) and sends nothing of theirs.

### Speaking (VOC-2.1 to 2.6)
- **The speech thread** is started on first use. If `tts::Tts::default()` fails, `speech` in `voice_status` holds the reason and every Speak control is hidden (VOC-2.6):
  - Linux: "Speech needs speech-dispatcher. Install it (on Ubuntu: `sudo apt install speech-dispatcher`), then restart Cursora."
  - Windows: "Windows has no voices installed. Add one in Settings → Time & language → Speech."
- **Sentences (VOC-2.2, `speech.ts`):**
  - `speakable(markdown)` drops fenced and indented code for the words "code block", keeps a link's text, drops images, and strips heading marks, emphasis, inline code ticks, list markers, quote marks and table pipes.
  - `Sentences` takes the answer text as it grows and returns each newly finished sentence. A sentence ends at `.`, `!`, `?` or `।` followed by a space or the end, or at a blank line. "3.14" doesn't end one. The last piece is spoken when the answer ends.
- **Speak** on an answer speaks it whole, sentence by sentence, and turns into **Stop** while speaking. With **Auto-speak**, each new answer's sentences are spoken as they stream.
- **Stop**, a new Speak, or hiding the bubble stops the voice at once and drops the queue (VOC-2.3). A cancelled answer stops speaking too.
- **`speak(text, persona)`** picks the voice, sets the rate and volume (the setting's multiple of the engine's normal rate; volume as is), and queues the sentence. The page hears `speech-ended` when the queue empties.

### Choosing the voice (VOC-2.4, 2.5)
`pick_voice(persona, voices, settings) → (voice, note)`, in this order:
1. Personality off: the default voice (`defaultVoice`, else the system's).
2. The persona's own voice, if it's installed (VOC-2.5).
3. Male or female: the first installed voice with that gender and the reply language; if the reply language is "the language of the question", the gender alone.
4. Neutral or custom with no voice of its own: the default voice, with no note.
5. Nothing matches: the default voice, and the note "No female Hindi voice is installed, so the default voice is speaking." It's shown once per persona while Cursora runs.

The reply language is the user's words ("Brazilian Portuguese", "हिन्दी"), so it's matched through a small table of language names to codes, or used as a code if it is one ("pt-BR"). An unknown name means no language preference.

### Push-to-talk (VOC-1.1)
- Registered with the other hotkeys (SEL-1.3 rules), but not on Wayland, where apps can't have global hotkeys (LNX-2).
- Pressed: if the bubble is closed, the Ask flow runs (the selection is captured and the bubble opens), then recording starts. Released: as the mic button's stop; the text reaches the bubble as a `dictation` event.
- X11 repeats a held key as release-and-press pairs. A release is acted on only if no press follows within 40 ms.

### Settings → Voice
- **Speech to text:**
  - "Transcribe on": This computer, or each OpenAI-compatible provider; a cloud one is disabled in local-only, with the reason. A provider shows the model field and "Recordings are sent to <host>."
  - This computer: the three models with their sizes, each "Downloaded" or **Download** (with progress), and which one is used.
  - Stop recording after N seconds; Send after dictation; where to set push-to-talk (General → Hotkeys).
  - The microphone's problem, if there is one.
- **Speech output:** Auto-speak answers; Default voice; Rate; Volume; **Test** ("Hello, I'm Cursora."). If speech is unavailable, the explanation instead (VOC-2.6).
- **Personality editor:** Voice (the default, or an installed voice), Rate, Volume (VOC-2.5).

## Commands and events
| Name | Kind | Does |
|---|---|---|
| `voice_status()` | command | The microphone, speech, the downloaded models and the voices. |
| `dictate_start(on)` | command | Starts recording; `on` is a channel of `{ level, ms }`, then `{ stopped }` when it stops by itself. |
| `dictate_stop()` | command | `{ text }`, `{ needsModel }` or `{ needsConfirm }`. |
| `dictate_finish(go)` | command | Transcribes (`true`) or discards (`false`) held audio; returns like `dictate_stop`. |
| `whisper_download(model, on)` | command | Downloads a model; `on` gets `{ done, total }`. `whisper_cancel()` stops it. |
| `speak(text, persona)` | command | Queues one sentence; returns the voice note, if this is its first time. |
| `speak_stop()` | command | Silences and empties the queue. |
| `speech-ended` | event | The queue is empty. |
| `dictation` | event | Push-to-talk's text, for the bubble's input. |

## Changes to earlier specs' code
- **Spec 01:** `hotkey::apply` registers push-to-talk.
- **Spec 03:** the bubble's mic, meter, Speak/Stop and notes; closing it stops voice.
- **Spec 04:** `net::upload` and `net::download`; an OpenAI-compatible provider can transcribe.
- **Spec 05:** `Persona.voice`.
- **Spec 07:** speech-to-text goes through `route()` and the guard (PRI-1.1), and asks first when confirming is on (PRI-3.7).

## Dependencies
- **`whisper-rs` 0.16**: builds whisper.cpp from source with CMake (CPU only, about 3 MB). `.cargo/config.toml` sets `WHISPER_DONT_GENERATE_BINDINGS=1`, so it uses its bundled bindings and needs no libclang. CMake must be on the PATH: on this PC, the one in Visual Studio Build Tools; the CI runners have it.
- **`cpal` 0.18**: WASAPI; ALSA on Linux (`libasound2-dev` to build), which reaches PipeWire or PulseAudio.
- **`tts` 0.26**: WinRT on Windows; speech-dispatcher 0.11 on Linux (`libspeechd-dev` to build). It links `libspeechd`, so the .deb depends on `libspeechd2` (installed on Ubuntu desktops) and the AppImage bundles it; the speech-dispatcher service itself is optional (VOC-2.6).
- **`sha2` 0.10**: the download's hash.
- **`reqwest`**: the `multipart` feature.

## Decisions
1. **whisper.cpp in process, through `whisper-rs`.** *Rejected:* `whisper-server` as a sidecar, like llama-server (whisper.cpp publishes no Linux build, and it's another process and port); `whisper-cli` per dictation (it reads a WAV file, so the audio would touch the disk, against VOC-1.7).
2. **The model is loaded for each dictation and dropped after.** Loading takes well under a second; keeping 150 to 500 MB loaded would break the idle-memory budget (constitution §8).
3. **Sentences are cut in the page (`speech.ts`).** The page already has the streamed text, with redacted values restored and the markdown rules; Rust only speaks what it's given.
4. **The `tts` crate on one thread of its own.** *Rejected:* each OS's speech API by hand (the crate already covers WinRT and speech-dispatcher, with rate, volume, voices and stop).
5. **Cloud speech-to-text reuses a provider from Models** (its base URL and keychain key) plus a model name. No second place for keys.
6. **Push-to-talk starts like Ask**, so a held key on a selection opens the bubble with the selection and records the question about it.

## Requirement tweaks (approved, applied)
1. **VOC-1.1:** on Wayland there is no push-to-talk hotkey (no global hotkeys there, LNX-2); the mic button works.
2. **VOC-1.4:** the model download is allowed while local-only is on (a download the user started, PRI-4.1).
3. **VOC-1.5:** with "Confirm before sending to cloud" on, a recording isn't sent to a cloud provider until the user confirms (PRI-3.7, spelled out for audio).
4. **VOC-2.4:** on Linux, speech-dispatcher doesn't report a voice's gender, so a male or female persona gets its own chosen voice, or the default voice with the note.

## Known limits
- `// shortcut:` resampling is linear, with no filter, which is fine for speech. Upgrade if accuracy suffers on 44.1 kHz devices.
- `// shortcut:` about 20 language names are known for matching voices. Upgrade when a user's reply language isn't matched.
- `// shortcut:` an abbreviation ("e.g. this") ends a sentence early, so there is a short pause. Upgrade if it's noticeable.
- There is no button to delete a downloaded model; the Voice tab says where they are.
- A slow PC can take several seconds for the small model; the bubble shows "Transcribing…".
- Wayland: no push-to-talk.

## Test strategy

| Criterion | Test |
|---|---|
| VOC-1.3 | `afterDictation()`: the text is added after a space, focus goes to the input, and it's sent only with "Send after dictation" on. |
| VOC-1.4 | The model table: three multilingual models, one pinned revision, the sizes and hashes. A download from a fake server: a good file is kept; a wrong hash or a short file leaves no file and no `.part`. Without the model, `dictate_stop` answers `needsModel` and makes no request. With `CURSORA_WHISPER_MODEL` set (like `CURSORA_XVFB`), a short speech WAV is transcribed. |
| VOC-1.5 | `route()`: this computer by default; the provider when set; a cloud provider in local-only → this computer; a local provider stays. A fake server gets the multipart request (the model, a 16 kHz mono WAV, the key), and its `text` comes back. The guard still refuses a cloud host in local-only. |
| VOC-1.7 | The recording is moved into transcription and dropped after; `wav()` builds the bytes in memory; the recording and transcription code uses no file API (a source check). |
| VOC-2.2 | `speakable()`: markdown, code blocks, links, images. `Sentences`: each sentence end, a sentence held until it ends, the rest at the end, "3.14" kept whole. |
| VOC-2.4 | `pick_voice()`: male and female with and without a language; the persona's own voice; neutral and custom; personality off; no match → the default and the note, once. |

Manual: VOC-1.1, 1.2, 1.6, 2.1, 2.3, 2.5 and 2.6.
- **Without a human voice:** debug builds read `CURSORA_MIC_FILE=<wav>` as the microphone (like `CURSORA_ECHO`), so the end-to-end runs cover the meter, the stop, the download offer, transcription and insertion on Windows and in WSL. Speech is checked through the Speak/Stop state and the log.
- **With the user:** a real microphone, hearing the voices, rate and volume, and the Windows privacy switch.
- **Linux:** in WSL with speech-dispatcher and espeak-ng; WSLg gives a PulseAudio microphone and output. Real Ubuntu is issue 2.
- The tests use the tiny model (78 MB) to spare the C: drive.
