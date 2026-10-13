# 06: Skills (design)

Status: **Approved** (2026-10-13). Implements [requirements.md](requirements.md). It fills the skill slot that spec 05 left in the prompt, and turns `/persona` into one of several system commands.

## Overview
```
skills/*.md (built-in, compiled in) ─┐
<config dir>/skills/*.md (the user's) ┴─→ skills::load()  read on use, no cache
                                              ├─ valid skills (user ones override built-ins)
                                              └─ invalid files, with line and reason (SKL-1.3)
Bubble input "/…" ─→ autocomplete (skills_available, filtered in TS)          (SKL-2.1)
ask("/name rest") ─→ chat::command()
     ├─ persona · help · clear · new-skill · snip   system commands, answered locally
     ├─ a skill ─→ begin(): skill instructions in the system message, its model and temperature
     └─ unknown ─→ local reply "Unknown skill", the closest names, nothing sent
Settings → Skills ─→ list, form (fields → Rust writes the .md), Test, duplicate, delete, import, export
```
- **One format, one parser.** Built-in skills are `.md` files in the same format as the user's (`src-tauri/skills/`, compiled in with `include_str!`), so "Duplicate" is a plain copy and every built-in is checked by the same validator (SKL-3.1).
- **Rust owns the format.** The form sends fields; Rust writes the file, validates and renders. TypeScript only filters the autocomplete list.

## Components

| File | Job |
|---|---|
| `src-tauri/src/skills.rs` (new) | `Skill`, `Invalid`, the frontmatter parser, `validate()`, `load()`, `render()`, `closest()`, the built-ins, the commands for the Skills tab. |
| `src-tauri/skills/*.md` (new) | The nine built-in skills. |
| `src-tauri/src/prompt.rs` | `system(persona, skill)`: the skill's rendered instructions go between the persona and the untrusted-content rules (PER-4.1). |
| `src-tauri/src/chat.rs` | `command()`: parses `/name rest` and dispatches; `begin` takes the skill from the latest question, so Retry and Regenerate re-run it; `Turn.note`; `/clear`, `/help`. |
| `src-tauri/src/llm/mod.rs` | `for_skill()`: the skill's model if available, else the active one with a note (SKL-2.5); a temperature on the request. |
| `src/Chat.tsx`, `src/skills.ts` (new) | The autocomplete list (a combobox), "Save as skill" on user questions, notes on turns. `skills.ts` holds the pure filter and key handling (vitest). |
| `src/Skills.tsx` (new) | Settings → Skills (SKL-4.1, 4.6). |

## The skill file (SKL-1.1)
```markdown
---
name: eli5
description: Explain like I'm five
args: <topic>
requires: [vision]
model: Ollama/llama3.2
temperature: 0.3
output: chat
---
Explain the captured content as if to a five-year-old. Focus on: {{input}}
```
- **Frontmatter subset.** The fields are flat, so Cursora reads a small, strict subset of YAML instead of adding a YAML library:
  - `key: value` lines; values may be quoted with `"` or `'`;
  - lists either inline (`[vision]`) or as `- item` lines under the key;
  - `#` comments and blank lines.

  Anything else is an error with its line number, e.g. "line 4: unknown field 'descripton'" or "line 6: expected 'key: value'". `// shortcut:` YAML subset; switch to a YAML crate if users need multi-line values.
- **Rules:**
  - `name` matches `^[a-z0-9][a-z0-9-]{0,31}$` and isn't reserved (SKL-2.6).
  - `description` is 1–120 characters; `args` is at most 40.
  - `requires` may only contain `vision`; `output` may only be `chat`.
  - `temperature` is a number from 0 to 2.
  - `model` is `<provider>/<model id>`. The provider is matched by name, ignoring case, or by id. The model id may itself contain `/` (OpenRouter), so the split is at the first `/`.
  - The template (the body) is not empty.
  - Files over 64 KB are skipped as invalid.
