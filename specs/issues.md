# Issues

Every known problem, gap and untested item, from the whole of development. Bugs, missing behavior, checks that couldn't be run, and test-environment traps all go here when they are found. A fixed issue moves to **Resolved** with the date and what fixed it. Ideas for later go in [backlog.md](backlog.md) instead.

## Open

| # | Found | Area | Issue | Next step |
|---|---|---|---|---|
| 1 | 2026-10-13 | BUB-1.5 | **Two monitors are untested** on Windows and on Linux. The per-monitor logic has unit tests with two monitors, but no real check: this PC has one screen, and a virtual X server can't give GTK a second monitor (Xvfb's monitors and the dummy driver's output-less monitor are invisible to it). | Run BUB-1.5 with a second screen, on Windows and on Linux. |
| 2 | 2026-10-13 | Spec 03 on Linux | **The X11 checks ran in WSL** (Xvfb + Openbox), not on a real Ubuntu desktop. GNOME is stricter than Openbox about which windows may take focus, so BUB-2.2 (input focus) can still differ there. The Wayland session is untested. | Repeat the spec 03 X11 checks on Ubuntu (GNOME, X11 session), then the Wayland session (spec 08). |
| 3 | 2026-10-13 | Linux, BUB-3.1 | **Linux captures nothing yet** (`capture/other.rs` is a stub): the bubble opens as a general chat. | Spec 08: X11 PRIMARY, files, Wayland. |
| 4 | 2026-10-13 | Linux, BUB-2.3 | **Focus doesn't go back to the app after the bubble closes** on Linux (`bubble/other.rs` has no window-handle focus control). | Spec 08. |
| 5 | 2026-10-13 | SEL-6.1 | **The admin-app check hasn't run**: it needs someone to click Yes on the Windows admin prompt. Also, the spec asks for a "Restart Cursora as administrator" action, and the bubble only shows the message. | Run the check with the user at the PC; decide whether to add the button or change the spec. |
| 6 | 2026-10-13 | APP-1.2 (tests) | **The tray menu sometimes doesn't open** for the test script's synthetic clicks (about 1 in 4 right after a menu was closed). Not confirmed as a Cursora bug; real clicks may behave differently. Windows 11 also hides new tray icons in the overflow flyout, which the script can't drive reliably, so it shows the icon in the taskbar during tests. | Watch for it in real use; if real users see it, look at the tray-icon crate's menu handling. |
| 7 | 2026-10-13 | LLM-8.4 | **WebView2 still makes a DNS-over-HTTPS probe** to the PC's DNS servers (here 8.8.8.8 and 8.8.4.4) when it starts. No startup argument turns it off. It carries no Cursora data, and offline it simply fails. Cursora itself and the engine connect to nothing but the chosen model. | Leave unless a WebView2 option appears; mention it in the privacy notes if asked. |
| 8 | 2026-10-13 | SEL-8.1 | **Apps not tested**: Firefox, VS Code (editor and terminal), an Electron app, IntelliJ. None is installed on the test PC. | Install and test, or test on another PC. |
| 9 | 2026-10-13 | LLM-3.1, APP-8.2 | **The installers and CI are unverified**: whether the .msi/.deb/.AppImage contain llama-server and its libraries, and whether CI is green with a `v*` tag producing draft-release installers. | Push a test tag, check the CI run and the installers. |
| 10 | 2026-10-13 | APP-3.1 | **"Start with system" across a logout and login** hasn't been checked (the Run entry is written; the login itself wasn't tried). | Log out and in once. |
| 11 | 2026-10-13 | Linux, all specs | **The remaining Linux manual items**: specs 01, 02 and 04 (tray, menu, Quit, command line, providers, built-in engine on Linux, LLM-6.3 with no Secret Service). Many can run in WSL like spec 03 did. | Next WSL session, then real Ubuntu. |
| 12 | 2026-10-13 | History tab | **The search box is too narrow**: its placeholder is cut off ("Search questions, answe"). Cosmetic. | Widen the input. |
| 13 | 2026-10-13 | Dev builds | **A debug build opens a console window** that takes the focus when Cursora starts. With Cursora not running, `cursora --ask` then reads from the console instead of the app the user was in. Release builds have no console. | None for users; test scripts start the debug build hidden. |
| 14 | 2026-10-13 | Linux (tests) | **The lock emoji in the "🔒 Local" badge shows as a box** on a Linux system without an emoji font (seen in WSL). Ubuntu installs Noto Color Emoji by default. | Check on real Ubuntu; consider a plain-text fallback. |
| 15 | 2026-10-13 | SEL-1.3 | **The plain hotkey-conflict message isn't checked in the app yet** (the debug text was removed from it; the code is built and tested, the message wasn't watched). | Look at it in the next Windows run. |

