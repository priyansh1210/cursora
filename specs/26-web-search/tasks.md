# 26: Web search with sources (tasks)

Status: **In progress**.

- [x] 1. The constitution's §3 amendment; `web.rs` with tests. Covers WEB-1.2, WEB-1.3, WEB-1.4, WEB-1.5.
- [x] 2. `/web` in `ask`, `begin`, the worker and "What will be sent"; the system command. Covers WEB-1.2, WEB-1.3, WEB-1.6.
- [x] 3. The Web search group in Settings → Models; the manual. Covers WEB-1.1.
- [x] 4. All checks; by hand on Windows; then `/ponytail-review`. 2026-10-29: the first check in the app found "What will be sent" saying nothing is sent for `/web`, and the query losing its subject (only the typed question was searched); both fixed. The review found that the results weren't counted in the request's size, so a long local conversation could overflow (now 1,200 tokens are kept free), and that the design promised Stop would cancel the search (corrected).

## Manual checklist
- [ ] WEB-1.1 The key field in Models: saved, shown by its last 4 characters, removed. Windows ✓ (2026-10-29, test copy: "Saved •••• 1234" for a made-up key, Remove key, then empty) Linux X11 ☐
- [ ] WEB-1.2 A `/web` question answered from the results, with [1] citations and the Sources links. Windows ✓ with a fake search server (2026-10-29, test copy with `CURSORA_WEB_URL`: "Tauri 2" selected, `/web when was it released as stable?` searched "when was it released as stable? Tauri 2" with the key in the header; the built-in Small model answered from both results, without [1] marks, and the Sources list had both links) ✓ a real Tavily key, by the user (2026-10-29: `/web what is the latest version of Tauri?`, 5 results, answered by the built-in model with the Sources list) Linux X11 ☐
- [ ] WEB-1.4 Local-only on, then no key: the bubble says so and nothing is sent. Windows ✓ (2026-10-29: both messages, and the fake server got no request for either) Linux X11 ✓ (2026-10-29, the stronger PC's WSL, `scripts/x11-issues-check.sh` `before`: without a key, and in local-only mode, the messages)
