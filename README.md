# Cursora

Select text in any app, press **Ctrl+G**, and ask an AI about it in a small chat bubble right at your cursor. Cursora works with local models (LM Studio, any OpenAI-compatible server, or a built-in engine for `.gguf` files) and with cloud API keys (Claude, GPT, Gemini, …). It has a personality you can customize or switch off.

**Status:** pre-alpha. The specs are in [specs/](specs/README.md).

## Linux
- **X11** works as on Windows. The selected text is read without any simulated keys; only file managers get Ctrl+C, to copy the selected files.
- **The tray icon** goes through AppIndicator. Ubuntu shows it; on other GNOME systems, install the AppIndicator extension. Everything also works without it: the hotkey asks, and `cursora --settings` opens Settings.
- **Wayland:** apps can't listen for hotkeys there, so Settings → General shows the command to bind as a custom shortcut in GNOME or KDE. The selection is read with `wl-paste` (package `wl-clipboard`). Cursora can't tell which app is in front, so the app blocklist doesn't apply, and selected files can't be read.

## Develop (Windows)
Prerequisites:
- Rust (rustup, stable-msvc)
- Build Tools for Visual Studio 2022, with the "Desktop development with C++" workload. This is the compiler only, not the IDE.
- Node.js 22

```
npm install
npm run tauri dev
```
