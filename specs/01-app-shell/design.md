# 01: App shell (design)

Status: **Approved** (2026-10-11). Implements [requirements.md](requirements.md).

## Overview
A windowless Tauri app that lives in the tray:
- The **Settings** and **Setup** windows are created on demand and destroyed when closed, which keeps idle RAM low (constitution §8).
- The bubble window belongs to spec 03.
- Settings live in one JSON file that the Rust backend owns. The UI reads and writes it through two commands.

```
main.rs → lib.rs::run()
  ├─ plugins: single-instance (first), log, dialog, opener; auto-launch crate for autostart
  ├─ settings::load()  → AppState { settings: Mutex<Settings>, paths }
  ├─ tray::build()     → menu from Settings; rebuilt on every settings change
  ├─ setup not done?   → windows::open("setup")
  └─ single-instance callback → cli::parse(argv) → actions::handle()
```

## Components

| File | Job |
|---|---|
| `src-tauri/src/settings.rs` | The `Settings` struct, `load(path) -> (Settings, Option<Warning>)` and `save(path, &Settings)`. Saving is atomic: write `settings.json.tmp`, then rename. |
| `src-tauri/src/cli.rs` | `parse(args) -> Vec<Action>`, where `Action` is `Ask \| Snip \| Settings \| Quit`. Unknown arguments are logged and ignored. |
| `src-tauri/src/tray.rs` | Builds the tray menu from `&Settings` and handles clicks. Sets the paused icon. |
| `src-tauri/src/lib.rs` | Wiring: plugins, state, the commands below, and `apply(&Settings)` (tray + autostart + emitting `settings-changed`). |
| `src/main.tsx` | Picks the view by window label: `settings` → `<Settings/>`, `setup` → `<Setup/>`. There is no router library. |
| `src/Settings.tsx` | Tab frame and the General and About tabs. Each other tab shows a placeholder line until its spec lands. |
| `src/Setup.tsx` | The setup wizard. It is a list of step components, and later specs add their own steps. |
| `src/HotkeyInput.tsx` | Records a key combination as `"Ctrl+Shift+G"`. Validation comes from SEL-1.2. |

### Settings (schemaVersion 1)
```rust
#[serde(default, rename_all = "camelCase")]
struct Settings {
    schema_version: u32,          // 1
    setup_done: bool,             // false → open Setup at start
    paused: bool,
    start_with_system: bool,      // default true (APP-3.2)
    theme: Theme,                 // system | light | dark
    hotkeys: Hotkeys,             // ask "Ctrl+G", snip "Ctrl+Shift+G", push_to_talk None
    personality_on: bool,         // tray toggle; PER gives it meaning
    local_only: bool,             // tray toggle; PRI gives it meaning
}
```
- Later specs add their own fields with defaults. `#[serde(default)]` lets an older file load with its values kept (APP-4.3). An explicit `migrate(Value)` step is added only when a field is renamed or changes type.
- **The config directory** is Tauri's `app_config_dir()`: `%APPDATA%\com.cursora.app\` on Windows and `~/.config/com.cursora.app/` on Linux. **Logs** go to `app_log_dir()`.

### Commands and events
| Command / event | Shape |
|---|---|
| `get_settings` | `() → Settings` |
| `set_settings` | `(Settings) → Result<(), String>`. Saves, then runs `apply()`. |
| `app_info` | `() → { version, os, session, configDir, logDir }`. `session` is `"windows"`, or `XDG_SESSION_TYPE` on Linux (spec 08 refines the detection). |
| event `settings-changed` | Payload `Settings`, sent to every window, so changes apply without a restart (APP-4.1). |

## Behavior
- **Tray (APP-1.2):** Open Settings · Persona ▸ · Personality on/off · Local-only on/off · Pause · Quit.
  - The Persona submenu stays empty until spec 05.
  - The paused icon is the app icon converted to grayscale at runtime, so there's no extra asset.
