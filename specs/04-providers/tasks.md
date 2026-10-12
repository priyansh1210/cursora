# 04: AI providers (tasks)

Status: **In progress**.

- [x] 1. Checks before building: reqwest 0.13 with `native-tls` builds on Windows and in WSL; keyring 4's API and its test store; the pinned llama.cpp build and its SHA-256 values.
- [x] 2. `llm/mod.rs` types, the `models` settings section, `locality()`, `availability()`; removing a provider deletes its key; the APP-4.4 test walks every provider kind (tests first). Covers LLM-8.1, 8.2, 9.1 (pure part), APP-4.4.
- [x] 3. `secrets.rs`: set, has, hint, delete; a clear error when there is no keychain (tests first). Covers LLM-6.1, 6.2, 6.3.
- [x] 4. `llm/net.rs`: SSE framing, `classify()`, the local-only guard, no redirects, no proxy for local hosts, the stream loop with cancel and timeouts; a fake HTTP server for the tests (tests first). Covers LLM-1.2 (framing), 1.3, 1.4, 1.5, PRI-1.1.
- [x] 5. Adapters `openai.rs`, `anthropic.rs`, `gemini.rs`: request bodies, stream parsing against fixtures, model lists; `vision.rs` (tests first). Covers LLM-1.1, 1.2, 2.2, 5.1, 5.2.
- [x] 6. `ProviderModel`, `llm::active`, `capabilities()`; chat.rs uses them: `Message.images`, `Conversation.unavailable`, the too-long check, Echo only with `CURSORA_ECHO=1`. Covers LLM-5.3.
- [x] 7. Health check at the keypress, no capture without a model, the bubble swaps its input when the check fails. Covers LLM-9.1.
- [x] 8. Built-in engine: `scripts/fetch-llama.mjs`, `engine.rs` (start, health wait, idle stop, stop on quit, Job Object / PDEATHSIG, crash reason), `tauri.llama.conf.json` and the release job. Covers LLM-3.2–3.6.
- [x] 9. Commands: providers, keys, model list, Test connection, LM Studio probe, RAM suggestion, engine status, enabled models, `set_model`. Covers LLM-2.1, 2.4, 7.1.
- [x] 10. UI: `providers.ts` presets (vitest), the Models tab, the Setup "AI source" step (Next only after a passing test), the header switcher, the reason in "No AI model is available". Covers LLM-4.1, 4.2, 5.4, 5.5, 8.3, APP-6.1 step 1.
- [x] 11. End-to-end run on Windows with the built-in engine and a small `.gguf`, then `/ponytail-review`.
- [x] 12. How long the model stays loaded (2026-10-13): `lifetime` always / idle / answering, "Always on" by default; never stopped mid-answer; "Stop the model" in the tray and the Models tab; the choice in Setup and the Models tab (test first). Covers LLM-3.5, APP-1.2, APP-6.1 step 1.

## Manual checklist
- [ ] LLM-3.1 The installers contain llama-server and its libraries. Windows ☐ Linux ☐
- [ ] LLM-3.2 Choosing a `.gguf` starts the engine on a free port and it answers. Windows ✓ (auto-verified: setup with a 0.5B model, first words in 0.7 s) Linux ✓ chosen in setup's file picker, engine on 127.0.0.1, answers (auto-verified 2026-10-13 in WSL: Xvfb + Openbox)
- [ ] LLM-3.5 Each lifetime: "Always on" loads with Cursora and unloads only by hand (tray, Models tab) or on Quit; "Off when idle" unloads after the idle time; "Only while answering" unloads once the bubble and the expanded window are closed. Never mid-answer. Windows ✓ starts on the first request, starts again after a crash, stops on Quit (auto-verified) ✓ idle stop (after 108 s with a 1-minute idle time) ✓ always on, stop by hand in the tray and the Models tab, only while answering (also with the expanded window), switching to Always on loads it (auto-verified 2026-10-13) Linux ✓ all three lifetimes, Stop in the Models tab, switching to Always on loads it (auto-verified 2026-10-13 in WSL: Xvfb + Openbox) ☐ tray Stop (no tray host in WSL)
- [ ] LLM-3.6 A broken model file or a killed engine shows the reason in the bubble and the Models tab. Windows ✓ killed engine, in the bubble and the Models tab (auto-verified) ✓ broken model file (auto-verified 2026-10-13) Linux ✓ broken model file, in the bubble and the Models tab (auto-verified 2026-10-13 in WSL: Xvfb + Openbox) ☐ killed engine
- [ ] LLM-4.2 Add, edit and remove providers. Windows ✓ (auto-verified 2026-10-13): added with a key, name, base URL and models edited, removal deletes the key Linux ✓ (auto-verified 2026-10-13 in WSL: Xvfb + Openbox)
- [ ] LLM-5.4 Pick the default model. Windows ✓ (auto-verified 2026-10-13) Linux ✓ (auto-verified 2026-10-13 in WSL: Xvfb + Openbox)
- [ ] LLM-5.5 The header switcher lists the chosen models; cloud models are disabled in local-only mode. Windows ✓ (auto-verified 2026-10-13) Linux ✓ cloud model disabled in local-only (auto-verified 2026-10-13 in WSL: Xvfb + Openbox)
- [ ] LLM-6.3 With no Secret Service, saving a key explains how to enable one and stores nothing; LM Studio and the built-in engine still answer. Linux ✓ the message explains, nothing stored, the built-in engine still answers (auto-verified 2026-10-13 in WSL: Xvfb + Openbox)
- [ ] LLM-8.3 Adding a provider on a LAN host asks whether to trust it; this can be changed later. Windows ✓ (auto-verified 2026-10-13): asked, trusted, then untrusted in the Models tab Linux ✓ (auto-verified 2026-10-13 in WSL: Xvfb + Openbox)
- [ ] LLM-8.4 With Wi-Fi off, the built-in engine answers and the rest of Cursora works. Windows ✓ (auto-verified 2026-10-13, offline the whole run: start with the model loading, hotkey, answer and follow-up, Models tab, History, setup's test) Linux ✓ (auto-verified 2026-10-13 in WSL, inside a network namespace with no network)
- [ ] APP-6.1 step 1 Setup can't go past "AI source" until the model passes Test connection. Windows ✓ (auto-verified: Next stays disabled until "It works"); for the built-in engine it asks how long the model stays loaded, "Always on" preselected (auto-verified 2026-10-13) Linux ✓ (auto-verified 2026-10-13 in WSL: Xvfb + Openbox)
- Spec 03 items that need a real model: BUB-4.3 (Stop) ✓ and BUB-2.5 scrolling ✓ (auto-verified); BUB-4.5 (Thinking) ✓ and BUB-6.1 (Regenerate) ✓ (auto-verified 2026-10-13).