- **Variables (SKL-1.2):** `{{input}}`, `{{app}}`, `{{language}}`, `{{date}}`, with optional spaces inside the braces.
  - `{{app}}` is the source app's name, never the window title (SEL-7.2).
  - `{{language}}` is the persona's reply language, or "the language of the user's question" when there is none or personality is off.
  - `{{date}}` is today's local date, `2026-10-13`, from the OS.
  - Any other `{{…}}` is an error at its line.

## Loading (SKL-1.4, 1.3, 4.5)
- `load(dir)` returns the valid skills and the invalid files.
  - The built-ins come first.
  - Then the user's `*.md` files, in alphabetical order. A user skill with a built-in's name replaces it and is marked "overrides built-in". A second user file with a name already taken is invalid: "the name 'x' is already used by a.md".
- **Read on use, no cache.** The folder is read when the autocomplete opens, when a command is submitted, and by the Skills tab, which also re-reads it every second while it is on screen. A file added, changed or deleted outside Cursora therefore counts at the very next use, well within SKL-4.5's second, with no watcher thread and no new dependency. A few small files take well under a millisecond.

## Using skills (SKL-2)

### Commands
`chat::ask` reads `/<name> <rest>`. The name is matched ignoring case, which also fixes issue 23 for `/Persona`. Then:

| Name | Does |
|---|---|
| `persona` | As in spec 05. |
| `help` | A local reply listing the available skills (name, args, description), then the commands (SKL-2.7). |
| `clear` | A new conversation with the same capture, model and persona. The old one stays in history (SKL-2.7). |
| `new-skill` | Opens Settings → Skills → New with the previous question of this conversation as the template (SKL-4.2). |
| `snip` | Reserved for spec 10. Until then it replies "Snipping isn't available yet." and is left out of `/help` and the autocomplete. |
| a skill | Runs it (below). |
| anything else | A local reply: "Unknown skill /sumary. Did you mean /summary?". The closest names are those within an edit distance of 2 or starting with the typed name, at most 3. Nothing is sent (SKL-2.3). |

Local replies are spec 05's local turns: shown and saved, never sent.

### Running a skill (SKL-2.2, 2.4, 2.5)
- **The turn's question is what the user typed** (`/translate French`). `begin` finds the skill from the latest question each time, so Retry, Regenerate and a conversation reopened from History run it again.
- **The request:** the system message gets the rendered template, with `{{input}}` = the trimmed rest, under "Instructions for this request:". The user message is the capture, then the typed command, as for any question. Earlier turns are sent as they were typed.
- **`requires: vision`:** a skill the answering model can't serve is hidden from the autocomplete. If typed anyway, it gets a local reply: "/x needs a vision model, and qwen can't see images."
- **`model`:** `llm::for_skill()` uses that model if it is configured and allowed (local-only mode still applies, PRI-1). Otherwise it uses the active model, and the turn gets a note: "Ollama/llama3.2 isn't available, so qwen answered." `Turn.note` is shown under the question and saved.
- **`temperature`** is sent in the request (OpenAI-compatible, Anthropic and Gemini each in their own field) only when the skill sets it.

### Autocomplete (SKL-2.1)
- When the input starts with `/` and has no space yet, the bubble asks Rust once for `skills_available()`: name, args, description and kind, already filtered for the answering model. `skills.ts` then filters as the user types:
  1. names that start with the typed text, alphabetically;
  2. then names that contain the typed letters in order (the fuzzy match), alphabetically.
- **Keys:** ↑ and ↓ move the selection. Tab or Enter completes to `/name ` (plus nothing else, so the user can type the args). Enter on a name already typed in full runs it. Esc closes the list and keeps the text. Enter with the list closed sends as usual.
- **Accessibility:** the input keeps its text-box role (a `textarea` may not take another), with `aria-autocomplete="list"`, `aria-controls` and `aria-activedescendant`; the list is a `listbox` of `option`s.

## Creating skills (SKL-4)

### Settings → Skills
- **The list:**
  - Built-ins are marked "Built-in", with Duplicate.
  - User skills offer Edit, Export and Delete (Delete asks first), and show "overrides built-in" where that applies.
  - Invalid files are shown with their file name, line and reason.
  - At the end: **New**, **Import…** and **Open folder**.
