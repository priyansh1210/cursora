# 06: Skills (tasks)

Status: **In progress**.

- [x] 1. `skills.rs`: the frontmatter parser and validator with line numbers, `render()`, `load()` (built-ins, the user's folder, overrides, duplicates), `closest()`, the command list, `/help` text; the nine built-in files (tests first). Covers SKL-1.1–1.4, 2.3, 2.6, 2.7 (help), 3.1, 4.5.
- [x] 2. `llm`: `pick()` and `for_skill()` (the skill's model, else the active one with a note), the temperature in all three request formats, `ModelInfo.vision` (tests first). Covers SKL-2.4, 2.5.
- [x] 3. `prompt.rs` and `chat.rs`: the skill slot in the system message; `command()` with names that ignore case; `/help`, `/clear`, `/new-skill`, `/snip`, unknown skills and refusals as local replies; skills run from `begin`, so Retry and Regenerate repeat them; `Turn.note` (tests first). Covers SKL-2.2, 2.3, 2.4, 2.7.
- [x] 4. Commands for the bubble and the Skills tab: `skills_available`, `skills_list`, `skill_check`, `skill_save`, `skill_test`, `skill_delete`, `skill_import`, `skill_export`, `skill_draft`, `skill_new`; "Open folder" (save tested). Covers SKL-4.4, 4.6.
- [x] 5. UI: the autocomplete in the bubble (`skills.ts`, vitest), notes and "Save as skill" on turns, Settings → Skills with the form, Test, duplicate, delete, import, export. Covers SKL-2.1, 4.1, 4.2, 4.3, 4.6.
- [x] 6. End-to-end run on Windows and in WSL with the built-in engine; the evaluation sheet (SKL-3.2) with the 0.5B model; `/ponytail-review` (2026-10-13).

## Manual checklist
- [ ] SKL-3.2 Each built-in skill gives a sensible result on its sample, with one local and one cloud model ([evaluation.md](evaluation.md)). Local ☐ only a 0.5B model so far: `/translate` sensible, most others not (issue 26) Cloud ☐ (issue 16)
- [x] SKL-4.1 Settings → Skills → New: the fields, the variable picker, live validation, Test. Windows ✓ bad and reserved names refused, {{input}} inserted at the cursor, the instructions as they will read, Test, Save writes `eli5.md` (auto-verified 2026-10-13) Linux ✓ (auto-verified 2026-10-13 in WSL)
- [x] SKL-4.2 `/new-skill` opens the form with the previous question as the template. Windows ✓ (empty right after `/clear`) (auto-verified 2026-10-13) Linux ✓ (auto-verified 2026-10-13 in WSL)
- [x] SKL-4.3 "Save as skill" on a question opens the form prefilled. Windows ✓ (auto-verified 2026-10-13) Linux ✓ (auto-verified 2026-10-13 in WSL)
- [x] SKL-4.6 Duplicate a built-in, delete a user skill, import and export a file, open the folder. Windows ✓ duplicate makes an "Overrides built-in" /summary, delete asks first, export and import through the file dialogs, Explorer opens the folder (auto-verified 2026-10-13) Linux ✓ the same, except Open folder (no file manager in WSL) (auto-verified 2026-10-13 in WSL)
- [ ] The autocomplete with the keyboard, and with a screen reader's announcements (Narrator / Orca). Windows ✓ keyboard: '/' lists 13, prefix then fuzzy, ↑/↓, Tab and Enter complete, Enter runs a full name, Esc closes and keeps the text (auto-verified 2026-10-13) Linux ✓ keyboard (auto-verified 2026-10-13 in WSL) ☐ Narrator ☐ Orca
- SKL-2.3 / 2.7 in the app: `/sumary` → "Did you mean /summary?", `/help`, `/snip` "isn't available yet", `/Persona ada` (issue 23), `/clear` keeps the capture. Windows ✓ Linux ✓ (auto-verified 2026-10-13)
- SKL-4.5 in the app: a file added, broken or deleted outside Cursora shows in the Skills tab within a second. Windows ✓ 784 ms and 738 ms Linux ✓ 195 ms and 474 ms (auto-verified 2026-10-13)
