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
| Medium: Qwen3 4B Instruct 2507 Q4_K_M | | | | | | | |
| Large: Qwen2.5 7B Instruct Q4_K_M | | | | | | | |
| Gemini (`gemini-flash-lite-latest`) | | | | | | | |

**Small:** it isn't the prompt's wording. Asked again with "Target language: Hindi" stated plainly, it still answered in English, and its Tamil was meaningless words (issue 52).
