# 16: Indian languages (tasks)

- [x] 1. Settings `my_language` and `answer_in_my_language`; `persona::check_language`, also run by `update_settings`. Covers LNG-1.1 (the check).
- [x] 2. `prompt::reply_language`, `language_name`, `system(persona, language, skill)`; `chat` passes the language on. Covers LNG-1.3, 1.4.
- [x] 3. `{{my_language}}`, `skills::language(s)`; the translate, fix, rewrite and simplify wording. Covers LNG-1.2, 1.5, 2.1 (issue 51).
- [x] 4. Settings → General's Language section, Setup's My language (its heading is now "Choose a personality and your language"), the persona's Reply language suggestions, the variable picker, the right-to-left CSS. Covers LNG-1.1, 3.2.
- [ ] 5. Try the models in Hindi and Tamil, then write the LNG-4.1 line. **Small (2026-10-26, laptop, the engine on its own with Cursora's prompts):** answered in English when asked to translate into Hindi or Tamil, or to explain in either, even with "Target language: Hindi" stated plainly; its Tamil was meaningless; its Hindi-to-English translation made up a time; Hinglish came back as English (issue 52). The line says so, and points to Medium or a cloud model. **Still to try, on the stronger PC:** Medium, Large and Gemini (the laptop ran out of memory for Medium), with [trial.mjs](trial.mjs); results in [evaluation.md](evaluation.md). Change the line if they disappoint.
- [ ] 6. Check on Windows (test build) and in WSL (X11); then `/ponytail-review`. Windows done 2026-10-26 (below); found R64 (a stale error after a refused name), fixed.

## Manual checklist
- [ ] LNG-1.1 My language in Settings → General and Setup, the suggestions, typing another language, a refused name. Windows ✓ (2026-10-26, test build: "Hindi" saved; "Hindi 2" and "Tamil 2" refused with the reason, and the error clears once a good name is back (R64); Setup's step 2 shows the field, past a Test connection to a stand-in server) ☐ the suggestion list, by hand X11 ☐
- [ ] LNG-3.1 Hindi, Bengali, Gurmukhi, Gujarati, Odia, Tamil, Telugu, Kannada, Malayalam and Urdu text in the preview, answer, input, History and Settings. Windows ✓ (2026-10-26: all ten in the expanded preview with joined letters and vowel signs; Hindi and Urdu in the input, the question, the answer and History) X11 ☐
- [ ] LNG-3.2 Urdu right to left, mixed with English, in an answer and the input. Windows ✓ input, question and preview (an Urdu line runs right to left beside left-to-right lines) ☐ an answer that starts in Urdu (needs a model that writes Urdu) X11 ☐
- [ ] LNG-3.3 Typing with Windows' Hindi Phonetic keyboard; the Enter that confirms a word doesn't send. Windows ☐
- [ ] LNG-3.4 Replace and Insert with a Hindi answer. Windows ✓ Replace (2026-10-26, the test window: the Echo answer with a Hindi question went in intact) ☐ Insert X11 ☐
- [ ] LNG-4.1 The model line, from the trial in task 5. Windows ☐ X11 ☐
