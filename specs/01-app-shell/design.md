# 01: App shell (design)

Status: **Approved** (2026-10-11). Implements [requirements.md](requirements.md). **Amended 2026-10-14, approved:** Run as administrator (APP-3.3–3.7).

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
| `src-tauri/src/admin.rs` | Run as administrator (APP-3.3–3.7): `startup(on, elevated) -> Startup`, `task_xml(exe, user)`, `starts(xml, exe)` and `inside(exe, dirs)` are OS-independent and unit-tested; running the task, the UAC prompt and the `--after` wait are `#[cfg(windows)]`. |

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
- **Run as administrator (APP-3.3–3.7, Windows):** a `run_as_admin: bool` setting, default false.
  - **Why a task:** Windows doesn't let a normal app read or send keys to an admin app (SEL-6.1). It silently skips Run entries that need admin rights at login, and a UAC prompt at every start was rejected by the user. A Task Scheduler task registered with the highest privileges starts Cursora as administrator with no prompt. There is one task per Windows account, named `Cursora (administrator, <user name>)`, so two administrators on one PC don't overwrite each other's.
  - **The task (APP-3.6)** is registered from XML, in memory, through the Task Scheduler COM API (`RegisterTask`): no trigger, the current user, `InteractiveToken`, `HighestAvailable`. Task Scheduler's own defaults would break Cursora, so the XML overrides them: no 72-hour time limit (`PT0S`), not "AC power only", priority 5 instead of 7 (below normal), and `MultipleInstancesPolicy` Parallel so a relaunch can reach the running copy.
  - **At start (APP-3.7),** first thing in `run()`, before the single-instance plugin (a normal-rights copy can't open an admin copy's lock, so it would run alongside it):
    - on, not admin: run the task (`GetTask(…).Run`) if its XML starts this exe (`starts`): Task Scheduler reports success even when the exe is gone, and a task left by a copy installed elsewhere would start that copy. If there is no such task, `ShellExecuteW("runas")`. When either starts a copy, exit at once. If both fail, run as a normal app and log why (APP-3.3).
    - on, admin: create or update the task, so it always points at the current exe.
    - off, admin: delete the task if it exists.
    - off, not admin: nothing.
    - Whether this copy is admin comes from `capture::is_elevated` on the current process: the one check SEL-6.1 already uses.
  - **Turning it on (APP-3.4),** from a normal copy: `ShellExecuteW("runas", exe, "--after=<pid> --settings")`. Cancelled → `set_settings` fails with "Windows didn't allow it." and the setting isn't saved. Yes → save the setting and `app.exit(0)`. The admin copy waits up to 10 s for that pid to end, then starts normally: "on, admin" creates the task, and `--settings` reopens Settings. `--after=` is read in `run()` and is not a `cli::Action`.
  - **Turning it off (APP-3.5):** that copy is the admin one, so it can delete the task. Settings shows "Cursora runs as a normal app from its next start."
  - **Administrator accounts only:** `app_info` gains `canRunAsAdmin`, true when the token's elevation type is Limited (an admin with UAC on) or Full. A standard account's UAC prompt asks for another user's password, and Cursora would then run as that user, with that user's settings.
  - **Only from Program Files (APP-3.8):**
    - `in_program_files()` checks that the exe is inside `FOLDERID_ProgramFiles` or `FOLDERID_ProgramFilesX86`. They are read with `SHGetKnownFolderPath`, which takes them from HKLM. The `%ProgramFiles%` variable isn't used: a user can set it.
    - Windows uses the same rule for the apps it trusts with UI access.
    - The engine is bundled next to the exe, so the check covers it. A task left after an uninstall points into Program Files, where no normal program can put a file.
    - `app_info` gains `inProgramFiles`. Settings greys the box out and says why, unless the box is ticked, so it can still be turned off.
    - Outside Program Files, a start counts the setting as off. A normal copy runs as a normal app and logs why; an admin copy deletes the task.
  - **WebView2:** a normal and an admin WebView2 can't share one data folder while both run, and the old copy's WebView2 processes can outlive its pid. Checked 2026-10-15: they were gone 0.4 s after it, and the admin copy started fine (one unexplained failure before that is issue 41). If the admin webview ever fails, the admin copy gets its own folder (`EBWebView-admin`); nothing is kept in webview storage.
  - **Limit:** an action given to a normal-rights launch (`cursora --ask`, `--snip`, `--settings`) isn't passed on: the task takes no arguments, so the admin copy starts or opens Settings instead. Hotkeys are unaffected. Marked `// shortcut:` in the code.