- **The form (SKL-4.1):**
  - **Fields:** name, description, args hint, a vision checkbox (`requires`), a model picker (the configured models, or "the active model"), temperature (empty = the model's default), and the template in a text area.
  - **Variable picker:** inserts `{{input}}` and the others at the cursor.
  - **Live validation:** `skill_check(fields)` returns Rust's verdict with the line, or the rendered example.
  - **Test** runs the skill once on a sample text (editable, prefilled with a paragraph) with the chosen model, and shows the answer below the form.
  - **Save** writes `<skills folder>/<name>.md` (SKL-4.4). Saving a new skill under a name that already has a user file is refused. Renaming a user skill writes the new file and deletes the old one.
- **Duplicate a built-in (SKL-4.6)** opens the form prefilled with it under the same name, with a note that saving overrides the built-in; a different name keeps both.
- **Import** checks the file, then copies it into the folder; it refuses a name already taken by a user skill. **Export** saves the file through a save dialog. **Open folder** uses the existing `open_folder`.

### From the bubble (SKL-4.2, 4.3)
- `/new-skill`, and **Save as skill** on any user question, both open the same form in Settings → Skills, prefilled: the template is that question, and the name is empty. The bubble stays as it is.
- A pending draft waits in Rust until the Skills tab takes it, so the form fills both when Settings opens and when it is already open.

## Commands

| Command | Does |
|---|---|
| `skills_available()` | Autocomplete entries for the calling window's model: skills plus the available commands. |
| `skills_list()` | The Skills tab: every skill with its source (built-in, user, overrides built-in) and file, plus the invalid files. |
| `skill_check(fields)` | The file that would be written, validated: OK with a rendered example, or the error and its line. |
| `skill_save(fields, path)` | Writes the file; `path` is the file being edited, if any. Returns the list. |
| `skill_test(fields, sample)` | Runs the skill once on the sample and returns the answer or the error. |
| `skill_delete(path)` | Deletes a user skill file. |
| `skill_import()`, `skill_export(name)` | File dialogs, as for personas. |
| `skill_draft()` | Takes the pending draft from `/new-skill` or Save as skill, if any. |
| `skill_new(question)` | Save as skill: stores the draft and opens Settings → Skills. |

## Built-in skills (SKL-3.1)

| Name | Args | Instructions (summary) |
|---|---|---|
| `summary` | | A few short bullet points with the key facts, names and numbers. |
| `explain` | | What it means, why it matters, and any terms a reader may not know. |
| `simplify` | | Simpler words and shorter sentences, same meaning; only the new text. |
| `define` | | A one-line definition of the word or phrase in its context, then an example. |
| `translate` | `<language>` | Into the given language (empty means English, or Spanish if the text is already English); only the translation. |
| `fix` | | Grammar, spelling and punctuation, keeping meaning and tone; the corrected text, then a short list of changes. |
| `rewrite` | `<tone>` | In the given tone (empty means clearer and more concise); only the new text. |
| `code` | | What the code does, step by step, and the language. |
| `bugs` | | Each bug: where, what goes wrong, how to fix it; says so plainly if there are none. |

The full wording is in the files. SKL-3.2's evaluation sheet is `specs/06-skills/evaluation.md`: for each skill, a sample input with one local and one cloud model's answer, judged as sensible or not.

## Changes to earlier specs' code
- **Spec 05:**
  - `prompt::system` gains the skill parameter, and the snapshots gain a skill case.
  - `/persona` moves into `command()`.
  - Command names match ignoring case (issue 23).
- **Spec 03:**
  - Turns gain `note`.
  - The bubble input gains the autocomplete and "Save as skill".
- **Spec 04:** `chat_request` takes an optional temperature.

## Dependencies
None new: a small frontmatter parser instead of a YAML crate, and reading on use instead of a file watcher.

## Decisions
1. **Read the folder on use instead of watching it.** It costs nothing while idle and needs no dependency, and every use sees the latest files (SKL-4.5).
2. **A strict YAML subset.** The format has only flat fields; errors carry line numbers either way (SKL-1.3).
3. **The typed command is the question.** The skill is found again from it, so Retry, Regenerate and History need no extra state.
4. **One skill form, in Settings.** `/new-skill` and "Save as skill" open it prefilled instead of a second, compact form inside the bubble (SKL-4.2 asks for "a compact skill form"; this is the same form, opened from the bubble).
5. **`/snip` stays hidden** until spec 10 makes it work.

## Known limits
- `// shortcut:` the built-in skills' instructions are in English; models follow them for any language.
- `// shortcut:` a skill's `model` names a provider by its display name; renaming the provider breaks the reference until the skill is edited (it then falls back with a note).

## Test strategy

| Criterion | Test |
|---|---|
| SKL-1.1 | Parser and validator tables: every field's valid and invalid values, quoted values, inline and block lists, comments; each error names its line. |
| SKL-1.2 | `render()` fills all four variables; an unknown `{{variable}}` is an error at its line; the capture is never part of the template. |
| SKL-1.3 | A folder with bad YAML, a missing field, a bad name, an unknown variable, and a duplicate and a reserved name: each is reported with its line, and the valid skills still load. |
| SKL-1.4 | Built-ins plus a user folder; a user skill named `summary` overrides and is marked; two user files with one name: the first alphabetically wins. |
| SKL-2.1 | `skills.ts`: prefix matches before fuzzy ones; ↑/↓, Tab, Enter, Esc on the list state (vitest). |
| SKL-2.2 | `/translate  French ` runs `translate` with `{{input}}` = "French"; the request's system message has the instructions between the persona and the rules (snapshot). |
| SKL-2.3 | `closest()`; an unknown command adds a local turn and asks no model. |
| SKL-2.4 | A vision skill is left out of `skills_available` for a text-only model, and is refused with the reason. |
| SKL-2.5 | `for_skill()`: the skill's model when configured, the active one with a note when not, and when local-only mode blocks a cloud model. |
| SKL-2.6 | Each reserved name is invalid as a user skill. |
| SKL-2.7 | `/help` lists skills and commands, not `snip`; `/clear` keeps the capture and empties the turns. |
| SKL-3.1 | The nine built-ins load and pass validation. |
| SKL-4.4 | `skill_save` writes `<name>.md`, and the next `load()` has it. |
| SKL-4.5 | A temporary folder: a file added, changed and deleted between two loads shows each change. |

Manual: SKL-3.2 (the evaluation sheet), SKL-4.1, 4.2, 4.3, 4.6, and the autocomplete with a screen reader.

## Changes during implementation
- **2026-10-13, build:**
  - **The command names are constants in `skills.rs`** (`PERSONA`, `HELP`, `CLEAR`, `NEW_SKILL`, `SNIP`). `chat.rs` dispatches on them, and `/help`, the autocomplete and the reserved-name check read the same list, so a name is written once.
  - **The reply-language fallback is one constant**, `prompt::QUESTION_LANGUAGE`, used by the persona block and by `{{language}}` (`skills::language()`).
  - **`skill_test` and the prompt preview share `chat::sample()`**, and the skills folder has one definition (`skills::dir()`), which "Open folder" uses too.
  - **The input keeps its text-box role** (see Autocomplete); the design first said combobox, which a `textarea` may not take.
- **2026-10-13, end-to-end run:**
  - **Enter on a name typed in full runs it** (SKL-2.1 amended). Before, every command needed two Enters: the first completed `/help` to `/help `.
  - **The autocomplete crashed the bubble on the second letter.** Newer WebView2 builds return a Promise from `scrollIntoView`, and a one-line effect handed it to React as its cleanup. The effect now has a block body.
  - **The Skills tab re-reads the folder twice a second**, so a file changed outside Cursora shows within a second (once a second could take 1.1 s).
- **2026-10-13, review:**
  - **Saving checks for the same file, not the same spelling**: on Windows, `ELI5.md` is `eli5.md`, so a hand-made file can be edited, and a rename never deletes the file just written. Import uses the same check.
  - **`render` looks for `}}` after the `{{`**, so a lone `}}` earlier in the template no longer stops the variables being filled in.
