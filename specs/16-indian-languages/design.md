# 16: Indian languages (design)

Status: **Approved** (2026-10-26, PR #46). Implements [requirements.md](requirements.md). Two settings and one place in Rust that decides the reply language. The prompt builder, `{{language}}` and the new `{{my_language}}` all ask that place. Templates and CSS do the rest: no new module.

## Overview
```
Settings: myLanguage ("English"), answerInMyLanguage (false)
   prompt::reply_language(s) = the persona's own reply language (personality on)
                               else myLanguage, while answerInMyLanguage is on
                               else None (the question's language, as now)   (LNG-1.3)
   prompt::language_name(l)  = l, except Hinglish, described for the model   (LNG-1.4)
   → prompt::system(): the persona block's language line, or one line after the base when personality is off
   → {{language}} (SKL-1.2) and {{my_language}} (new, LNG-1.5)
translate.md: no language typed → {{my_language}}, or English when the text is already in it (LNG-1.2)
fix.md, rewrite.md, simplify.md: keep the text's own language (LNG-2.1)
styles.css: each paragraph finds its own direction, so Urdu runs right to left (LNG-3.2)
```

## Components
| File | Change |
|---|---|
| `src-tauri/src/settings.rs` | `my_language: String` ("English") and `answer_in_my_language: bool` (false). |
| `src-tauri/src/persona.rs` | The reply-language check moves into `check_language(field, l)`, used for a persona and for My language. |
| `src-tauri/src/lib.rs` | `update_settings` checks My language (LNG-1.1). |
| `src-tauri/src/prompt.rs` | `reply_language(s)`, `language_name(l)`. `system(persona, language, skill)` gets the reply language as a parameter instead of reading it from the persona. |
| `src-tauri/src/chat.rs` | `request` and `assemble` pass the reply language on. |
| `src-tauri/src/skills.rs` | `Vars.my_language`, `VARIABLES` gets `my_language`, and `language(s)` reads the settings. |
| `src-tauri/skills/translate.md`, `fix.md`, `rewrite.md`, `simplify.md` | New wording (below). |
| `src/personas.ts` | `LANGUAGES`, the suggested list, next to `TONES`. |
| `src/Settings.tsx` | General gets a **Language** section: My language, **Always answer in my language**, and the LNG-4.1 line. |
| `src/Setup.tsx` | The persona step gets My language below the personas. |
| `src/Personality.tsx` | The persona's Reply language suggests the same list. |
| `src/SkillsTab.tsx` | The variable picker gets `my_language`. |
| `src/styles.css` | `unicode-bidi: plaintext` on answers, questions, the selection preview and the input. |
| `specs/06-skills/requirements.md`, `specs/01-app-shell/requirements.md` | SKL-1.2 and APP-6.1 step 2 amended. |

## Key mechanics
- **The reply language, decided once** (LNG-1.3):
  ```rust
  pub fn reply_language(s: &Settings) -> Option<String> {
      persona::in_use(s).and_then(|p| p.reply_language)
          .or_else(|| s.answer_in_my_language.then(|| s.my_language.clone()))
  }
  ```
  `system(persona, language, skill)` writes `Always reply in <language_name>.` in the persona block, where the persona's own setting used to go. While personality is off, it writes the same line right after the base instructions. With no reply language, the request stays byte-identical to today's (PER-4.3: the existing snapshots don't change).
- **`{{language}}`** is `reply_language(s)`, or "the language of the user's question" as now. **`{{my_language}}`** is My language, whatever the box says. Both go through `language_name`.
- **Hinglish** (LNG-1.4): `language_name("Hinglish")` (any letter case) is "Hinglish (Hindi written in English letters, the way people type it in chats)". Every other name passes through unchanged.
- **`translate.md`** (LNG-1.2):
  ```
  Translate the captured text. Keep its formatting, and reply with the translation only.
  Target language: {{input}}
  When no target language is given: translate into {{my_language}}, unless the text is already in {{my_language}}. Then translate into English, or into Spanish when the text is English.
  ```
  For a Hindi user, English text goes into Hindi and Hindi text into English. With My language English, English text goes into Spanish, as now.
- **`fix.md`, `rewrite.md`, `simplify.md`** (LNG-2.1) gain one sentence: "Keep the text in its own language, whatever language you were told to reply in." The skill's instructions come after the persona block, so the more specific rule is the last word. Fixes issue 51.
- **The settings fields** (LNG-1.1): an `<input list="languages">` with a `<datalist>` from `LANGUAGES`. Hinglish is listed as `value="Hinglish"`, labelled "Hindi in English letters". The field saves when it loses focus or on Enter, so a half-typed name isn't checked as the user types. An empty field saves as English. A refused name shows the check's reason, like the persona editor.
- **The model line** (LNG-4.1, 4.2): one sentence, `MODELS_FOR_LANGUAGES` in `MyLanguage.tsx`, shown under My language and under the download choices while `isIndian(myLanguage)`: one of the suggested languages other than English. Its wording comes from task 5's trial. The download choices read My language from the settings when they open.
- **Right to left** (LNG-3.2): `.answer > *, .answer li, .question, .context .preview, textarea { unicode-bidi: plaintext; text-align: start; }`. Each paragraph takes the direction of its first letter, so an Urdu line runs right to left and an English one left to right, in the same answer. Chromium (WebView2) and WebKitGTK both support it.
- **Fonts** (LNG-3.1): Windows 10 and 11 include Nirmala UI, which covers every Indian script, and the webview falls back to it by itself. On Linux, a desktop install usually has Noto fonts; the WSL check (task 6) shows whether the `.deb` and `.rpm` also need to depend on them.

  `shortcut:` no font of our own. Add a font dependency to the Linux packages if a user reports boxes.
- **Typing** (LNG-3.3): the input already ignores an Enter that confirms a word in progress (`isComposing`, BUB-4.1), so this is a manual check only.

## Decisions
1. **One function for the reply language.** The prompt and both variables read it, so they can't disagree. *Rejected:* copying My language into the persona, which would change the user's persona file and vanish with personality off.
2. **Two plain settings fields, not a group.** There are only two, like `personality_on`.
3. **A suggested list, with free typing.** Hindi users write "Hindi"; others keep any language they want. *Rejected:* a closed dropdown, which would drop the world's other languages that work today.
4. **Template wording, not code, for Translate and LNG-2.1.** A user who copies a built-in skill sees the same rules.
5. **CSS for right to left.** *Rejected:* `dir="auto"` on every element the Markdown renderer makes, which is more code for the same result.

## Test strategy
| Criterion | How |
|---|---|
| LNG-1.2 | The translate template rendered with My language Hindi says to translate into Hindi, or English when it's already Hindi. With English, it keeps Spanish. |
| LNG-1.3 | `reply_language`: personality off with the box off → None; box on → Hindi; a persona with Tamil wins over Hindi; personality off with the box on still gives Hindi. `system()` with None is byte-identical to the current snapshot, and with Hindi it says "Always reply in Hindi." with personality on and off. |
| LNG-1.4 | `language_name("hinglish")` mentions English letters; "Hindi" passes through. |
| LNG-1.5 | A template using `{{my_language}}` parses and renders; `{{language}}` follows LNG-1.3. |
| LNG-2.1 | The built-in `fix`, `rewrite` and `simplify` say to keep the text's own language. |
| Manual | LNG-1.1, 3.1 to 3.4, 4.1 on Windows; 1.1, 3.1, 3.2 in WSL (X11). |