## Test environment notes
Traps found while automating checks; they waste time if forgotten.

- **WSL has its own Wayland display (WSLg).** GTK picks it over the test X server, so Linux Cursora windows appear on the Windows desktop. Tests set `unset WAYLAND_DISPLAY; export GDK_BACKEND=x11`. (Early in the spec 03 X11 run, a Linux bubble may have flashed on the Windows screen because of this.)
- **GTK creates a hidden window's X window only when it is first shown**, so the bubble can't be found before the first hotkey. GTK's helper window is also named "cursora", and `xdotool search --name` ignores case: search `--onlyvisible`.
- **`xwininfo` and `xprop` wait for a mouse click when given no window id**, which hangs a script.
- **`xdotool getwindowgeometry` reports positions relative to the window manager's frame**; `xwininfo` gives absolute ones.
- **`pkill -f "<pattern>"` inside `bash -c "...<pattern>..."` kills its own shell.**
- **Windows PowerShell 5.1 reads a script without a byte order mark as ANSI**: keep test scripts ASCII, and write Unicode as `\uXXXX`.
- **PowerShell variable names ignore case**: `$out` overwrites `$Out`, `$c` overwrites `$C`.
- **Test scripts must never type into the user's apps.** In one run, keystrokes meant for Cursora's Settings went to the user's Chrome window (a GitHub Copilot chat) when another window came to the front. The scripts now refuse to type, press Enter or Tab, or click unless the window in front belongs to Cursora or the test.
- **Windows reconnects Wi-Fi on its own** after `netsh wlan disconnect`; an offline test must keep disconnecting while it runs.
- **Edge's PDF viewer ignores Ctrl+A**; select by dragging.
- **The Desktop's icons aren't visible to UI Automation** from the test shell; select an icon by typing its name.

## Resolved

| # | Found | Area | Issue | Fixed |
|---|---|---|---|---|
| R1 | 2026-10-12 | LLM-3.6 | A killed engine showed "Couldn't reach the provider" instead of its reason. | `engine::exited_within`, and the bubble shows the error's own words. |
| R2 | 2026-10-12 | APP-4.2 | `settings.json` with a byte order mark was taken as broken and replaced by defaults. | The mark is skipped when loading. |
| R3 | 2026-10-12 | LLM-6.3 | On Linux without a keyring, even local models stopped working. | Only cloud providers need the keyring. |
| R4 | 2026-10-12 | LLM-3.6 | A cleanly closed connection from a killed engine skipped the engine check; every engine error waited a second for nothing. | The check covers a clean close and runs only for network errors. |
| R5 | 2026-10-12 | APP-7.2 | A log line could write the engine's output (file paths) to the log. | Only the error kind is logged. |
| R6 | 2026-10-13 | BUB-3.2, 8.1 | After × removed the context, keyboard focus was lost. | Focus goes back to the input. |
| R7 | 2026-10-13 | SEL-1.3 | The hotkey-conflict message ended with debug text. | The detail goes to the log only (see open issue 15). |
| R8 | 2026-10-13 | LLM-4.2 | A provider's name and base URL couldn't be edited. | Editable fields in the provider card. |
| R9 | 2026-10-13 | APP-7.1 | Log rotation kept 6 files instead of 5. | `KeepSome(4)`: 4 old files plus the current one. |
| R10 | 2026-10-13 | APP-2.3 | With Cursora not running, `cursora --ask` opened an empty bubble. | The page fetches the conversation once it listens. |
| R11 | 2026-10-13 | LLM-3.5 | A long answer could lose its engine to the idle timer mid-answer. | An answer holds the engine; the idle time counts from its end. |
| R12 | 2026-10-13 | LLM-3.5 | How long the model stays loaded wasn't the user's choice. | Always on (default) / Off when idle / Only while answering, in Setup and Settings; "Stop the model" in the tray and the Models tab. |
| R13 | 2026-10-13 | LLM-8.4 | WebView2 contacted Microsoft services in the background. | Background networking, component updates, domain reliability and pings are off in every window. |
| R14 | 2026-10-13 | Linux, BUB-1.2 | The bubble hung 33 px off the bottom edge on Linux (GTK kept the non-resizable window at 200 px). | The bubble is resizable on Linux (still frameless). |
| R15 | 2026-10-13 | Linux, BUB-1.1, 2.2 | On Linux the bubble opened at the top-left or where it was last, not by the cursor, and typing went to the app behind it. | The place is set again 150 ms after showing; focus works too. |
| R16 | 2026-10-13 | Tests | Three network tests failed when WSL was busy (borrowed 1-second deadlines). | Only the timeout test keeps them. |
