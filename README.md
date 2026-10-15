# Cursora

Select text in any app, press **Ctrl+G**, and ask an AI about it in a small chat bubble right at your cursor. Cursora works with local models (LM Studio, any OpenAI-compatible server, or a built-in engine for `.gguf` files) and with cloud API keys (Claude, GPT, Gemini, …). It has a personality you can customize or switch off.

Runs on Windows 10/11 and Linux (X11 fully, Wayland best-effort).

**Status:** pre-alpha. Every planned feature is built except voice, and CI passes on Windows and Ubuntu; the hands-on checks on every platform are still open. There's no release yet, so build it from source (below). The specs are in [specs/](specs/README.md), and known problems and untested items in [specs/issues.md](specs/issues.md).

## What it does
- **Ask about a selection:** text from browsers, editors, terminals, Office, PDF viewers, Electron and Java apps, or files and folders selected in File Explorer or on the Desktop. Your clipboard is restored afterwards, and terminals never get Ctrl+C. With nothing selected, the bubble opens as a plain chat.
- **The bubble** opens at the cursor, streams Markdown answers with highlighted code, and takes follow-ups. Press the hotkey again to close it. **Expand** opens the conversation in a larger window.
- **History:** conversations are saved locally (SQLite) for 90 days by default. You can search them, reopen them, or turn saving off.
- **Snip** (Ctrl+Shift+G): drag a box around anything on screen and ask about it. Shown only when the model can read images.
- **Skills:** type `/` for commands: `/summary`, `/explain`, `/simplify`, `/define`, `/translate <language>`, `/fix`, `/rewrite <tone>`, `/code`, `/bugs`. Make your own in Settings → Skills, with `/new-skill`, or with **Save as skill** on any question. Skills are Markdown files in the skills folder and reload as soon as they change.
- **Personality:** give your companion a name, gender, tone and style, or switch personality off for the plain model. Switch personas with `/persona <name>` or from the tray.
- **Models:** LM Studio is found automatically. Add any OpenAI-compatible server by URL, or pick a `.gguf` file for the built-in llama.cpp engine. Cloud presets: OpenAI, Anthropic, Google Gemini, OpenRouter, Groq, DeepSeek, Mistral, or a custom endpoint.
- **Privacy:**
  - API keys are stored only in the OS keychain (Windows Credential Manager, Linux Secret Service).
  - **Local-only mode** blocks every request to a cloud provider.
  - Before text goes to a cloud model, emails, phone, card and IBAN numbers, API keys, Aadhaar and PAN numbers are replaced with placeholders, then put back in the answer. "What will be sent" shows the exact outgoing text.
  - Password managers are blocked by default. Block any other app from the tray.
  - No telemetry, no analytics, no update checks.

## Command line
`cursora --ask`, `cursora --snip`, `cursora --settings` and `cursora --quit` act on the running Cursora, starting it first if needed.

## Windows
- **Apps running as administrator:** Cursora can't read them unless it runs as administrator too. Turn on Settings → General → **Run as administrator** (administrator accounts only). It needs Cursora installed in Program Files.

## Linux
- **X11** works as on Windows. The selected text is read without any simulated keys; only file managers get Ctrl+C, to copy the selected files.
- **The tray icon** goes through AppIndicator. Ubuntu shows it; on other GNOME systems, install the AppIndicator extension. Everything also works without it: the hotkey asks, and `cursora --settings` opens Settings.
- **Wayland:** apps can't listen for hotkeys there, so Settings → General shows the command to bind as a custom shortcut in GNOME or KDE. The selection is read with `wl-paste` (package `wl-clipboard`). Cursora can't tell which app is in front, so the app blocklist doesn't apply, and selected files can't be read. To snip, copy a region with the desktop's screenshot tool, then run `cursora --snip`.
- **API keys** need a Secret Service (GNOME Keyring or KWallet). Local models work without one.

## Develop (Windows)
Prerequisites:
- Rust (rustup, stable-msvc)
- Build Tools for Visual Studio 2022, with the "Desktop development with C++" workload. This is the compiler only, not the IDE.
- Node.js 22

```
npm install
npm run tauri dev
```

To try the built-in engine in dev, run `npm run fetch-llama` first. It downloads llama-server into `src-tauri/llama/`.

Tests:
```
npm run trace        # every (auto) spec criterion has a test
npm test             # JS tests
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:linux   # the same Rust tests, built for Linux in WSL (Ubuntu-24.04)
```

To build on Ubuntu, install Rust, Node.js 22 and:
```
sudo apt install libwebkit2gtk-4.1-dev build-essential libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Cursora is built spec-first: read the [constitution](specs/00-constitution.md) before changing anything, put behavior changes into the spec first, and name tests after the criterion they cover (`SEL-3.1`).

**Releases:** pushing a `v*` tag makes CI build the installers (`.msi` and `.exe` for Windows, `.deb` and `.AppImage` for Linux, with the built-in engine) as a draft GitHub release.
