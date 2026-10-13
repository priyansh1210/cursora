# 05: Personality (tasks)

Status: **In progress**.

- [x] 1. `persona.rs`: `Persona`, the four presets, `validate()` and `validate_all()` (unique names and ids), `active()`, `find()` and the `/persona` reply (tests first). Covers PER-1.1, 1.3, 3.1 (pure part).
- [x] 2. Settings `activePersona` and `personas`; `set_settings` refuses invalid personas with the reason. Covers PER-1.1.
- [x] 3. `prompt.rs` rewritten: base instructions with no name, the persona block, the untrusted-content rules, `<selected_text>` / `<file>` with escaping, snapshot tests with `UPDATE_SNAPSHOTS=1` (tests first). Covers PER-1.4, 2.1, 4.1, 4.2, 4.3.
- [x] 4. `chat.rs`: `assemble` takes the persona and skips local turns; `/persona` in `ask` adds a local turn; open conversations follow persona changes; Regenerate leaves local turns alone (tests first). Covers PER-3.1, 2.1.
- [x] 5. Commands: `personas`, `persona_preview`, `persona_add`, `persona_import`, `persona_export`, `prompt_preview`; the tray's "Persona ▸". Covers PER-1.2, 3.2, 4.4.
- [x] 6. UI: the Personality tab, Setup step 2, the header toggle, local turns as notes; `header()` with personality off (test first). Covers PER-1.2, 2.2, 2.3, APP-6.1 step 2.
- [x] 7. End-to-end run with the built-in engine on Windows and Linux (WSL: Xvfb + Openbox, stalonetray for the tray), then `/ponytail-review` (2026-10-13).

## Manual checklist
- [x] PER-1.2 The Personality tab creates, edits, duplicates, deletes, imports and exports personas, with a live preview. Windows ✓ the same steps, export and import through the Windows file dialogs (auto-verified 2026-10-13) Linux ✓ new, edit with custom tone and language, live preview, a taken and an empty name refused (Save disabled), duplicate ("Ada 2"), export, import ("Maya 2"), delete after a confirm, focus back on Edit (auto-verified 2026-10-13 in WSL: Xvfb + Openbox)
- [x] PER-2.2 The toggle in the tray, the bubble header and the Personality tab applies from the next message. Windows ✓ header, tab and tray; the open bubble's header follows each (auto-verified 2026-10-13) Linux ✓ header, tab and tray (stalonetray); the header follows a change made in Settings or the tray while it is open (auto-verified 2026-10-13 in WSL)
- [x] PER-3.2 The tray lists the personas with the active one checked; choosing one switches. Windows ✓ (auto-verified 2026-10-13) Linux ✓ (auto-verified 2026-10-13 in WSL, stalonetray hosting the icon)
- [x] PER-4.4 The prompt preview shows the exact messages (redaction comes with spec 07). Windows ✓ (auto-verified 2026-10-13) Linux ✓ the persona, the delimited selection and the question; no persona with personality off, the rules kept (auto-verified 2026-10-13 in WSL)
- [x] APP-6.1 step 2 Setup asks for a personality, Cursora preselected, "Personality off" offered. Windows ✓ (auto-verified 2026-10-13) Linux ✓ (auto-verified 2026-10-13 in WSL)
- [ ] A persona changes the answers of a real model: the name, the tone, emoji, the reply language. Windows ✓ the name, with a 0.5B model: Ada "My name is Ada, a female.", Arjun "My name is Arjun, …", personality off "Your name is not specified in the given text." (auto-verified 2026-10-13) Linux ✓ the name, same model: "My name is Ada.", "I am Arjun, a male.", none with personality off (auto-verified 2026-10-13 in WSL) ☐ tone, emoji, language with a larger model (issue 25)
- PER-3.1 in the app: `/persona ad` switches to Ada and the header follows; `/persona a` lists Ada and Arjun; `/persona zed` lists all; `/persona` alone lists them; no model is asked, and the next question is sent without these replies. Linux ✓ (auto-verified 2026-10-13 in WSL) Windows ✓ (auto-verified 2026-10-13)
