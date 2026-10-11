# 02: Trigger and selection capture (SEL)

## Scope
- **In:**
  - The global "ask" hotkey
  - Capturing what the user selected in **any** app (web browsers, file managers, editors, terminals, office apps, PDF viewers, Electron and Java apps) as text or as file/folder paths
  - Clipboard safety, terminal safety, and edge cases
  - Requirements are platform-neutral unless they are marked *Windows*.
- **Out:**
  - Linux implementations (spec 08 LNX)
  - Deciding whether an app is blocked (spec 07 PRI, which runs **before** capture)
  - Screen snipping (spec 10 VIS)
  - Secure desktops (UAC prompt, lock screen), where hotkeys don't fire
  - Rich formats (HTML/RTF): v1 captures plain text only
  - Extracting text from PDF/DOCX files that are selected *as files*: metadata only

## Requirements

### SEL-1 Ask hotkey
User story: As a user, I want one shortcut that works everywhere, so that I can ask about whatever I'm looking at.
- SEL-1.1 (manual) WHEN the user presses the ask hotkey (default **Ctrl+G**) in any app THE SYSTEM SHALL start the capture flow.
- SEL-1.2 (auto) WHEN the user sets a new hotkey THE SYSTEM SHALL validate it. The hotkey must:
  - include at least one modifier,
  - not be OS-reserved (e.g. Ctrl+Alt+Del, Win+L), and
  - not duplicate another Cursora hotkey.

  An invalid hotkey SHALL be rejected with the reason.
- SEL-1.3 (manual) IF the hotkey cannot be registered because another app owns it THE SYSTEM SHALL tell the user and keep the previous hotkey.
- SEL-1.4 (manual) WHEN the bubble is open and the ask hotkey is pressed (wherever the focus is) THE SYSTEM SHALL close the bubble. The hotkey is the only way to close it (BUB-2.3).
- SEL-1.5 (manual) WHEN the user picks Ctrl+G as the ask hotkey (in setup or Settings) THE SYSTEM SHALL show a one-line note that it overrides "Go to line" in VS Code/Notepad and "Find next" in browsers.

### SEL-2 Capture pipeline
- SEL-2.1 (auto) WHEN the capture flow starts THE SYSTEM SHALL record the foreground window and app **before** showing any Cursora window.
- SEL-2.2 (auto) THE SYSTEM SHALL try the capture strategies in their configured order, by default:
  1. files (when the app is a file manager)
  2. accessibility API
  3. simulated copy

  It SHALL stop at the first strategy that returns non-empty content.
- SEL-2.3 (auto) WHEN every strategy returns nothing THE SYSTEM SHALL open the bubble in general-chat mode with no attached context.
- SEL-2.4 (auto) WHEN captured text exceeds the context limit (default 50,000 characters, configurable) THE SYSTEM SHALL truncate it and mark the capture as truncated. The bubble shows a notice (BUB-3.3).
- SEL-2.5 (manual) WHEN the hotkey is pressed THE bubble SHALL become visible within 200 ms, including on the simulated-copy path (constitution §8).

### SEL-3 Clipboard safety for simulated copy
User story: As a user, I don't want Cursora to mess up what I copied earlier.
- SEL-3.1 (auto) WHEN the simulated-copy strategy runs THE SYSTEM SHALL snapshot the clipboard first and restore it afterwards. This applies on success, failure and timeout alike, and covers at least the text, image and file-list formats.
- SEL-3.2 (auto) BEFORE sending simulated keys THE SYSTEM SHALL wait until the user has physically released the hotkey's keys (up to 300 ms). IF any key is still held after that THE SYSTEM SHALL skip the simulated copy rather than send a combined shortcut.
- SEL-3.3 (auto) THE SYSTEM SHALL detect a successful copy only by a clipboard change (Windows: `GetClipboardSequenceNumber`). It SHALL give up after 150 ms (configurable).
- SEL-3.4 (auto) WHEN the clipboard did not change after a simulated copy THE SYSTEM SHALL treat the result as "nothing selected" and SHALL NOT use the old clipboard content.
- SEL-3.5 (manual) WHEN the user copies X, then selects other text Y in another app, presses the hotkey, closes the bubble and pastes THE pasted content SHALL be X, AND the bubble SHALL have shown Y.

