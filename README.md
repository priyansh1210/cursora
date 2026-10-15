<div align="center">

<img src="src-tauri/icons/icon.svg" width="112" alt="Cursora logo">

# Cursora

### Your AI companion, right at your cursor.

Select anything · press <kbd>Ctrl</kbd> + <kbd>G</kbd> · ask

![Windows 10 | 11](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D6)
![Linux X11 | Wayland](https://img.shields.io/badge/Linux-X11%20%7C%20Wayland-FCC624)
![Local or cloud models](https://img.shields.io/badge/models-local%20%2B%20cloud-8A2BE2)
![No telemetry](https://img.shields.io/badge/telemetry-none-2EA44F)
![Pre-alpha](https://img.shields.io/badge/status-pre--alpha-orange)

</div>

---

You're reading an error in your terminal, a paragraph in a PDF, or a contract in your browser, and you have a question. With Cursora you don't copy, paste or switch windows: **select it, press the hotkey, and a small chat bubble opens right where your mouse is**, with your selection already attached.

```text
  PS> cargo build
  error[E0382]: borrow of moved value: `config`   ← select it, press Ctrl+G
    ┌────────────────────────────────────────────────┐
    │ Ada · qwen2.5-7b-instruct · Local              │
    ├────────────────────────────────────────────────┤
    │ Windows Terminal · 45 characters               │
    │ error[E0382]: borrow of moved value: `config`  │
    ├────────────────────────────────────────────────┤
    │ You: why does this happen?                     │
    │                                                │
    │ Ada: `config` was moved into the thread on     │
    │ line 12, so it can't be used after that.       │
    │ Clone it before the move, or pass a reference. │
    ├────────────────────────────────────────────────┤
    │ Ask about the selection                        │
    └────────────────────────────────────────────────┘
```

Use a model on your own computer or your own cloud API key. Give it a name and a personality, or keep it plain.

> [!NOTE]
> **Pre-alpha.** Everything is built except voice, and CI passes on Windows and Ubuntu, but the hands-on checks on every platform are still open ([known issues](specs/issues.md)). There's no release yet: [build it from source](#-get-started).

## 🌟 Features

### 🎯 Works with anything you can select
Browsers, editors, terminals, Office, PDF viewers, Electron and Java apps, and even **files and folders** in File Explorer or on the Desktop. Nothing selected? The bubble opens as a plain chat.

### 💬 A bubble, not another window
It opens at your cursor, streams the answer as it's written (Markdown, with highlighted code), and takes follow-up questions. **Expand** moves the chat to a bigger window. Press the hotkey again to close it, and you're back where you were.

### ✂️ Snip anything on screen
Press <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>G</kbd> and drag a box around an image, a video frame, or a scanned page, then ask about it. Snip appears whenever your model can read images.

### ⚡ Skills: one-word commands
Type `/` in the bubble:

| Command | What it does |
|---|---|
| `/summary` | Summarize the selection in a few bullet points |
| `/explain` | Explain what the selection means |
| `/simplify` | Rewrite it in simpler words |
| `/define` | Define a word or phrase |
| `/translate <language>` | Translate it |
| `/fix` | Fix grammar, spelling and punctuation |
| `/rewrite <tone>` | Rewrite it in another tone |
| `/code` | Explain what the code does |
| `/bugs` | Find bugs in the code |

**Make your own** (`/eli5`, `/to-sql`, …) in Settings → Skills, with `/new-skill`, or with **Save as skill** on any question you asked. A skill is a plain Markdown file, and it's ready the moment you save it.

### 🎭 A companion with a personality
Pick a persona (Cursora, Ada, Arjun or Sage) or create your own: name, gender, tone, how much it says, emoji, reply language, and your level of expertise. Switch with `/persona <name>` or from the tray, or turn personality off for the plain model.

### 🧠 Your model, your choice

| 🏠 On your computer | ☁️ In the cloud, with your API key |
|---|---|
| **LM Studio**, found automatically | **OpenAI**, **Anthropic**, **Google Gemini** |
| **Any OpenAI-compatible server:** llama.cpp, vLLM, Jan, LocalAI, KoboldCpp, … | **OpenRouter**, **Groq**, **DeepSeek**, **Mistral** |
| **Built-in engine:** pick a `.gguf` file. Nothing else to install. | **Any OpenAI-compatible endpoint** |

A model on your computer needs no internet at all: setup, answers, Settings and History all work offline.

### 📚 History
Conversations are saved on your computer for 90 days by default. You can search and reopen them, delete them, or switch saving off.

## 🔒 Private by design

- 🔑 **API keys stay in your OS keychain** (Windows Credential Manager or Linux Secret Service), never in a file or a log.
- 🏠 **Local-only mode:** one switch, and nothing leaves your computer.
- 🙈 **Redaction before the cloud:** emails, phone numbers, card numbers, IBANs, API keys, and Aadhaar and PAN numbers become placeholders, and are put back in the answer on your side. **What will be sent** shows the exact outgoing text.
- 🚫 **Blocked apps:** Cursora never reads from password managers, and you can block any other app from the tray in one click.
- 📋 **Your clipboard is safe:** whatever you copied before is restored, and terminals never get <kbd>Ctrl</kbd> + <kbd>C</kbd>, so your running process keeps running.
- 📡 **No telemetry, no analytics, no update checks.** Cursora talks only to the models you set up.

## 🖥️ Platforms

| | Windows 10/11 | Linux X11 | Linux Wayland |
|---|:---:|:---:|:---:|
| Hotkey | ✅ | ✅ | via a desktop shortcut¹ |
| Selected text | ✅ | ✅ | ✅ (`wl-clipboard`) |
| Selected files | ✅ | ✅ | ❌ |
| Bubble at the cursor | ✅ | ✅ | centered on screen |
| Snip | ✅ | ✅ | from a copied screenshot² |
| App blocklist | ✅ | ✅ | ❌ |

¹ Apps can't listen for hotkeys on Wayland, so Settings → General shows the command (`cursora --ask`) to bind in GNOME or KDE.
² Copy a region with the desktop's screenshot tool, then run `cursora --snip`.

## 🚀 Get started

There's no installer yet, so build Cursora from source. On Windows you need:
- Rust (rustup, stable-msvc)
- Build Tools for Visual Studio 2022, with the "Desktop development with C++" workload (the compiler only, not the IDE)
- Node.js 22

```
npm install
npm run tauri dev
```

A short setup walks you through it: choose an AI source (it checks that the model answers), choose a persona, choose your hotkeys, then try it on some sample text.

To try the built-in engine, run `npm run fetch-llama` first. It downloads llama-server into `src-tauri/llama/`.

<details>
<summary><b>💡 Tips</b></summary>

- **Apps running as administrator (Windows):** Cursora can't read them unless it runs as administrator too. Turn on Settings → General → **Run as administrator** (administrator accounts only; Cursora must be installed in Program Files).
- **Command line:** `cursora --ask`, `--snip`, `--settings` and `--quit` act on the running Cursora, and start it first if needed.
- **Ctrl+G** replaces "Go to line" in VS Code and Notepad, and "Find next" in browsers. You can pick another hotkey in Settings → General.

</details>

<details>
<summary><b>🐧 Linux notes</b></summary>

- **X11** works as on Windows. The selected text is read without any simulated keys; only file managers get Ctrl+C, to copy the selected files.
- **The tray icon** goes through AppIndicator. Ubuntu shows it; on other GNOME systems, install the AppIndicator extension. Everything also works without it: the hotkey asks, and `cursora --settings` opens Settings.
- **Wayland:** the selection is read with `wl-paste` (package `wl-clipboard`). Cursora can't tell which app is in front, so the app blocklist doesn't apply, and selected files can't be read.
- **API keys** need a Secret Service (GNOME Keyring or KWallet). Models on your computer work without one.
- **To build on Ubuntu**, install Rust, Node.js 22 and:
  ```
  sudo apt install libwebkit2gtk-4.1-dev build-essential libssl-dev libayatana-appindicator3-dev librsvg2-dev
  ```

</details>

<details>
<summary><b>🛠️ Contributing</b></summary>

Cursora is built **spec-first**: read the [constitution](specs/00-constitution.md) and the [specs](specs/README.md) before changing anything. Behavior changes go into the spec first, and every test is named after the criterion it covers (`SEL-3.1`).

```
npm run trace        # every (auto) spec criterion has a test
npm test             # JS tests
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:linux   # the same Rust tests, built for Linux in WSL (Ubuntu-24.04)
```

**Releases:** pushing a `v*` tag makes CI build the installers (`.msi` and `.exe` for Windows, `.deb` and `.AppImage` for Linux, with the built-in engine) as a draft GitHub release.

</details>

<div align="center">
<sub>Built with Tauri v2, Rust and React.</sub>
</div>