- **Pause (APP-1.3):** sets `paused` and swaps the icon. Spec 02's hotkey registration reads `paused`, so APP-1.3 is verified once spec 02 exists.
- **Quit (APP-1.4):** `app.exit(0)`. Hotkeys are unregistered when the plugin is dropped. Later specs that start child processes stop them in `RunEvent::Exit`.
- **CLI actions:**
  - `Settings` and `Quit` work now.
  - `Ask` and `Snip` log "not available yet" until specs 02 and 10 exist.
- **Corrupt settings (APP-4.2):** rename the file to `settings.bak-<unix-ts>.json`, use defaults, and show one native message (dialog plugin).
- **Autostart (APP-3.1):** `sync_autostart()` runs at startup and on every settings change. It uses the `auto-launch` crate directly in `CurrentUser` mode, which writes the per-user Run key.
- **Logging (APP-7.1):** tauri-plugin-log writes to `LogDir`, with `max_file_size(1_000_000)` and `RotationStrategy::KeepSome(5)`.
- **Setup (APP-6):** this spec ships the **Hotkeys** and **Start with system** steps and the "Re-run setup" button (APP-6.3). Specs 04, 05 and 03 add the **AI source**, **Personality** and **Try it** steps, so APP-6.1 is verified at the end of v1.

## Decisions
- **A plain serde JSON file instead of tauri-plugin-store.** serde is already a dependency. We need atomic writes, a backup on corruption and a schema version, and owning the file makes those a few lines each.
- **Settings and Setup windows are created on demand** instead of being pre-created. They're rarely opened, and this saves RAM. Only the bubble (spec 03) is pre-created, because of its speed budget.
- **No UI framework or router.** It's plain React with one CSS file and the window label as the "route".
- **Dialog plugin** for the one corruption message. It's needed later anyway for file pickers (`.gguf`, skill import/export).

## Requirement tweaks (approved, applied)
- **APP-7.1 (auto) → (manual).** The rotation itself is tauri-plugin-log's code. An automated test of our two constants would only repeat them. Manual check instead: write more than 5 MB of logs and confirm there are at most 5 files.
- **SKL-1.4 paths** become `<config dir>/skills/` (see above), replacing `%APPDATA%\cursora\skills\`.

## Test strategy
| Criterion | Test |
|---|---|
| APP-2.1 | Rust unit tests on `cli::parse` |
| APP-4.1 | Rust: save → load round trip in a temp dir |
| APP-4.2 | Rust: missing file → defaults; invalid JSON → backup file exists, defaults returned, warning returned |
| APP-4.3 | Rust: a v1 file missing newer fields keeps the user's values and gets defaults for the rest |
| APP-4.4 | Rust: the serialized `Settings` has no field name matching `key\|token\|secret\|password` |
| APP-7.2 | Node: scan `src-tauri/src/**/*.rs` and fail on log macros that use user-content identifiers (`text`, `selection`, `content`, `question`, `answer`, `api_key`, …) |
| APP-8.1 | Done: `scripts/trace.test.mjs` |
| Others | Manual checklist in `tasks.md` |

## Changes during implementation
- **2026-10-11:** tauri-plugin-autostart was replaced by direct `auto-launch` calls. The plugin hard-codes the "Dynamic" mode, which wrote the machine-wide HKLM Run key on this PC, while APP-3.1 says per-user. `auto-launch` was already a transitive dependency.
- **2026-10-11:** autostart is also synced at startup. Otherwise closing Setup early left "Start with system ✓" in Settings without an OS entry (`/ponytail-review` finding 1).
- **2026-10-11:** APP-2.4 was added: relaunching Cursora with no arguments opens Settings (review finding 3).
- **2026-10-13, manual checks:** log rotation keeps 4 old files (`KeepSome(4)`), since the plugin counts old files only; with the current one that makes the 5 of APP-7.1. It kept 6.
- **2026-10-13:** with Cursora not running, `cursora --ask` opened an empty bubble: the action ran before the bubble's page had subscribed to its events. The page now also fetches the current conversation once it is listening (APP-2.3).
