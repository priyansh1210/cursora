# 16: Indian languages (model trial)

LNG-4.1's line under My language comes from this trial. [trial.mjs](trial.mjs) sends six Cursora-shaped requests (the base instructions, the reply-language line, the skill's instructions and the untrusted-content rule, then the delimited selection and the command) to an OpenAI-compatible server:

1. English into Hindi, with `/translate` and My language Hindi
2. English into Tamil, the same with Tamil
3. Hindi into English (the text is already in My language)
4. English into Hinglish
5. `/explain` with "Always reply in Hindi."
6. `/explain` with "Always reply in Tamil."

## How to run it
1. Start the built-in engine with the model: `src-tauri\llama\llama-server.exe -m <file>.gguf --host 127.0.0.1 --port 18573 -c 4096` (`npm run fetch-llama` gets the engine).
2. In another window: `node specs/16-indian-languages/trial.mjs http://127.0.0.1:18573/v1`. For Gemini, the user (not Claude) first runs `$env:TRIAL_API_KEY = Read-Host "Gemini key"` in that window and pastes the key at the prompt, so it stays out of the command history; then `node specs/16-indian-languages/trial.mjs https://generativelanguage.googleapis.com/v1beta/openai gemini-flash-lite-latest`.
3. Write down below, for each case, whether the answer is in the right language and script, and whether it is correct.

## Results

| Model | Date, PC | Hindi | Tamil | Hindi into English | Hinglish | Explain in Hindi | Explain in Tamil |
|---|---|---|---|---|---|---|---|
| Small: Qwen2.5 1.5B Instruct Q4_K_M | 2026-10-26, laptop | No: English | No: English | Wrong: made up "6 PM" | No: English | No: English | No: English |
| Medium: Qwen3 4B Instruct 2507 Q4_K_M | 2026-10-26, stronger PC | Devanagari, wrong: Saturday for Friday, "one-third o'clock" for 3 pm, the contract garbled | Tamil script, wrong: Friday lost, "subscription" for contract | Right | Hinglish, partly wrong: "le jao" (take away) for bring | Right, with a correct worked example (37.6 s) | Tamil script, then one sentence repeated until the limit (176.8 s) |
| Large: Qwen2.5 7B Instruct Q4_K_M | 2026-10-26, stronger PC | Devanagari, wrong: "editing" for meeting, Saturday 3 AM | No: meaningless words, English fragments | No: answered in Chinese | No: English | Devanagari, a made-up term for compound interest, English words mixed in; the example right | Tamil script, meaningless terms, no real explanation |
| Gemini (`gemini-flash-lite-latest`), first run | 2026-10-26, laptop | No: **Spanish**, from the old Translate wording (R70) | Right | Right (it read "कल" as yesterday; it can also mean tomorrow) | Right, natural Hinglish | Right, with a correct worked example (2.3 s) | Right (2.1 s) |
| Gemini, after the R70 fix | 2026-10-26, laptop | **Right**: Friday, 3 pm, the contract and the laptop all kept (2.0 s) | Right | Right ("yesterday" again) | Right | Right, with the same correct example | Right |

**Medium and Large** (2026-10-26, stronger PC: the files from the pinned URLs, SHA-256 matching `download.rs`; llama.cpp b11570 CPU build, `-c 4096`; one run each). Medium is the best of the three downloads, and good at Hindi into English and at explaining in Hindi, but its translations into Hindi and Tamil changed the facts (the day and the time), which is worse than no translation, and its Tamil explanation looped. Large (an older Qwen2.5) was worse than Medium in every case. So the LNG-4.1 line no longer points to Medium: it says the free downloads aren't reliable in Indian languages and names a cloud model. Gemini's row is still empty (the user runs it with their key); if it disappoints too, change the line again.

**Gemini** (2026-10-26, laptop, the user's key; the trial run by the user so the key never reached Claude): right and fast in five cases out of six, and in all six after the fix. The sixth found a bug in Translate's wording, not in Gemini: "…or into Spanish when the text is English" was meant for people whose My language is English, but Gemini read it as "English text goes into Spanish", for a Hindi user too (R70). The wording now ties that rule to My language. Since then, trial.mjs reads the wording from the skill files, `prompt.rs` and `skills.rs`, so it sends what Cursora sends (before, it had its own copy, without the replace rule).

**Small:** it isn't the prompt's wording. Asked again with "Target language: Hindi" stated plainly, it still answered in English, and its Tamil was meaningless words (issue 52).
