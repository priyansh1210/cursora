# 01: App shell (APP)

## Scope
- **In:**
  - App lifecycle, tray, single instance, CLI arguments, autostart
  - Settings persistence and the Settings window frame
  - First-run onboarding, logging, CI and the traceability checker
- **Out:** the *contents* of feature settings tabs. Each feature spec owns its own tab.

## Requirements

### APP-1 Background app with tray
User story: As a user, I want Cursora to run quietly in the background, so that it is ready whenever I press the hotkey.
- APP-1.1 (manual) WHEN Cursora starts THE SYSTEM SHALL run without a main window and SHALL show a tray icon.
- APP-1.2 (manual) WHEN the user opens the tray menu THE SYSTEM SHALL offer:
  - Open Settings
  - Persona ▸ (list)
  - Personality on/off
  - Local-only on/off
  - Pause Cursora
  - Stop the model (when a built-in model is set up, LLM-3.5)
  - Quit
- APP-1.3 (manual) WHILE Cursora is paused THE SYSTEM SHALL unregister all global hotkeys, SHALL ignore `cursora --ask`, and SHALL show a distinct "paused" tray icon. (Amended 2026-10-13: on Wayland the desktop shortcut is the hotkey, and Cursora can't unregister it.)
- APP-1.4 (manual) WHEN the user chooses Quit THE SYSTEM SHALL unregister hotkeys, stop all child processes (built-in engine, speech), and exit.

### APP-2 Single instance and CLI
User story: As a user (especially on Linux Wayland), I want to trigger Cursora from a command, so that I can bind it to a system shortcut.
- APP-2.1 (auto) THE SYSTEM SHALL parse the command-line arguments:
  - `--ask`: start the capture flow
  - `--snip`: start a snip
  - `--settings`: open Settings
  - `--quit`: exit

  Unknown arguments SHALL be ignored, and a warning logged.
- APP-2.2 (manual) WHEN a second instance is launched THE SYSTEM SHALL forward its arguments to the running instance and exit. The running instance SHALL perform the action.
- APP-2.3 (manual) WHEN `cursora --ask` is run and Cursora is not running THE SYSTEM SHALL start it and then perform the action.
- APP-2.4 (manual) WHEN Cursora is launched again without arguments while it is running (e.g. from the Start menu) THE SYSTEM SHALL open Settings, so the launch visibly does something.

### APP-3 Autostart
- APP-3.1 (manual) WHERE "Start with system" is enabled THE SYSTEM SHALL launch at user login:
  - Windows: the per-user Run key
  - Linux: an XDG autostart `.desktop` file
- APP-3.2 (manual) WHEN onboarding completes THE SYSTEM SHALL have asked the user whether to start with the system. The default answer is yes.

### APP-4 Settings persistence
- APP-4.1 (auto) WHEN a setting changes THE SYSTEM SHALL persist it to the per-user config directory and apply it without restart.
- APP-4.2 (auto) IF the settings file is missing THE SYSTEM SHALL use defaults. IF it is unreadable or invalid THE SYSTEM SHALL:
  - rename it to `settings.bak-<timestamp>.json`,
  - load the defaults, and
  - tell the user once.
- APP-4.3 (auto) THE settings file SHALL carry a `schemaVersion`, and older versions SHALL be migrated forward on load without losing user values.
- APP-4.4 (auto) THE serialized settings SHALL NOT contain any secret (API keys, tokens). See LLM-6.

### APP-5 Settings window
- APP-5.1 (manual) THE Settings window SHALL have these tabs: General · Models · Personality · Skills · Voice · Privacy · History · About.
- APP-5.2 (manual) THE General tab SHALL contain:
  - hotkeys (ask, snip, push-to-talk). (Amended 2026-10-14: snip and push-to-talk appear once spec 10 and spec 09 are built; until then they would do nothing, issue 37.)
  - Start with system
  - theme (system / light / dark)
  - Pause
- APP-5.3 (manual) THE About tab SHALL show:
  - version
  - OS and session type (Windows / X11 / Wayland)
  - config and log folder paths, with "Open folder" buttons

### APP-6 First-run onboarding
User story: As a new user, I want a short setup, so that my first Ctrl+G just works.
- APP-6.1 (manual) WHEN Cursora runs for the first time THE SYSTEM SHALL show onboarding with three steps:
  1. Choose an AI source: detected local runtimes, the built-in engine (pick a `.gguf`), or a cloud API key. Setup SHALL NOT continue past this step until the chosen model passes Test connection (LLM-7.1). For the built-in engine, the user also picks how long the model stays loaded (LLM-3.5), with "Always on" preselected.
  2. Choose a persona, or turn personality off.
  3. Choose the hotkeys: ask (default Ctrl+G), snip (default Ctrl+Shift+G), and push-to-talk (optional, unset by default). (Amended 2026-10-14: snip and push-to-talk appear once spec 10 and spec 09 are built, as in APP-5.2.)
  4. A "try it" page with sample text and the hotkey.

  In step 1, adding a LAN provider asks whether to trust that host (LLM-8.3).
- APP-6.2 (manual) WHEN the hotkey is pressed and no model works THE bubble SHALL show "No AI model is available" (LLM-9.1).
- APP-6.3 (manual) Every choice made in setup SHALL be editable later in Settings, and setup SHALL be re-runnable from Settings → General.

### APP-7 Logging
- APP-7.1 (manual) THE SYSTEM SHALL write rotating local logs: at most 5 files of 1 MB each.
- APP-7.2 (auto) Log entries SHALL NOT contain API keys or user content (selected text, file contents, questions, answers, images). Only metadata such as lengths, durations, error kinds and app names SHALL be logged.

### APP-8 Build and traceability
- APP-8.1 (auto) `scripts/trace.mjs` SHALL exit non-zero when:
  - any `(auto)` criterion of a spec whose status is "In progress" or "Done" has no test containing its ID (not-yet-started specs are reported as pending), or
  - any test references an ID not defined in the specs.

  It SHALL print each offending ID.
- APP-8.2 (manual) WHEN a commit is pushed THE CI SHALL:
  - build on Windows and Ubuntu
  - run `trace`, `cargo test`, `vitest`
  - produce installers on tags (`.msi`/`.exe`, `.deb`/`.AppImage`)

## Open questions
- _(none)_
