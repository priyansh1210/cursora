# Cursora

AI cursor companion: select text anywhere → hotkey → chat bubble at the cursor. Tauri v2 (Rust) + React/TS, Windows + Linux.

## Rules for every session
- **Spec-driven:** read [specs/00-constitution.md](specs/00-constitution.md) and [specs/README.md](specs/README.md) first. Feature code needs its spec's `requirements.md` **and** `design.md` Approved. Behavior changes go into the spec first.
- **Use the `ponytail` skill (level full) for all development.** Write the smallest complete change, and mark every known limit with `// shortcut: <limit>, <when to upgrade>`. Use `/ponytail-review` before marking a spec Done.
- **Tests cite criterion IDs:** Rust `fn sel_3_1_<what>()`, TS/JS `it('SEL-3.1 …')`.

## Commands
- `npm run trace`: the spec ↔ test traceability check
- `npm test`: JS tests
- `npm run build`, then `cargo test --manifest-path src-tauri/Cargo.toml`
- `npm run test:linux`: the same Rust tests, built for Linux in WSL (`Ubuntu-24.04`). Run it after `npm run build` and before every push.
- `npm run tauri dev`: run the app

Develop on Windows. Linux is built by CI (`.github/workflows/ci.yml`) and in WSL; it is tested by hand on a Live USB or VM.
