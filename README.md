<div align="center">

<img src="src-tauri/icons/icon.svg" width="120" alt="Cursora logo">

# Cursora

### Your AI companion, right at your cursor.

**Select anything · press <kbd>Ctrl</kbd> + <kbd>G</kbd> · ask.**<br>
In any app, with any model, and nothing leaves your computer unless you want it to.

[![Latest release](https://img.shields.io/github/v/release/priyansh1210/cursora-releases?label=release&color=15803d)](https://github.com/priyansh1210/cursora-releases/releases/latest)
[![CI](https://github.com/priyansh1210/cursora/actions/workflows/ci.yml/badge.svg)](https://github.com/priyansh1210/cursora/actions/workflows/ci.yml)
[![License: GPL v3](https://img.shields.io/badge/license-GPL--3.0-blue)](LICENSE)
![Windows 10 | 11](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D6)
![Linux X11 | Wayland](https://img.shields.io/badge/Linux-X11%20%7C%20Wayland-FCC624)
![Local or cloud models](https://img.shields.io/badge/models-local%20%2B%20cloud-8A2BE2)
![No telemetry](https://img.shields.io/badge/telemetry-none-2EA44F)
[![Stars](https://img.shields.io/github/stars/priyansh1210/cursora?style=social)](https://github.com/priyansh1210/cursora/stargazers)

[**Download**](https://github.com/priyansh1210/cursora-releases/releases/latest) · [Features](#-features) · [Privacy](#-private-by-design) · [User manual](user-manual.html) · [Contributing](CONTRIBUTING.md)

</div>

---

You're reading an error in your terminal, a paragraph in a PDF, a contract in your browser, or a form you have to fill, and you have a question. With Cursora you don't copy, paste or switch windows: **select it, press the hotkey, and a small chat bubble opens right where your mouse is**, with your selection already attached. Ask, read the answer, put it back into the app with one click, and press the hotkey again to be exactly where you were.

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

## ✨ Why Cursora

| | |
|---|---|
| ⚡ **No context switching** | The AI comes to your text, not the other way round. One hotkey, from any app. |
| 🌍 **Works everywhere** | Browsers, editors, terminals, Office, PDFs, Electron and Java apps, even files in File Explorer. |
| 🔒 **Private by design** | Personal details are hidden before anything reaches a cloud model, keys live in your OS keychain, and there is no telemetry at all. |
| 🏠 **Fully offline if you want** | One click downloads a free model that runs on your own computer, on your graphics card when you have one. No account, no internet needed. |
| 🧠 **Any model you like** | OpenAI, Anthropic, Gemini, OpenRouter, Groq, DeepSeek, Mistral, LM Studio, llama.cpp, Ollama and other OpenAI-compatible servers, or the built-in engine. |
| 🇮🇳 **Speaks your language** | Hindi, Hinglish, Tamil, Bengali and every other language: answers, one-click translation, voice input, Indian scripts and Urdu shown right. |
| 📝 **Fills forms for you** | Save your details once, and Cursora fills web forms and app forms for you. Your details never go to a model. |
| 💚 **Free and open source** | GPL-3.0, built in the open, with every behavior written down in a spec and checked by a test. |

## 🌟 Features

### 🎯 Ask about anything you can select
Text in any app, and even **files and folders** selected in File Explorer or on the Desktop. Nothing selected? The bubble opens as a plain chat, ready to write for you.

### 💬 A bubble, not another window
It opens at your cursor and streams the answer as it's written, in Markdown with highlighted code. Ask follow-up questions, switch the model for one question, or **Expand** the chat into a bigger window. Press the hotkey again and it's gone.

### ✍️ Put the answer straight back
**Replace** swaps the answer in for your selection, and **Insert** types it where your cursor was. One-click buttons suit the app you're in, so the common jobs take no typing at all:

| Where you are | One click |
|---|---|
| Documents, browsers and notes | **Fix** · **Rewrite** · **Shorter** · **Explain** · **Summarize** · **Translate** |
| Email and chat | **Reply** · **Fix** · **More polite** · **Shorter** · **Translate** · **Explain** |
| Terminals | **Explain error** · **Explain command** · **Explain** |
| Code editors | **Explain** · **Find bugs** · **Explain error** · **Summarize** |

### 📝 Fill forms in seconds
Write your details once in **Settings → My details**: name, date of birth, phone, address, college or company, links, and anything else. Then click into any form, press <kbd>Ctrl</kbd> + <kbd>G</kbd> and choose **Fill this form**.

- **Understands how forms ask.** "Name", "Applicant's name", "Contact No.", "DOB", "Pincode", "Please enter your email" and nearly 200 other common wordings, Hindi labels too, match the right detail instantly, without a model. Anything unusual goes to your AI model, which sees only the *names* of your details, never the values.
- **Fits each value to its box.** Your first name comes from your full name, a date is written in the box's own format (DD/MM/YYYY, YYYY-MM-DD, …), and a question gets the option your value names ("F" picks **Female**).
- **Works with small local models.** On real form labels, the built-in 1.5B model went from 7 wrong answers out of 48 to none.
- **You stay in charge.** Cursora shows its plan with a tick box per field, then types each value letter by letter in a green frame. Press <kbd>Esc</kbd> to stop at any moment. Passwords, one-time codes, card numbers, captchas and "I agree" boxes are never touched.

<sub>Text boxes are filled in v0.3.1; drop-down lists, radio buttons, check boxes and date boxes come with the next release. Windows for now.</sub>

### 🔎 Point at a word to define it
Press <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>G</kbd> while pointing at any word, in any app, a PDF or even a picture: its meaning appears right there, without selecting anything.

### ✂️ Snip anything on screen
Press <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>G</kbd> and drag a box around an image, a chart, a video frame or a scanned page, then ask about it. A model that can see images gets the picture; any other model gets the text in it, read on your own computer.

### 🎙️ Talk instead of typing
Click the microphone and say your question. A speech model on your computer turns it into text, in English, Hindi and the other languages it knows, and the recording never leaves your PC.

### 🌐 Answers from the web, with sources
`/web <question>` searches the web (with your own free Tavily key) and lists the sources as links under the answer. It works with local models too.

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
| `/reply <what to say>` | Draft a reply to the selected message |
| `/error` | Explain an error message and how to fix it |
| `/web <question>` | Search the web and answer with sources |
| `/fill` | Fill the form you're in from My details |

**Make your own** (`/eli5`, `/to-sql`, …) in Settings → Skills, with `/new-skill`, or with **Save as skill** on any question you asked. A skill is a plain Markdown file, ready the moment you save it.

### 🗣️ Your language
Set **My language** (Hindi, Hinglish, Tamil, Bengali and the other Indian languages are suggested; any language works). **Translate** puts text into it with one click, or back into English, and Cursora can answer in it every time. Indian scripts and Urdu, right to left, show correctly everywhere. Tip: a cloud model such as Gemini handles these languages best; the small free downloads are better in English.

### 🎭 A companion with a personality
Pick a persona (Cursora, Ada, Arjun or Sage) or create your own: name, tone, how much it says, emoji, reply language and your level of expertise. Add a few lines **About you** (who you are, what you work on, how you like things explained), and every answer takes them into account. Prefer the plain model? Turn personality off.

### 🧠 Your model, your choice

| 🏠 On your computer | ☁️ In the cloud, with your API key |
|---|---|
| **One-click download:** a free model in three sizes (Qwen 1.5B, 4B and 7B), the right one for your PC marked, checked before use | **OpenAI**, **Anthropic**, **Google Gemini** |
| **Built-in engine:** answers on your graphics card (NVIDIA, AMD or Intel Arc) when you have one, or pick any `.gguf` file | **OpenRouter**, **Groq**, **DeepSeek**, **Mistral** |
| **LM Studio**, found automatically | **Any OpenAI-compatible endpoint** |
| **Any OpenAI-compatible server:** llama.cpp, vLLM, Jan, LocalAI, KoboldCpp, … | |

**Smart routing:** set a small **Quick model** on your computer, and routine jobs go to it while your main model handles the rest. When you're offline, it answers instead. A model on your computer needs no internet at all: setup, answers, Settings and History all work offline.

### 📌 Keep what matters
**Pin** keeps an answer on screen in a small note, **Save** writes it to a Markdown file (Obsidian-ready), and **History** keeps your conversations on your computer for 90 days by default, searchable and easy to reopen, or not at all if you switch it off.

### 🤫 Stays out of your way
Turn Cursora off in any app with one click from the tray, so <kbd>Ctrl</kbd> + <kbd>G</kbd> stays "Go to line" in VS Code. All hotkeys can be changed.

## 🔒 Private by design

- 🔑 **API keys stay in your OS keychain** (Windows Credential Manager or Linux Secret Service), never in a file or a log.
- 🏠 **Local-only mode:** one switch, and nothing leaves your computer.
- 🙈 **Redaction before the cloud:** emails, phone numbers, card numbers, IBANs, API keys, and Aadhaar and PAN numbers become placeholders, and are put back in the answer on your side. **What will be sent** shows the exact outgoing text.
- 📝 **Your form details never reach a model:** only their names do, and Cursora types the values itself.
- 🚫 **Blocked apps:** Cursora never reads from password managers, and you can block any other app from the tray in one click.
- 📋 **Your clipboard is safe:** whatever you copied before is restored, and terminals never get <kbd>Ctrl</kbd> + <kbd>C</kbd>, so your running process keeps running.
- 📡 **No telemetry, no analytics.** Cursora talks only to the models you set up, plus a once-a-day check for a new version (it sends nothing about you, and one tick turns it off) and the downloads you start yourself.
- 👀 **Open source:** don't take our word for it. Every line is here to read.

## 🏗️ Built to be trusted

Cursora is built **spec-first**: every behavior is written down as a numbered requirement before any code is written, and every automatic requirement has a test named after it.

- **27 specs** with nearly **200 automatically checked requirements**, traced to their tests by a script in CI ([`npm run trace`](scripts/trace.mjs)).
- **More than 250 Rust tests** and the JavaScript tests run on **Windows and Ubuntu** for every change.
- **Signed automatic updates**, and an app written in Rust with Tauri: small, fast to open, and light on memory.
- Every known gap is written down in the open, in [specs/issues.md](specs/issues.md) and the [issue tracker](https://github.com/priyansh1210/cursora/issues).

## 🖥️ Platforms

| | Windows 10/11 | Linux X11 | Linux Wayland |
|---|:---:|:---:|:---:|
| Hotkey | ✅ | ✅ | via a desktop shortcut¹ |
| Selected text | ✅ | ✅ | ✅ (`wl-clipboard`) |
| Selected files | ✅ | ✅ | ❌ |
| Bubble at the cursor | ✅ | ✅ | centered on screen |
| Snip | ✅ | ✅ | from a copied screenshot² |
| App blocklist | ✅ | ✅ | ❌ |
| Fill forms | ✅ | not yet | not yet |

¹ Apps can't listen for hotkeys on Wayland, so Settings → General shows the command (`cursora --ask`) to bind in GNOME or KDE.
² Copy a region with the desktop's screenshot tool, then run `cursora --snip`.

## 🚀 Get started

### Download
Get the latest release from **[cursora-releases](https://github.com/priyansh1210/cursora-releases/releases/latest)**:

| System | File |
|---|---|
| Windows 10/11 | `Cursora_<version>_x64_en-US.msi` |
| Ubuntu, Debian | `Cursora_<version>_amd64.deb` |
| Fedora, openSUSE | `Cursora-<version>-1.x86_64.rpm` |
| Any other Linux | `Cursora_<version>_amd64.AppImage` |

A short setup walks you through it: choose an AI source (one click downloads a free one, and it checks that the model answers), choose a persona, choose your hotkeys, then try it on some sample text. Cursora then keeps itself up to date: it checks once a day (one tick in Settings turns that off), and every update is signed.

- The installers aren't code-signed yet, so Windows may say it doesn't recognize the app: choose **More info**, then **Run anyway**.
- The built-in engine needs Microsoft's Visual C++ runtime, which most PCs already have. If the engine doesn't start, install the [Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe).

**New to Cursora?** Read the [user manual](user-manual.html): download it and open it in your browser.

> [!NOTE]
> **Early access.** Cursora works today on Windows, and CI builds and tests it on Windows and Ubuntu for every change. Hands-on checks on real Linux desktops are still open: if you have one, [you can help](https://github.com/priyansh1210/cursora/issues?q=is%3Aopen+label%3A%22needs+testing%22).

### Build from source
On Windows you need:
- Rust (rustup, stable-msvc)
- Build Tools for Visual Studio 2022, with the "Desktop development with C++" workload (the compiler only, not the IDE)
- Node.js 22
- CMake and LLVM's libclang, which build the speech engine for voice input. The Build Tools include CMake: if it isn't on your PATH, set `CMAKE` to its `cmake.exe`. For libclang, install LLVM, or `pip install libclang` and set `LIBCLANG_PATH` to its `clang/native` folder. A [user-level Cargo config](https://doc.rust-lang.org/cargo/reference/config.html) (`%USERPROFILE%\.cargo\config.toml`, under `[env]`) keeps both settings in one place.

```
npm install
npm run tauri dev
```

To try the built-in engine, run `npm run fetch-llama` first. It downloads llama-server into `src-tauri/llama/`.

<details>
<summary><b>💡 Tips</b></summary>

- **Apps running as administrator (Windows):** Cursora can't read them unless it runs as administrator too. Turn on Settings → General → **Run as administrator** (administrator accounts only; Cursora must be installed in Program Files).
- **Command line:** `cursora --ask`, `--snip`, `--settings` and `--quit` act on the running Cursora, and start it first if needed.
- **Ctrl+G** replaces "Go to line" in VS Code and Notepad, and "Find next" in browsers. Turn Cursora off in those apps from the tray, or pick another hotkey in Settings → General.

</details>

<details>
<summary><b>🐧 Linux notes</b></summary>

- **X11** works as on Windows. The selected text is read without any simulated keys; only file managers get Ctrl+C, to copy the selected files.
- **The tray icon** goes through AppIndicator. Ubuntu shows it; on other GNOME systems, install the AppIndicator extension. Everything also works without it: the hotkey asks, and `cursora --settings` opens Settings.
- **Wayland:** the selection is read with `wl-paste` (package `wl-clipboard`). Cursora can't tell which app is in front, so the app blocklist doesn't apply, and selected files can't be read.
- **Snip with a model that can't see images** reads the text with Tesseract: install `tesseract-ocr`.
- **API keys** need a Secret Service (GNOME Keyring or KWallet). Models on your computer work without one.
- **To build on Ubuntu**, install Rust, Node.js 22 and:
  ```
  sudo apt install libwebkit2gtk-4.1-dev build-essential libssl-dev libayatana-appindicator3-dev librsvg2-dev cmake libasound2-dev
  ```
  and set `WHISPER_DONT_GENERATE_BINDINGS=1`, so the speech engine uses its ready-made Linux bindings instead of needing libclang.

</details>

## 🤝 Contributing

Cursora is open source, and help is very welcome:
- **Try it on a setup we don't have** (a real Linux desktop, two monitors, a screen reader, an app we haven't tested) and tell us what happened. The [`needs testing`](https://github.com/priyansh1210/cursora/issues?q=is%3Aopen+label%3A%22needs+testing%22) issues list them, and there are [`good first issue`](https://github.com/priyansh1210/cursora/labels/good%20first%20issue)s too.
- **Report a bug or share an idea** in [Issues](https://github.com/priyansh1210/cursora/issues) or [Discussions](https://github.com/priyansh1210/cursora/discussions).
- **Send code:** start with [CONTRIBUTING.md](CONTRIBUTING.md). Behavior changes go into their [spec](specs/README.md) first.
- **Like it?** A ⭐ helps other people find Cursora.

<details>
<summary><b>📦 Releases</b></summary>

Pushing a `v*` tag makes CI build the installers (`.msi` for Windows; `.deb`, `.rpm` and `.AppImage` for Linux, all with the built-in engine) and the signed update files, as a draft release in the public [`cursora-releases`](https://github.com/priyansh1210/cursora-releases) repo. Publishing the draft by hand makes it the update every installed Cursora sees. A tag with a `-` (such as `v0.2.0-rc.1`) makes a pre-release.

</details>

## 📄 License

Cursora is free software under the [GNU General Public License v3.0 or later](LICENSE). You may use, study, share and change it; copies you share, changed or not, must stay under the same license. To report a security problem privately, see [SECURITY.md](SECURITY.md).

<div align="center">
<br>
<sub>Built with Tauri v2, Rust and React.</sub>
</div>
