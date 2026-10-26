---
name: dev-machines
description: "Two dev PCs share this memory through git: the laptop (PRIYANSH) does the Windows work; the stronger PC, reached over Chrome Remote Desktop, does the Linux work in WSL"
metadata:
  type: project
---

Since 2026-10-25 Cursora is developed on two PCs, each with its own Claude Code, sharing this memory through the repo's `memory/` folder (linked with `scripts/link-memory.ps1`; rules in CLAUDE.md).

- **The laptop** (hostname `PRIYANSH`, repo at `D:\cursora`): 8 GB RAM, i5-11300H, Windows 11 Home, C: nearly full ([[keep-big-files-off-c-drive]]). Does the Windows work and the Windows hands-on checks. Its WSL is slow and long runs get killed for low memory.
- **The stronger PC** (hostname `DESKTOP-RTIKELL`, Windows user `FITT`, repo at `C:\Users\FITT\Desktop\PRIYANSH!!\cursora`; the user reaches it with Chrome Remote Desktop): Intel Core Ultra 9 285 (24 threads), 63 GB RAM, NVIDIA RTX 4000 SFF Ada, Windows 11 Pro, ~84 GB free on C: (no D:). Does the Linux work in WSL, chosen by the user on 2026-10-25 over a VirtualBox Ubuntu VM. So issue 2 (a real Linux desktop) stays open. Its WSL `Ubuntu-24.04` (and `docker-desktop`) belong to the user's separate `ethify` work, and the user said on 2026-10-25 not to mix Cursora with it. So Cursora uses its own user `cursora` in that distro (home `/home/cursora`, no password, not in sudo; Rust from rustup in its `~/.cargo`). The user will delete that user when development is finished. CI's apt packages were installed system-wide with `wsl -d Ubuntu-24.04 -u root`, which needs no password. So was the v0.1.0-rc.1 `cursora` .deb (2026-10-25, `--no-install-recommends`, at the user's request); deleting the user doesn't remove it: `apt remove cursora` as root does. The check's files are in `/home/cursora/rc1`. Node 22 is a per-user zip at `%LOCALAPPDATA%\Programs\node-v22.23.4-win-x64`, because winget hangs on this PC. No `gh`, and no Rust on the Windows side.

**X11 checks on this PC (since 2026-10-26):** openbox, xdotool, stalonetray, imagemagick, xclip, mousepad, xterm, python3-pyatspi and fonts-noto-core are installed system-wide (as root, `--no-install-recommends`). A session runs as user `cursora`: Xvfb :99, Openbox, stalonetray and a D-Bus session, with Cursora's settings in `~/x11test` (XDG folders). The app is built with `cargo build --features tauri/custom-protocol` in `~/cursora`, and `~/cursora/src-tauri/llama` links to the rc.1 package's engine `/usr/lib/Cursora/llama` (build 11570). Drive it with step files, not quoted `wsl` commands (issues.md test notes). The pinned Small, Medium and Large models are in `C:\Users\FITT\Models\cursora`.

**On this PC, never run plain `npm run test:linux`:** it runs as the default user `ethify` and copies the repo into `/home/ethify/cursora`. Run `wsl -d Ubuntu-24.04 -u cursora -- bash scripts/wsl-test.sh` instead, after `npm run build`. Never put Cursora files in `/home/ethify`, and never add tools there. Claude Code's memory folder here is `C--Users-FITT-Desktop-PRIYANSH---cursora`.

**Why:** the laptop is short of RAM and disk; the user offered the stronger PC for heavy work.

**How to apply:** check `hostname` at the start of a session to know which PC this is, and read machine-specific notes (paths, scratchpads, installed tools) only for that PC. Put heavy builds and WSL runs on the stronger PC. Related: [[project-status]].
