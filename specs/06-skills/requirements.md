# 06: Skills (SKL)

## Scope
- **In:**
  - The skill file format and built-in skills
  - `/` slash-command invocation and autocomplete
  - **Users creating, editing and sharing their own skills**, with hot reload
  - Reserved system commands
- **Out:**
  - `output: replace` (writing back into the app), which is in the backlog
  - `/snip`, which is a system command defined in VIS

## Requirements

### SKL-1 Skill file format
- SKL-1.1 (auto) A skill SHALL be a UTF-8 Markdown file with YAML frontmatter. The body is the skill's instruction template. Frontmatter fields:

  | Field | Required | Rules |
  |---|---|---|
  | `name` | yes | `^[a-z0-9][a-z0-9-]{0,31}$` |
  | `description` | yes | ≤ 120 characters |
  | `args` | no | hint text, e.g. `<language>` |
  | `requires` | no | list; v1 value: `vision` |
  | `model` | no | provider/model reference |
  | `temperature` | no | 0–2 |
  | `output` | no | v1 value: `chat` |

- SKL-1.2 (auto) Templates MAY use the variables `{{input}}` (text typed after the command), `{{app}}` (source app name), `{{language}}` (the persona's reply language) and `{{date}}`. Captured text and files SHALL **always** be attached automatically by the prompt builder (PER-4.2), so skills never need to place them. Any unknown `{{variable}}` SHALL be a validation error.
- SKL-1.3 (auto) WHEN a skill file is invalid (bad YAML, missing field, bad name, unknown variable, duplicate or reserved name) THE SYSTEM SHALL skip it and show it in Settings → Skills with the error and line number. It SHALL NOT crash, and SHALL keep the other skills working.
- SKL-1.4 (auto) Skills SHALL be loaded from two places:
  - the bundled built-ins (read-only)
  - the user's skills folder: `<app config dir>/skills/`, i.e. `%APPDATA%\com.cursora.app\skills\` on Windows and `~/.config/com.cursora.app/skills/` on Linux (see the 01 design)

  A user skill with the same name as a built-in SHALL override it and be labelled "overrides built-in". IF two user files use the same name THE alphabetically first file SHALL win, and the others SHALL be flagged invalid.

### SKL-2 Using skills
User story: As a user, I want to type `/summary` and get a summary of my selection.
- SKL-2.1 (auto) WHEN the input starts with `/` THE bubble SHALL show an autocomplete list of available skills and system commands (name, args hint, description), filtered by prefix and then fuzzy match as the user types. ↑/↓ moves the selection, Tab or Enter completes, and Esc closes the list. Enter on a name already typed in full runs it, so a command needs one Enter (amended 2026-10-13).
- SKL-2.2 (auto) WHEN the user submits `/<name> <rest>` THE SYSTEM SHALL run skill `<name>` with `{{input}}` set to the trimmed `<rest>`.
- SKL-2.3 (auto) WHEN `/<name>` matches no skill or command THE SYSTEM SHALL show "Unknown skill" with the closest names, and SHALL send nothing to the model.
- SKL-2.4 (auto) Skills whose `requires` the active model doesn't meet SHALL be hidden from autocomplete. If typed anyway, they SHALL be rejected with the reason (e.g. "needs a vision model").
- SKL-2.5 (auto) WHERE a skill sets `model` THE SYSTEM SHALL use that model for the request, subject to local-only mode (PRI-1). IF that model is unavailable THE SYSTEM SHALL use the active model and say so.
- SKL-2.6 (auto) THE names `persona`, `new-skill`, `snip`, `help` and `clear` SHALL be reserved for system commands. User skills with these names SHALL be invalid.
- SKL-2.7 (auto) `/help` SHALL list the available skills and commands. `/clear` SHALL start a new conversation that keeps the current capture.

### SKL-3 Built-in skills
- SKL-3.1 (auto) THE SYSTEM SHALL ship these built-in skills, and every one SHALL pass validation:
  - `summary`, `explain`, `simplify`, `define`
  - `translate` (args: `<language>`)
  - `fix` (grammar and spelling)
  - `rewrite` (args: `<tone>`)
  - `code` (explain code), `bugs` (find bugs)
- SKL-3.2 (manual) Each built-in skill SHALL produce a sensible result on its sample input, recorded in an evaluation sheet, with one local model and one cloud model.

### SKL-4 Creating your own skills
User story: As a user, I want to make my own commands (like `/eli5` or `/to-sql`) and use them immediately.
- SKL-4.1 (manual) Settings → Skills → **New** SHALL open a form with:
  - name, description, args hint, requires and model fields
  - a template editor with a variable picker and live validation
  - a **Test** button that runs the skill on sample text
- SKL-4.2 (manual) WHEN the user submits `/new-skill` in the bubble THE SYSTEM SHALL open a compact skill form, with the template pre-filled from the user's previous question in the conversation.
- SKL-4.3 (manual) Each user question in the bubble SHALL have **Save as skill**, which opens the same form pre-filled with that question.
- SKL-4.4 (auto) WHEN a skill is saved from any form THE SYSTEM SHALL write `<skills folder>/<name>.md`, and the skill SHALL be available in autocomplete immediately.
- SKL-4.5 (auto) WHEN a `.md` file in the skills folder is added, changed or deleted outside Cursora THE SYSTEM SHALL reload skills within 1 second without a restart (debounced).
- SKL-4.6 (manual) THE user SHALL be able to:
  - duplicate a built-in skill into an editable user skill
  - delete user skills
  - import and export `.md` skill files
  - open the skills folder

## Open questions
- _(none.)_

## Resolved
- Templates have no `{{selection}}` variable. Captured content is always attached by the prompt builder (confirmed).
