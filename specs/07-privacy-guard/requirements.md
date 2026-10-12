# 07: Privacy guard (PRI)

## Scope
- **In:**
  - Local-only mode
  - The app blocklist
  - PII redaction with the "what will be sent" preview
  - Confirming before a cloud send
  - Network discipline
- **Out:**
  - History storage and controls (BUB-9)
  - Key storage (LLM-6)
  - Log hygiene (APP-7.2)
  - Disabling Cursora per app with hotkey pass-through (backlog)

## Requirements

### PRI-1 Local-only mode
User story: As a privacy-conscious user, I want a switch that guarantees nothing leaves my computer.
- PRI-1.1 (auto) WHILE local-only mode is on THE SYSTEM SHALL refuse every request to a provider that is not local (LLM-8.1, LLM-8.2), with error `blocked_local_only`. This covers chat, model listing, connection tests, speech-to-text and text-to-speech. The check SHALL happen at a single network chokepoint in the Rust backend.
- PRI-1.2 (manual) WHILE local-only mode is on:
  - cloud providers SHALL be greyed out in every model picker, with an explanation
  - the tray and the bubble SHALL show "🔒 Local-only"
- PRI-1.3 (auto) WHEN local-only mode is turned on while the active model is a cloud model THE SYSTEM SHALL switch to the first available local model. If there is none, it SHALL show "No local model — set one up".

### PRI-2 App blocklist
User story: As a user, I never want Cursora to read from my password manager or banking apps.
- PRI-2.1 (auto) WHEN the capture flow starts AND the foreground app matches a blocklist rule THE SYSTEM SHALL run **no** capture strategy: no accessibility read, no simulated keys, no clipboard access. It SHALL show the notice "Cursora is blocked in <app>" near the cursor.
- PRI-2.2 (auto) A rule SHALL match on either:
  - a **process name** (case-insensitive, exact), or
  - a **window-title keyword** (case-insensitive substring)
- PRI-2.3 (auto) THE default rules SHALL block the process names:
  - password managers: `KeePass.exe`, `KeePassXC.exe`, `1Password.exe`, `Bitwarden.exe`, `Enpass.exe`
  - Windows credential prompts: `CredentialUIBroker.exe`, `consent.exe`
  - Linux equivalents: `keepassxc`, `bitwarden`, `1password`
- PRI-2.4 (manual) THE tray SHALL offer "Block Cursora in <current app>", which adds a rule in one click. The Privacy tab SHALL list, add, edit and remove rules.
- PRI-2.5 (manual) WHERE the foreground app can't be identified (e.g. Linux Wayland) THE Privacy tab SHALL warn that the blocklist can't be enforced in this session (LNX-4.3).

### PRI-3 PII redaction for cloud requests
User story: As a user, I want my emails, card numbers and ID numbers removed before text goes to a cloud AI.
- PRI-3.1 (auto) WHEN a request goes to a cloud provider AND redaction is on (the default) THE SYSTEM SHALL replace detected sensitive values in all user content (captured text, file contents, questions, earlier turns) with typed placeholders: `[EMAIL_1]`, `[PHONE_1]`, `[CARD_1]`, `[IBAN_1]`, `[APIKEY_1]`, `[AADHAAR_1]`, `[PAN_1]`, `[CUSTOM_<label>_1]`. Within a conversation, the same value SHALL always map to the same placeholder.
- PRI-3.2 (auto) THE detectors SHALL be:

  | Value | How it is detected |
  |---|---|
  | email | email address pattern |
  | phone | E.164 and common national formats, including Indian 10-digit mobile numbers |
  | payment card | 13–19 digits (spaces and dashes allowed) that pass the Luhn check |
  | IBAN | passes the mod-97 check |
  | API key or token | known prefixes `sk-`, `sk-ant-`, `AKIA`, `ghp_`, `github_pat_`, `xoxb-`/`xoxp-`, `AIza`, plus JWTs |
  | Aadhaar | 12 digits, not starting with 0 or 1, that pass the Verhoeff check |
  | PAN | `[A-Z]{5}[0-9]{4}[A-Z]` |

  Each detector SHALL have positive and negative test cases.
- PRI-3.3 (auto) WHEN the answer streams back THE SYSTEM SHALL turn placeholders back into their original values locally before display. This SHALL also work when a placeholder is split across stream chunks.
- PRI-3.4 (auto) THE user SHALL be able to switch each detector on or off, and to add custom regex patterns with a label. An invalid regex SHALL be rejected with the error.
- PRI-3.5 (auto) Redaction SHALL NOT apply to local providers unless "Redact for local models too" is on (default off).
- PRI-3.6 (manual) THE bubble SHALL have an expandable "What will be sent" panel. It shows the outgoing text after redaction, with placeholders highlighted, and says which provider it goes to.
- PRI-3.7 (auto) WHERE "Confirm before sending to cloud" is on (default off) THE SYSTEM SHALL show the preview and wait for an explicit **Send** before any cloud request.
- PRI-3.8 (auto) WHEN an image would be sent to a cloud provider THE preview SHALL warn that images are not redacted.

### PRI-4 Network discipline
- PRI-4.1 (manual) THE SYSTEM SHALL make network requests only to:
  - providers the user configured
  - the local runtime probes on `localhost`
  - downloads the user started (models, speech models)

  v1 has no telemetry, no analytics and no automatic update checks. This SHALL be verified with a network monitor during the manual checklist.

## Open questions
- _(none)_
