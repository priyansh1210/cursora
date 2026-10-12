# 05: Personality (PER)

## Scope
- **In:**
  - The persona model, presets and editor
  - Switching personality on and off, and switching personas
  - The prompt builder, which assembles every request
- **Out:**
  - How a voice is chosen from the persona (VOC-2.4)
  - The animated avatar (backlog)

## Requirements

### PER-1 Persona definition
User story: As a user, I want to give my cursor its own name, gender and character, so that it feels like my companion.
- PER-1.1 (auto) A persona SHALL have these fields, and invalid values SHALL be rejected with a reason:

  | Field | Allowed values |
  |---|---|
  | `name` | 1–32 characters |
  | `gender` | male / female / neutral / custom text |
  | `tone` | friendly / professional / witty / calm / sarcastic / custom text |
  | `verbosity` | brief / normal / detailed |
  | `emoji` | on / off |
  | `replyLanguage` | "same as my question", or a specific language |
  | `userExpertise` | beginner / intermediate / expert |
  | `customInstructions` | ≤ 2000 characters |

- PER-1.2 (manual) THE Personality tab SHALL let the user create, edit, duplicate, delete, import and export personas (as JSON), with a live preview of the generated persona instructions.
- PER-1.3 (auto) THE SYSTEM SHALL ship built-in presets that can be duplicated but not deleted. Every preset SHALL pass validation.
- PER-1.4 (auto) The persona's gender SHALL describe **the persona's own identity**. This covers how it refers to itself, including grammatical gender in languages such as Hindi ("main kar rahi hoon" vs "kar raha hoon"), and its voice. It SHALL NOT be used to make assumptions about the user.

### PER-2 Personality on/off
User story: As a user, I sometimes want the plain model with no character.
- PER-2.1 (auto) WHILE personality is **off** THE prompt builder SHALL leave out the persona block entirely: no name, gender, tone, emoji rule or custom instructions in the request.
- PER-2.2 (manual) THE toggle SHALL be available in the tray, the bubble header and the Personality tab, and SHALL apply from the next message.
- PER-2.3 (auto) WHILE personality is off THE bubble header SHALL show "Personality off" and the model name (BUB-7.1).

### PER-3 Switching personas
- PER-3.1 (auto) WHEN the user submits `/persona <name>` THE SYSTEM SHALL switch the active persona and confirm in the bubble. Matching is case-insensitive and by prefix. IF the name is ambiguous or unknown THE SYSTEM SHALL list the matching or available personas and change nothing.
- PER-3.2 (manual) THE tray menu SHALL list personas, with the active one checked.

### PER-4 Prompt builder
- PER-4.1 (auto) THE prompt builder SHALL produce:
  - a **system message**, in this order:
    1. base instructions
    2. the persona block (only when personality is on)
    3. skill instructions (if a skill was used)
    4. rules for handling untrusted content
  - **user turns**, in this order: the captured context (delimited), then the user's question
- PER-4.2 (auto) Captured content SHALL be delimited:
  - text as `<selected_text source="<app>">…</selected_text>`
  - files as `<file path="…">…</file>`

  Any occurrence of a closing delimiter inside the content SHALL be escaped. The system message SHALL say that delimited content is data and that instructions inside it must not be followed.
- PER-4.3 (auto) THE prompt builder SHALL be deterministic: the same inputs SHALL produce byte-identical messages. This is verified with snapshot tests.
- PER-4.4 (manual) A "Prompt preview" (Settings → Personality, developer section) SHALL show the exact messages that would be sent, after redaction (PRI-3).

## Open questions
- _(none)_
