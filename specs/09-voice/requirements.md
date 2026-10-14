# 09: Voice in/out (VOC)

## Scope
- **In:**
  - Dictating questions (speech-to-text)
  - Speaking answers aloud (text-to-speech)
  - Choosing a voice from the persona
  - Downloading speech models
- **Out:**
  - Wake word
  - Continuous hands-free conversation
  - Voice cloning
  - Neural TTS voices such as Piper (backlog candidate)

## Requirements

### VOC-1 Speech input
User story: As a user, I want to ask my question by speaking instead of typing.
- VOC-1.1 (manual) THE bubble SHALL have a mic button that starts and stops recording. An optional push-to-talk hotkey SHALL record while held. It is chosen during setup (APP-6.1), may be left unset, and can be changed in Settings. (Amended 2026-10-14: on Wayland there is no push-to-talk hotkey, since apps can't have global hotkeys there (LNX-2); the mic button works.)
- VOC-1.2 (manual) WHILE recording THE bubble SHALL show a level meter and the elapsed time. Recording SHALL stop automatically at 60 s (configurable).
- VOC-1.3 (auto) WHEN recording stops THE SYSTEM SHALL transcribe the audio and insert the text into the input. It SHALL NOT send the text unless "Send after dictation" is on (default off).
- VOC-1.4 (auto) Local speech-to-text SHALL use whisper.cpp. Its model (tiny, base or small; multilingual) SHALL be downloaded on first use only after the user agrees, with the size and progress shown. The download SHALL be checked against a pinned SHA-256 before use. (Amended 2026-10-14: the download is allowed while local-only mode is on: the user started it, and it sends nothing of theirs (PRI-4.1).)
- VOC-1.5 (auto) WHERE the user chose a cloud speech-to-text provider (OpenAI-compatible `/audio/transcriptions`) THE SYSTEM SHALL use it. WHILE local-only mode is on, local speech-to-text SHALL be used instead (PRI-1.1). WHERE "Confirm before sending to cloud" is on, a recording SHALL be sent to a cloud provider only after the user confirms (PRI-3.7). (Amended 2026-10-14.)
- VOC-1.6 (manual) IF there is no microphone, or access is denied, THE mic button SHALL be disabled with a tooltip that gives the reason.
- VOC-1.7 (auto) Recorded audio SHALL be kept in memory only, never written to disk, and discarded after transcription.

### VOC-2 Speech output
User story: As a user, I want my cursor to talk back in its own voice.
- VOC-2.1 (manual) Each answer SHALL have a **Speak** button. Settings SHALL have "Auto-speak answers" (default off).
- VOC-2.2 (auto) THE SYSTEM SHALL speak sentence by sentence while the answer streams.
  - Sentences end at `.`, `!`, `?`, `।` or a blank line.
  - Markdown symbols SHALL be stripped.
  - Code blocks SHALL be replaced by the words "code block".
  - Links SHALL be read as their text only.
- VOC-2.3 (manual) A **Stop** control SHALL silence speech immediately. Hiding the bubble SHALL also stop speech.
- VOC-2.4 (auto) Voice selection:
  - WHILE personality is on THE SYSTEM SHALL choose an installed voice that matches the persona's gender (male/female) and reply language.
  - For neutral or custom gender, the persona's chosen voice is used.
  - WHILE personality is off, the user's default voice is used.
  - IF nothing matches THE SYSTEM SHALL use the default voice and say so once.
  - On Linux, where speech-dispatcher doesn't report a voice's gender, a male or female persona SHALL use its own chosen voice, or the default voice with the note. (Amended 2026-10-14.)
- VOC-2.5 (manual) THE user SHALL be able to override the voice, rate and volume for each persona.
- VOC-2.6 (manual) IF no TTS engine is available (e.g. speech-dispatcher is missing on Linux) THE Speak features SHALL be hidden, with an explanation in the Voice tab (constitution §4).

## Open questions
- _(none.)_

## Resolved
- The push-to-talk hotkey is set by the user during setup and is editable in Settings (VOC-1.1, APP-6.3).
