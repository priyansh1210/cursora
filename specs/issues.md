# Issues

Every known problem, gap and untested item, from the whole of development. Bugs, missing behavior, checks that couldn't be run, and test-environment traps all go here when they are found. A fixed issue moves to **Resolved** with the date and what fixed it. Ideas for later go in [backlog.md](backlog.md) instead.

## Open

| # | Found | Area | Issue | Next step |
|---|---|---|---|---|
| 1 | 2026-10-13 | BUB-1.5 | **Two monitors are untested** on Windows and on Linux. The per-monitor logic has unit tests with two monitors, but no real check: this PC has one screen, and a virtual X server can't give GTK a second monitor (Xvfb's monitors and the dummy driver's output-less monitor are invisible to it). | Run BUB-1.5 with a second screen, on Windows and on Linux. |
| 2 | 2026-10-13 | Spec 03 on Linux | **The X11 checks ran in WSL** (Xvfb + Openbox), not on a real Ubuntu desktop. GNOME is stricter than Openbox about which windows may take focus, so BUB-2.2 (input focus) can still differ there. The Wayland session is untested. | Repeat the spec 03 X11 checks on Ubuntu (GNOME, X11 session), then the Wayland session (spec 08). |
| 3 | 2026-10-13 | Linux, BUB-3.1 | **Linux captures nothing yet** (`capture/other.rs` is a stub): the bubble opens as a general chat. It also can't tell which app is in front, so the blocklist (PRI-2.1) and the tray's "Block Cursora in <app>" (PRI-2.4) don't work there yet; the Privacy tab says so (PRI-2.5). | Spec 08: X11 PRIMARY, files, Wayland. |
| 4 | 2026-10-13 | Linux, BUB-2.3 | **Focus doesn't go back to the app after the bubble closes** on Linux (`bubble/other.rs` has no window-handle focus control). | Spec 08. |
| 5 | 2026-10-13 | SEL-6.1 | **The admin-app check hasn't run**: it needs someone to click Yes on the Windows admin prompt. Also, the spec asks for a "Restart Cursora as administrator" action, and the bubble only shows the message. | Run the check with the user at the PC; decide whether to add the button or change the spec. |
| 6 | 2026-10-13 | APP-1.2 (tests) | **The tray menu sometimes doesn't open** for the test script's synthetic clicks (about 1 in 4 right after a menu was closed). Not confirmed as a Cursora bug; real clicks may behave differently. Windows 11 also hides new tray icons in the overflow flyout, which the script can't drive reliably, so it shows the icon in the taskbar during tests. | Watch for it in real use; if real users see it, look at the tray-icon crate's menu handling. |
| 7 | 2026-10-13 | LLM-8.4 | **WebView2 still makes a DNS-over-HTTPS probe** to the PC's DNS servers (here 8.8.8.8 and 8.8.4.4) when it starts. No startup argument turns it off. It carries no Cursora data, and offline it simply fails. Cursora itself and the engine connect to nothing but the chosen model. | Leave unless a WebView2 option appears; mention it in the privacy notes if asked. |
| 8 | 2026-10-13 | SEL-8.1 | **Apps not tested**: Firefox, VS Code (editor and terminal), an Electron app, IntelliJ. None is installed on the test PC. | Install and test, or test on another PC. |
| 9 | 2026-10-13 | LLM-3.1, APP-8.2 | **The installers are unverified**: whether the .msi/.deb/.AppImage contain llama-server and its libraries, and whether a `v*` tag produces draft-release installers. CI itself is green on both OSes (PR #5). | Push a test tag (it creates a public draft release), check the run and the installers. |
| 10 | 2026-10-13 | APP-3.1 | **"Start with system" across a logout and login** hasn't been checked (the Run entry is written; the login itself wasn't tried). | Log out and in once. |
| 11 | 2026-10-13 | Linux | **Linux checks still to run**: spec 01 (tray, menu, Quit, command line), spec 02 (hotkey, clipboard), and from spec 04 the tray's "Stop the model" and a killed engine. Spec 04's other Linux checks and all of spec 05's (its tray menu too, with stalonetray as the tray host) passed in WSL (Xvfb + Openbox). Every WSL result still needs one pass on real Ubuntu (GNOME, X11 and Wayland), where the icon goes through the AppIndicator protocol instead of stalonetray's older one. | Spec 01 and 02 and the spec 04 tray item in WSL with stalonetray, then real Ubuntu. |
| 12 | 2026-10-13 | History tab | **The search box is too narrow**: its placeholder is cut off ("Search questions, answe"). Cosmetic. | Widen the input. |
| 13 | 2026-10-13 | Dev builds | **A debug build opens a console window** that takes the focus when Cursora starts. With Cursora not running, `cursora --ask` then reads from the console instead of the app the user was in. Release builds have no console. | None for users; test scripts start the debug build hidden. |
| 14 | 2026-10-13 | Linux (tests) | **The lock emoji in the "🔒 Local" badge, and the 🎭 personality toggle in the bubble header, show as boxes** on a Linux system without an emoji font (seen in WSL). Ubuntu installs Noto Color Emoji by default. | Check on real Ubuntu; consider a plain-text fallback. |
| 15 | 2026-10-13 | SEL-1.3 | **The plain hotkey-conflict message isn't checked in the app yet** (the debug text was removed from it; the code is built and tested, the message wasn't watched). | Look at it in the next Windows run. |
| 16 | 2026-10-13 | LLM-1.1, 1.2, 7.1 | **No real cloud provider has been tested end to end.** The adapters were tested against recorded responses and a fake server only; there are no API keys on the test PC (a Claude subscription doesn't include API access). | Free keys cover two of the three formats: Google AI Studio (Gemini) and Groq or OpenRouter's `:free` models (OpenAI-compatible). Check Test connection, the model list, a streamed answer, a wrong key and an unknown model. The third format needs prepaid API credits; until then it stays tested against recordings. Use test text only: free tiers may use what is sent. Then check redaction and confirming with it: spec 07 checked them with the Echo model posing as a cloud model. |
| 17 | 2026-10-13 | LLM-2.1 | **LM Studio detection was only tested against a fake server**; LM Studio isn't installed on the test PC. | Install LM Studio once and check it is found and answers. |
| 18 | 2026-10-13 | BUB-2.2 on X11 | **The bubble doesn't reliably get the keyboard focus on X11.** In WSL (Xvfb + Openbox) it sometimes does and sometimes doesn't; when it doesn't, typing goes to the app behind it, which in a terminal could run a command. The window manager's focus-stealing rules decide: Cursora's focus request carries no fresh user timestamp, because the hotkey arrives outside GTK. GNOME is stricter than Openbox. | Spec 08: activate the bubble with the hotkey's X timestamp (or `_NET_ACTIVE_WINDOW` as a pager), then check on GNOME. |
| 19 | 2026-10-13 | LLM-6.3 | **The no-keyring message shows the raw error** ("Platform failure: zbus error: org.freedesktop.DBus.Error.ServiceUnknown…") before telling the user what to do. Cosmetic. | Keep the plain sentence; put the detail in the log. |
| 20 | 2026-10-13 | Accessibility, Linux | **On Linux, screen readers can't read text inside flex rows.** WebKitGTK leaves the text of a `display: flex` container's items out of AT-SPI. Affected: the bubble header (persona, model, badge), the "Built-in", "Active" and "Overrides built-in" badges in Settings, and the context row ("From … · N characters"). Confirmed: with the header as a block, its text appears ("Sage echo 🔒 Local"). Windows (UI Automation) is not affected. Found while automating spec 05 on X11. | Spec 08: give these rows a layout or element that exposes text (and check with Orca); the spec 05 tests read the header from screenshots meanwhile. |
| 22 | 2026-10-13 | PER-1.3 | **A preset added in a later version can clash with a user's persona name.** Names must be unique, so every settings save would then be refused (the tray only logs it) until the user renames theirs. | When a preset is added, rename a clashing user persona on load (" 2"), as Import does. |

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
- **UI checks on Windows can't run while the screen is locked**: the foreground window is the lock screen, and the guard stops the script. Check for `LockApp` or `LogonUI` first: a locked screen can show either one.
- **On Linux, text in flex rows isn't in AT-SPI** (open issue 20): a script can't read the bubble header; it uses screenshots or a temporary block layout.
- **WSL's tray host is stalonetray**: with no AppIndicator watcher, libayatana-appindicator falls back to an XEmbed icon, which stalonetray shows; a left click opens the menu, and AT-SPI sees the items (the popup has no title, so search the whole app).
- **A screen grab closes an open GTK menu** (ImageMagick's `import` breaks the menu's pointer grab): take screenshots after the clicks.
- **The Windows test window is always on top**, so it can cover the window under test and catch its clicks (the guard allows clicks on it): hide it first.
- **Newer WebView2 returns a Promise from `scrollIntoView`**: a React effect written as `() => el.scrollIntoView()` hands it back as the cleanup, and React crashes on the next run. Effects get a block body.
- **A render error leaves the page blank, and there is no console in the tests.** A temporary error boundary that prints the error on the page lets UI Automation read it.
- **The WSL engine lives in `~/cursora/src-tauri/llama/`**, copied from `~/llama-b11570` (fetched and SHA-256 checked by the test script). Syncing the repo from D: must not overwrite it with the Windows engine.

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
| R15 | 2026-10-13 | Linux, BUB-1.1 | On Linux the bubble opened at the top-left or where it was last, not by the cursor. | The place is set again 150 ms after showing. (Focus first looked fixed too, but isn't reliable: open issue 18.) |
| R16 | 2026-10-13 | Tests | Three network tests failed when WSL was busy (borrowed 1-second deadlines). | Only the timeout test keeps them. |
| R17 | 2026-10-13 | Accessibility | Every window's page was titled "Tauri + React + Typescript" (the template's), which screen readers announce. | The title is "Cursora". |
| R18 | 2026-10-13 | Linux, BUB-1.1 | Cursora froze on Linux: the delayed re-placing of the bubble (R15) held the bubble's lock while moving the window, which waits for the main thread, while the main thread waited for that lock in `bubble_resize`. Found by the spec 04 Linux checks (a second `cursora --settings` hung). | The lock is let go before the window is moved. |
| R19 | 2026-10-13 | PER-4.2 | A window title (when shared) or a file path could carry `</selected_text>` or `</file>` into the request unescaped; a web page's title was enough. Found by the spec 05 review (spec 03's `<context>` had the same gap). | Attribute values are escaped like the content. |
| R20 | 2026-10-13 | PER-2.2 | A conversation reopened from History kept showing its saved persona after today's persona answered. | It shows today's persona when it opens. |
| R21 | 2026-10-13 | Accessibility | Closing the persona editor lost the keyboard focus. | Focus goes to the card's Edit button. |
| R22 | 2026-10-13 | Settings | Badges in setting rows ("Built-in", "Active", a provider's 🔒 Local) were stretched to 100 px by the row's label width. | Badges keep their own width. |
| R23 | 2026-10-13 | Tests | `npm run test:linux` replaced the WSL copy's Linux engine with the Windows one (it synced `src-tauri/llama/`), so the next Linux UI run said "The built-in engine isn't installed". | The sync leaves `src-tauri/llama/` alone. |
| R24 | 2026-10-13 | Spec 05 on Windows | Spec 05's Windows checks couldn't run while the screen was locked (was issue 21). | Run once the screen was unlocked: all passed. |
| R25 | 2026-10-13 | PER-3.1 | `/Persona ada` (capital P) went to the model as a question (was issue 23). | Command names ignore case (spec 06). |
| R26 | 2026-10-13 | SKL-2.1 | The autocomplete crashed the bubble on the second letter typed after `/` (found in the spec 06 Windows run, before any commit). | The effect that scrolls the selected item into view has a block body. |
| R27 | 2026-10-13 | SKL-2.1 | Every command needed two Enters: the first only completed `/help` to `/help `. | Enter on a name typed in full runs it (SKL-2.1 amended). |
| R28 | 2026-10-13 | SKL-4.4 | On Windows, a hand-made skill file named in other letters than `<name>.md` (`ELI5.md`) couldn't be saved after editing; a careless fix would have deleted it. Found by the spec 06 review. | Saving compares files, not spellings. |
| R29 | 2026-10-13 | SKL-1.2 | A lone `}}` before a variable stopped the template from being filled in. Found by the review. | `render` looks for `}}` after each `{{`. |
| R30 | 2026-10-13 | SKL-3.2 | The built-in skills hadn't been judged with a capable model; with the 0.5B test model only `/translate` gave a sensible answer (was issue 26). | Not a Cursora defect: answer quality depends on the model the user picks. SKL-3.2 is retired; the instructions reaching the model is covered by SKL-2.2 and SKL-3.1. |
| R31 | 2026-10-13 | PER-1.4 | A very small model repeated the persona's instructions ("My name is Ada, a female.") instead of following them (was issue 24). | Not a Cursora defect: how well a model follows the persona depends on the model the user picks. The instructions are covered by the PER-1.4 and PER-4.3 tests. |
| R32 | 2026-10-13 | PER-1 | Tone, emoji and reply language hadn't been checked with a capable model (was issue 25). | Not a Cursora check, for the same reason; the spec 05 checklist item is closed with a note. |
| R33 | 2026-10-13 | PRI-1.2 | The "🔒 Local-only" badge wrapped onto two lines when the bubble header was crowded (a long model name in the switcher). Found in the spec 07 Windows run, before any commit. | Badges never wrap. |
| R34 | 2026-10-13 | PRI-2.4 | Every process rule in Settings → Privacy had the same accessible name ("Process name"), so a screen reader couldn't tell the rows apart. Found while writing the spec 07 checks. | Each input is named by its kind and value ("Process name: KeePass.exe"). |
| R35 | 2026-10-13 | PRI-2.4 | Pressing Ctrl+G in a new app could freeze Cursora: `tray::seen` rebuilt the tray menu while holding the settings lock, and a tray update waits for the main thread, which may want that lock (was issue 27, found by the spec 07 review). | The settings are copied first, and `seen` runs after the bubble opens. |
| R36 | 2026-10-13 | PRI-3.6, 3.7 | "What will be sent" could name the wrong receiver after a model switch in the bubble header (was issue 28). | The preview refreshes when the settings change. |
| R37 | 2026-10-13 | PRI-3.4 | A refused pattern showed its error twice (was issue 29). | The pattern form saves directly, so the reason shows only next to it. |
