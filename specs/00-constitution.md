# Cursora Constitution

These rules are non-negotiable. Specs, designs, tasks and code must all follow them. Changing a rule requires an explicit, dated amendment at the bottom of this file.

## 1. Specs are the source of truth
- No feature code is written until its spec's `requirements.md` and `design.md` are **Approved** in [README.md](README.md).
- Behavior changes go into the spec first, then into the code.
- Each spec folder contains:
  - `requirements.md`: user stories and acceptance criteria
  - `design.md`: how it will be built
  - `tasks.md`: ordered implementation steps and the manual checklist

## 2. Every criterion is verified
- Acceptance criteria use EARS phrasing: `WHEN <trigger> THE SYSTEM SHALL <response>`, plus `IF/WHILE/WHERE` variants.
- Criterion IDs have the form `<PREFIX>-<requirement>.<criterion>` (e.g. `SEL-3.1`). An ID is never reused, even after its criterion is deleted.
- Each criterion is tagged:
  - **(auto)**: at least one automated test whose name contains the ID.
    - Rust: `fn sel_3_1_terminal_never_gets_ctrl_c()`.
    - TypeScript: `it('SEL-3.1 …')`.
  - **(manual)**: listed in the spec's manual checklist in `tasks.md`, and verified on every supported platform before the spec is marked Done.
- `scripts/trace.mjs` runs in CI. It fails when:
  - an (auto) criterion has no test, or
  - a test cites an ID that no spec defines.
- CI runs `trace`, `cargo test` and `vitest` on Windows and Linux.

## 3. Privacy first
- API keys are stored only in the OS keychain. They never reach the webview, logs, config files or history.
- Local-only mode is absolute. While it is on, no byte of user content leaves the machine.
- No telemetry and no analytics. Network calls go only to providers the user configured.

## 4. Graceful degradation
- Every core feature works with a text-only model.
- Features that need a capability (vision, voice) hide or disable themselves when it is missing, and explain why. They never error out.

## 5. User content is untrusted
- Selected text, file contents and screenshots are data, not instructions.
- Prompts wrap them in explicit delimiters and tell the model not to follow instructions found inside them.

## 6. Never harm the host app
- After any capture, the user's clipboard is restored.
- Cursora never sends a key combination that is destructive in the target app (e.g. Ctrl+C to a terminal).
- Cursora never types into or modifies another app unless the user explicitly asks it to.

## 7. Platform support
- OS-specific code sits behind traits (`SelectionReader`, `CursorLocator`, `ActiveApp`, …) in `#[cfg(target_os)]` modules. Shared logic is OS-independent and unit-tested.
- Windows 10/11 and Linux X11 are fully supported.
- Linux Wayland is best-effort, and every fallback is documented in its spec.

## 8. Performance budgets
- The bubble becomes visible ≤ 200 ms after the hotkey, on a typical machine, with a non-UIA fallback path.
- Idle RAM is ≤ 60 MB, excluding any local model process.
- The first streamed token is shown as soon as the provider sends it. No buffering of whole responses.

## 9. Simplicity
- Nothing is built that no spec asks for. Post-v1 ideas live in [backlog.md](backlog.md) until promoted into a spec.

---
### Amendments
- **2026-10-11, §3:** "Leaves the machine" means leaves this computer *or a LAN host the user explicitly marked as trusted* (LLM-8.2). Public hosts can never be trusted.
