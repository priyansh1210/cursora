# 19: Voice input (tasks)

Status: **In progress**.

- [x] 1. Build settings (`.cargo/config.toml`), the dependencies, CI and `wsl-test.sh`; the constitution amendment.
- [x] 2. `voice.rs`: `to_16k_mono`, `audio_ctx`, `clean`, `transcribe`, the recording thread and the commands, with tests. Covers VIN-1.2, 1.3, 1.5.
- [x] 3. `download.rs`: the speech models; the setting `voice_model`. Covers VIN-2.2.
- [x] 4. The mic button in the bubble; Voice input in Settings → General; the manual and README. Covers VIN-1.1, 1.4, 1.6, 2.1.
- [ ] 5. All checks; by hand on Windows; then `/ponytail-review`. 2026-10-28: trace, JS (20), build, cargo test (179, Windows), test:linux (185, WSL) pass; Windows by hand below, with the test copy.

## Manual checklist
- [ ] VIN-1.1 The mic starts and stops recording, shows the seconds, and stops by itself at 60 s. Windows ✓ (2026-10-28, test copy: the laptop's speakers said the question and the real microphone heard it; the button showed ■ 0:06; 3 s after Stop the box held "Explain this paragraph in simple words and give me two examples.", not sent. A 61 s recording stopped by itself. Closing the bubble while recording dropped it, VIN-1.5) Linux X11 ☐ (WSL has no microphone, issue 64)
- [ ] VIN-1.4 With the microphone blocked in Windows' privacy settings (or none connected), the bubble says why. Windows ☐ (a system setting, for the user: issue 64) Linux X11 ☐
- [ ] VIN-1.6 Without a speech model, the mic's tooltip says so and it opens Settings → General. Windows ✓ (2026-10-28: "Set up voice input in Settings", named "Dictate: set up voice input in Settings first"; it opened Settings) Linux X11 ☐
- [ ] VIN-2.1 Voice input in General: both models with sizes, Recommended following My language, Change…, Download, progress, Cancel, and the model in use. Windows ✓ Fast (60 MB) with Recommended for English, Accurate (190 MB), the folder and Change…; Download took 9 s and Fast showed In use ☐ Recommended for an Indian language ☐ Cancel Linux X11 ☐
- [ ] VIN-2.3 (retired 2026-10-29, now VIN-2.4) A question said in Hindi comes out in Hindi (Accurate). Windows ✓ by the user (2026-10-29: Hindi came out in Hindi letters, but a sentence with an Urdu word came out in Urdu script, so the user narrowed voice input to Hindi and English) Linux X11 ☐
- [ ] VIN-2.4 Hindi with Urdu words comes out in Hindi letters; English and a foreign language come out as said (Accurate). Windows ✓ by the user (2026-10-29: "मुजे आपकी मदद की जरूरत है", "Do you think I am a kid?", "¿Cómo estás?") Linux X11 ☐ (WSL has no microphone)
- [ ] VIN-1.4, with the privacy setting off (2026-10-29, the user): the bubble named the right setting, but behind Windows' raw error ("Failed to initialize audio client: OS Error -2004287487 (FormatMessageW() returned error 317)"), which isn't plain words. Now the raw error goes to the log, and the bubble says only what to check. Windows ✓ again after the fix, by the user: "Cursora couldn't use the microphone. Check that one is connected, and that Settings → Privacy & security → Microphone → Let desktop apps access your microphone is on."
