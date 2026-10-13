# 05: Personality (tasks)

Status: **In progress**.

- [x] 1. `persona.rs`: `Persona`, the four presets, `validate()` and `validate_all()` (unique names and ids), `active()`, `find()` and the `/persona` reply (tests first). Covers PER-1.1, 1.3, 3.1 (pure part).
- [x] 2. Settings `activePersona` and `personas`; `set_settings` refuses invalid personas with the reason. Covers PER-1.1.
- [x] 3. `prompt.rs` rewritten: base instructions with no name, the persona block, the untrusted-content rules, `<selected_text>` / `<file>` with escaping, snapshot tests with `UPDATE_SNAPSHOTS=1` (tests first). Covers PER-1.4, 2.1, 4.1, 4.2, 4.3.
- [x] 4. `chat.rs`: `assemble` takes the persona and skips local turns; `/persona` in `ask` adds a local turn; open conversations follow persona changes; Regenerate leaves local turns alone (tests first). Covers PER-3.1, 2.1.
- [x] 5. Commands: `personas`, `persona_preview`, `persona_add`, `persona_import`, `persona_export`, `prompt_preview`; the tray's "Persona ▸". Covers PER-1.2, 3.2, 4.4.
- [x] 6. UI: the Personality tab, Setup step 2, the header toggle, local turns as notes; `header()` with personality off (test first). Covers PER-1.2, 2.2, 2.3, APP-6.1 step 2.
- [ ] 7. End-to-end run with the built-in engine, then `/ponytail-review`. Linux ✓ (WSL: Xvfb + Openbox) and the review ✓ (2026-10-13); Windows ☐ (the screen was locked, issue 21).

## Manual checklist
- [ ] PER-1.2 The Personality tab creates, edits, duplicates, deletes, imports and exports personas, with a live preview. Windows ☐ Linux ✓ new, edit with custom tone and language, live preview, a taken and an empty name refused (Save disabled), duplicate ("Ada 2"), export, import ("Maya 2"), delete after a confirm, focus back on Edit (auto-verified 2026-10-13 in WSL: Xvfb + Openbox)
- [ ] PER-2.2 The toggle in the tray, the bubble header and the Personality tab applies from the next message. Windows ☐ Linux ✓ header and tab, the header follows a change made in Settings while it is open (auto-verified 2026-10-13 in WSL) ☐ tray (no tray host in WSL)
- [ ] PER-3.2 The tray lists the personas with the active one checked; choosing one switches. Windows ☐ Linux ☐ (no tray host in WSL)
- [ ] PER-4.4 The prompt preview shows the exact messages. Windows ☐ Linux ✓ the persona, the delimited selection and the question; no persona with personality off, the rules kept (auto-verified 2026-10-13 in WSL)
- [ ] APP-6.1 step 2 Setup asks for a personality, Cursora preselected, "Personality off" offered. Windows ☐ Linux ✓ (auto-verified 2026-10-13 in WSL)
- [ ] A persona changes the answers of a real model: the name, the tone, emoji, the reply language. Windows ☐ Linux ✓ the name, with a 0.5B model: Ada "My name is Ada.", Arjun "I am Arjun, a male.", personality off no persona name (auto-verified 2026-10-13 in WSL) ☐ tone, emoji, language with a larger model
- PER-3.1 in the app: `/persona ad` switches to Ada and the header follows; `/persona a` lists Ada and Arjun; `/persona zed` lists all; `/persona` alone lists them; no model is asked, and the next question is sent without these replies. Linux ✓ (auto-verified 2026-10-13 in WSL) Windows ☐