### SEL-4 App-specific copy keys and terminal safety
User story: As a developer, I want to ask about terminal output without killing my running process.
- SEL-4.1 (auto) WHEN the foreground app is in the **Ctrl+C-unsafe** list THE SYSTEM SHALL NOT simulate Ctrl+C.
- SEL-4.2 (auto) WHEN the accessibility strategy fails in a Ctrl+C-unsafe app THE SYSTEM SHALL simulate Ctrl+Insert instead (*Windows*).
- SEL-4.3 (auto) THE default Ctrl+C-unsafe list (*Windows*) SHALL contain:
  - terminals: `WindowsTerminal.exe`, `conhost.exe`, `OpenConsole.exe`, `cmd.exe`, `powershell.exe`, `pwsh.exe`, `mintty.exe`, `alacritty.exe`, `wezterm-gui.exe`
  - IDEs with built-in terminals: `Code.exe`, `Cursor.exe`, `idea64.exe`, `pycharm64.exe`

  The list SHALL be user-editable in Settings → General → Advanced.
- SEL-4.4 (auto) WHERE the user has set a per-app copy key (Ctrl+C, Ctrl+Insert, Ctrl+Shift+C, or "never simulate") THE SYSTEM SHALL use it for that app instead of the default.
- SEL-4.5 (manual) WHEN text is selected in Windows Terminal while a long-running process is running and the hotkey is pressed THE process SHALL keep running AND the bubble SHALL show the text.

### SEL-5 Files and folders
User story: As a user, I want to select files in File Explorer and ask about them.
- SEL-5.1 (manual) WHEN files or folders are selected in File Explorer or on the Desktop and the hotkey is pressed THE SYSTEM SHALL capture their full paths. It SHALL use the shell selection API first and fall back to the clipboard file list.
- SEL-5.2 (auto) WHEN a capture contains paths THE SYSTEM SHALL build a file context:
  - For each item: name, kind (file/folder), extension, size and modified date.
  - For each folder: a listing of up to 50 direct children.
- SEL-5.3 (auto) WHERE a file is text-like (by extension, plus a UTF-8/UTF-16 check of its first 8 KB) AND it is ≤ 100 KB AND the running total is ≤ 300 KB (both limits configurable) THE SYSTEM SHALL include its content. Otherwise THE SYSTEM SHALL include metadata only, with the reason the content was left out.
- SEL-5.4 (auto) WHERE a file is an image (png, jpg, jpeg, webp, gif) AND the active model supports vision THE SYSTEM SHALL attach it, downscaled so the longest side is ≤ 2048 px. At most 4 images are attached. Otherwise THE SYSTEM SHALL include metadata only.
- SEL-5.5 (auto) WHEN more than 20 items are selected THE SYSTEM SHALL include the first 20 and state how many were left out.
- SEL-5.6 (auto) THE SYSTEM SHALL NOT recurse into subfolders and SHALL NOT follow symlinks or junctions.

### SEL-6 Special cases
- SEL-6.1 (manual) *Windows:* WHEN the foreground window belongs to an elevated (admin) process and Cursora is not elevated THE SYSTEM SHALL NOT attempt simulated input. It SHALL show "Cursora can't read from apps running as administrator", with a "Restart Cursora as administrator" action.
- SEL-6.2 (auto) WHEN nothing could be captured:
  - AND the active model supports vision THE bubble SHALL offer "Snip an area instead" (VIS).
  - Otherwise THE bubble SHALL open in general-chat mode with the hint "No selectable text found."

### SEL-7 Source metadata
- SEL-7.1 (auto) THE capture result SHALL record:
  - the source app (executable name and display name)
  - the window title
  - the capture method: `uia`, `copy`, `files`, `primary` or `none`
- SEL-7.2 (auto) THE prompt context SHALL include the source app's display name. It SHALL NOT include the window title unless "Share window title with AI" is enabled (default **off**).

### SEL-8 Compatibility matrix (Windows)
- SEL-8.1 (manual) Text capture SHALL work in:
  - browsers: Chrome, Edge, Firefox
  - Notepad, Word, Excel (cells)
  - a PDF viewer (Edge PDF or SumatraPDF)
  - VS Code (editor **and** integrated terminal), Windows Terminal
  - an Electron app (e.g. Obsidian/Discord) and a Java IDE (IntelliJ)

  File capture SHALL work in File Explorer (files and folders) and on the Desktop.

## Open questions
- _(none.)_

## Resolved
- In the VS Code integrated terminal, Ctrl+Insert copies the selection, so `Code.exe` stays on the Ctrl+C-unsafe list (SEL-4.3).
- Cursora works on the selection, and text is never pasted into the chat by hand (BUB-3.4 retired). With nothing selected, the bubble opens as a general chat (SEL-2.3).
- A Ctrl+G conflict note is shown when picking the hotkey (SEL-1.5). The hotkey is user-changeable (SEL-1.2) and is chosen during setup (APP-6.1).
