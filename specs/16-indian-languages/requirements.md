# 16: Indian languages (LNG)

## Scope
- **In:** a **My language** setting, with Indian languages suggested first. It is used for one-click translating, and for every answer when the user asks for that. Hinglish. Indian scripts, including Urdu right to left, shown and typed correctly everywhere in Cursora. An honest line on which models handle these languages well.
- **Out:**
  - Cursora's own menus and messages in other languages: the app stays in English
  - Picking My language from the Windows or Linux language (most Indian users run them in English)
  - A model downloaded for a language: spec 15's three choices stay
  - Voice (spec 09)

Idea #7 from the improvement plan ([product direction](../../memory/product-direction.md)), started at the user's request of 2026-10-26.

## Requirements

### LNG-1 My language
User story: As someone who reads Hindi (or Tamil, Bengali, …) more easily than English, I want Cursora to know my language, so that translating takes one click and answers can come in it.
- LNG-1.1 (manual) Settings → General and Setup's persona step (APP-6.1, step 2) SHALL have **My language**, English by default. It SHALL suggest English, Hindi, Hinglish (Hindi in English letters), Bengali, Marathi, Telugu, Tamil, Gujarati, Urdu, Kannada, Odia, Malayalam, Punjabi and Assamese, and any other language can be typed. It is checked like a persona's reply language (PER-1.1): letters, spaces and hyphens, in any script.
- LNG-1.2 (auto) WHEN `/translate` runs with no language typed THE request SHALL ask for a translation into My language, or into English when the text is already in My language. The **Translate** quick action (QAC-2.2) does the same. While My language is English, nothing changes: English, or Spanish when the text is already English.
- LNG-1.3 (auto) WHERE **Always answer in my language** is on (off by default) THE requests SHALL ask for answers in My language, with personality on or off. A persona's own reply language (PER-1.1) still wins while that persona is in use. While it is off, answers come in the language of the user's question, as now.
- LNG-1.4 (auto) Hinglish SHALL be described to the model as Hindi written in English letters, the way people type it in chats, so a translation into Hinglish doesn't come back in Devanagari.
- LNG-1.5 (auto) Skills SHALL get a `{{my_language}}` variable (SKL-1.2 amended), and `{{language}}` SHALL follow LNG-1.3.

### LNG-2 Editing keeps the text's language
User story: As a user with a reply language set, I want Fix and Rewrite to correct my English text in English, not hand me a Hindi version.
- LNG-2.1 (auto) `/fix`, `/rewrite` and `/simplify` SHALL tell the model to keep the selected text's own language, whatever the reply language (LNG-1.3, PER-1.1), so Replace (spec 13) never swaps the text for a translation.

### LNG-3 Indian scripts on screen
User story: As a user, I want Hindi, Tamil or Urdu to look right in Cursora and to go into my apps intact.
- LNG-3.1 (manual) Text in Devanagari, Bengali, Gurmukhi, Gujarati, Odia, Tamil, Telugu, Kannada, Malayalam and Urdu script SHALL show correctly (joined letters and vowel signs, no empty boxes) in the selection preview, answers, the input, History and Settings.
- LNG-3.2 (manual) Urdu SHALL show right to left in answers and in the input, also when it is mixed with English.
- LNG-3.3 (manual) Typing with an Indian keyboard in the bubble (such as Windows' Hindi Phonetic) SHALL work, and the Enter that confirms a word SHALL NOT send the question (BUB-4.1).
- LNG-3.4 (manual) Replace and Insert (spec 13) SHALL put an answer in an Indian script into an app intact.

### LNG-4 Which models do this well
User story: As a user, I want to know which model to use for my language before I'm disappointed by a weak one.
- LNG-4.1 (manual) WHILE My language is one of the suggested Indian languages (LNG-1.1, Hinglish too) THE My language setting SHALL show one plain line saying which models handle Indian languages well. The line is written after trying the three downloadable models (spec 15) and Gemini with Hindi and with Tamil; what was found is kept in [evaluation.md](evaluation.md). (Amended 2026-10-26 at the user's request: was "isn't English", which showed the line to a French user.)
- LNG-4.2 (manual) WHILE My language is one of them THE download choices (MDL-1.1) SHALL show the same line. (Added 2026-10-26 at the user's request, after the trial found none of the downloads reliable in Indian languages; issue 52.)

## Choices made with the user (2026-10-26)
1. **Answers follow the language the user writes in.** **Always answer in my language** is off by default, and turns every answer into My language for those who want it.
2. **My language is in Setup and in Settings → General**, so new users meet it in their first minute.
3. **The suggested list** is English, Hindi, Hinglish and the eleven most-spoken other scheduled languages. Any other language can be typed.
4. **The app stays in English.**

## Open questions
None.
