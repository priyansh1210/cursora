# 19: Voice input (VIN)

Status: **Approved**¹ (2026-10-28).

## Scope
- **In:** saying a question instead of typing it. A mic button in the bubble records, and a speech model on this computer turns the recording into text in the question box. Two speech models to download, in any language they know, Hindi and the other Indian languages included.
- **Out:**
  - Spoken answers (text-to-speech): the user wants input only (2026-10-28). Spec 09 stays Optional and unused.
  - A push-to-talk hotkey: the mic button is reachable with the keyboard (BUB-8.1). The unused `hotkeys.pushToTalk` setting stays as it is.
  - Cloud speech-to-text services
  - Wake words, continuous dictation and voice commands
  - A level meter: the running seconds show that it's recording

Update #1 from the list of 2026-10-28 ([improvements batch 2](../../memory/improvements-batch-2.md)).

## Requirements

### VIN-1 Dictate a question
User story: As a user, I want to say my question instead of typing it, in my own language.
- VIN-1.1 (manual) THE bubble's question box SHALL have a mic button next to Send. Clicking it SHALL start recording. While recording, the button SHALL show Stop and the seconds recorded, and clicking it again SHALL stop. Recording SHALL stop by itself after 60 seconds.
- VIN-1.2 (auto) WHEN recording stops THE SYSTEM SHALL turn the speech into text on this computer and add it to the question box after anything already typed, without sending it. While that runs, the button SHALL show that it is writing. Whisper's marks for sounds that aren't speech, such as `[BLANK_AUDIO]` or `(music)`, SHALL be left out, and IF no words were heard THE bubble SHALL say so.
- VIN-1.3 (auto) THE recording SHALL stay in memory only. It is never written to disk or sent anywhere, and it is dropped once turned into text. Dictation therefore works in local-only mode (PRI-1).
- VIN-1.4 (manual) IF the microphone can't be used (none is connected, or Windows' microphone privacy setting blocks desktop apps) THE bubble SHALL say why in plain words.
- VIN-1.5 (auto) WHEN the bubble closes while recording THE recording SHALL be dropped.
- VIN-1.6 (manual) WHILE no speech model is set up, the mic button SHALL say so in its tooltip and open Settings → General, where Voice input is.

### VIN-2 The speech model
User story: As a user, I want to set up dictation once, with one click, and choose speed or accuracy.
- VIN-2.1 (manual) Settings → General SHALL have a **Voice input** group with two speech models and their sizes, **Fast** and **Accurate** (better for Hindi and the other Indian languages), the folder with **Change…**, and **Download** with progress and **Cancel**, as for the chat models (MDL-1.1, MDL-1.4). Accurate SHALL be marked Recommended while My language is an Indian language (LNG-1.1), Fast otherwise. Once a model is downloaded, the group SHALL say which one is in use.
- VIN-2.2 (auto) Each speech model's address (whisper.cpp's models on huggingface.co, at a fixed revision), size and SHA-256 SHALL be fixed in the code. A downloaded file that doesn't match SHALL be deleted and not used (MDL-1.3).
- VIN-2.3 (retired) Finding any spoken language by itself. Replaced by VIN-2.4 on 2026-10-29: Hindi with an Urdu word came out in Urdu script.
- VIN-2.4 (auto) THE speech model SHALL find the spoken language by itself, but never Urdu, Persian or another language of India besides Hindi, so Hindi with Urdu or Persian words comes out in Hindi letters (Devanagari), while English and foreign languages such as French come out as said.

## Defaults taken under the go-ahead of 2026-10-28
1. **Voice input only, no spoken answers** (the user's choice).
2. **Whisper, on this computer** (whisper.cpp through `whisper-rs`), so nothing is sent anywhere. No cloud speech service.
3. **Two models:** Fast is `ggml-base-q5_1` (60 MB) and Accurate is `ggml-small-q5_1` (190 MB), both multilingual. On the laptop (i5-11300H), a 5-second question took 1.6 s with Fast and 5.3 s with Accurate, both word for word.
4. **The text is added, never sent by itself,** so a misheard word can be fixed first.
5. **No push-to-talk hotkey** for now.
6. **The speech model is saved with the chat models**, in the same folder, with the same download (spec 15).
7. **Constitution §3:** the user-started download of a pinned model file (amendment of 2026-10-26) covers speech models too. Amended 2026-10-28.

## Changed by the user on 2026-10-29
- **Hindi, English and foreign languages, not the other languages of India** (VIN-2.4, replacing VIN-2.3). In the user's check, Hindi came out in Hindi letters, but a sentence with an Urdu word came out in Urdu script, since whisper picks among all its languages, and everyday Hindi has many Urdu and Persian words. The user: "we can just do hindi and english languages only and the foreign languages no other indic languages", then "I still want foreign languages". Whisper's language detection now runs first, and the most likely language is set, leaving out Urdu, Persian, Bengali, Tamil, Telugu, Marathi, Gujarati, Kannada, Malayalam, Punjabi, Nepali, Sinhala, Sanskrit, Assamese and Sindhi. Persian is left out although it isn't a language of India, since it caused the same problem; for review.

## Open questions
None.

¹ Approved under the user's go-ahead of 2026-10-28 to design, build and merge updates 1 and 3 to 9, with the user reviewing everything afterwards. The choices made for the user are listed above.