- **Logging (APP-7.1):** tauri-plugin-log writes to `LogDir`, with `max_file_size(1_000_000)` and `RotationStrategy::KeepSome(5)`.
- **Setup (APP-6):** this spec ships the **Hotkeys** and **Start with system** steps and the "Re-run setup" button (APP-6.3). Specs 04, 05 and 03 add the **AI source**, **Personality** and **Try it** steps, so APP-6.1 is verified at the end of v1.

## Decisions
- **A plain serde JSON file instead of tauri-plugin-store.** serde is already a dependency. We need atomic writes, a backup on corruption and a schema version, and owning the file makes those a few lines each.
- **Settings and Setup windows are created on demand** instead of being pre-created. They're rarely opened, and this saves RAM. Only the bubble (spec 03) is pre-created, because of its speed budget.
- **No UI framework or router.** It's plain React with one CSS file and the window label as the "route".
- **2026-10-14, Run as administrator through a Task Scheduler task**, so Windows asks once, when the setting is turned on (the user's choice). Rejected: a UAC prompt at every start and login (the other option offered); a "requireAdministrator" manifest (every user would get a prompt, and Windows skips such apps in the Run key); a restart button in the admin-app bubble (the user chose a message naming the setting).
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
| APP-3.6 | Rust: `task_xml` has no trigger, `HighestAvailable`, `PT0S`, both battery flags false, priority 5, Parallel, and the exe path XML-escaped |
| APP-3.7 | Rust: `startup` for all four combinations of setting and elevation |
| APP-3.3 | Rust: `starts` accepts the task's own exe and refuses a copy installed elsewhere and part of the path (the rest is the manual checklist) |
| APP-3.8 | Rust: `inside` accepts exes in Program Files in any letter case. It refuses Program Files (x86) when given Program Files only, a per-user install, a dev build and an empty folder name. |
| APP-7.2 | Node: scan `src-tauri/src/**/*.rs` and fail on log macros that use user-content identifiers (`text`, `selection`, `content`, `question`, `answer`, `api_key`, …) |
| APP-8.1 | Done: `scripts/trace.test.mjs` |
| Others | Manual checklist in `tasks.md` |

## Changes during implementation
- **2026-10-11:** tauri-plugin-autostart was replaced by direct `auto-launch` calls. The plugin hard-codes the "Dynamic" mode, which wrote the machine-wide HKLM Run key on this PC, while APP-3.1 says per-user. `auto-launch` was already a transitive dependency.
- **2026-10-11:** autostart is also synced at startup. Otherwise closing Setup early left "Start with system ✓" in Settings without an OS entry (`/ponytail-review` finding 1).
- **2026-10-11:** APP-2.4 was added: relaunching Cursora with no arguments opens Settings (review finding 3).
- **2026-10-13, manual checks:** log rotation keeps 4 old files (`KeepSome(4)`), since the plugin counts old files only; with the current one that makes the 5 of APP-7.1. It kept 6.
- **2026-10-13:** with Cursora not running, `cursora --ask` opened an empty bubble: the action ran before the bubble's page had subscribed to its events. The page now also fetches the current conversation once it is listening (APP-2.3).
- **2026-10-14, Run as administrator:** the task goes through the Task Scheduler COM API (`ITaskFolder::RegisterTask` with the XML in memory, `GetTask(…).Run`, `DeleteTask`) instead of `schtasks /xml <file>`. Any normal program could swap a temp file before the admin copy reads it, and put its own command into an admin task. COM runs on a thread of its own, so it never meets the main thread's own COM setup. The UAC prompt is `ShellExecuteW("runas")` on the command's own thread: the Settings window is frozen only while Windows' secure desktop covers it. The wait argument is `--after=<pid>`, so `cli::parse` skips it as one piece, and it comes with `--settings`, so the admin copy reopens Settings. The task is only re-registered when the setting changes, or at an admin start.
- **2026-10-15, review of tasks 10–12:** 3 findings, all fixed. (1) A task pointing at a copy installed elsewhere (the .exe and .msi installers use different folders, and settings survive a reinstall) made every start quit with nothing running, since Task Scheduler reports success even when the exe is gone: the task now runs only if its XML starts this exe, otherwise Windows asks. (2) Two administrator accounts on one PC overwrote each other's task: the user name is now part of its name. (3) Issue 39 now notes that uninstalling a per-user copy leaves the task behind.
- **2026-10-15, issue 39, APP-3.8 added:**
  - Run as administrator is now only available from Program Files. In a folder the user can write to, any program could replace `cursora.exe` or `llama-server.exe` and get admin rights from the task with no prompt, even after an uninstall.
  - The user chose this over also making the NSIS installer per-machine: NSIS stays per-user, and its users don't get the setting.
  - Rejected: checking the folder's access rights. The parent folders would need the same check, since a writable parent lets a program rename the folder and put its own in its place.
  - Testing Run as administrator now needs the .msi installed in Program Files.
