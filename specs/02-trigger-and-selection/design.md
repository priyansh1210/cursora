# 02: Trigger and selection capture (design)

Status: **Approved** (2026-10-12). Implements [requirements.md](requirements.md). This covers Windows; the Linux implementation is spec 08.

## Overview
```
Ask hotkey (tauri-plugin-global-shortcut, main thread)
  └─ bubble open?  → close it (SEL-1.4, needs spec 03)
  └─ else, if no capture is running → spawn a capture thread:
       capture(&OsPlatform, &cfg) -> Capture
         1. foreground app first (SEL-2.1); elevated → Notice::Elevated, stop (SEL-6.1)
         2. accessibility: UI Automation TextPattern selection
         3. simulated copy, with the app's copy key and clipboard snapshot/restore
              CF_HDROP → Files, CF_UNICODETEXT → Text
         4. nothing → Content::None + hint (SEL-2.3, SEL-6.2)
       on_capture(app, capture)   ← spec 02 logs metadata only; spec 03 opens the bubble
```
- All decisions live in OS-independent code, `capture::capture()`. That code talks to the OS only through a small `Platform` trait.
- Tests drive it with a scripted `FakePlatform`, so almost every criterion is tested automatically without touching the real clipboard.

## Components

| File | Job |
|---|---|
| `src-tauri/src/hotkey.rs` | `validate(new, old)` (SEL-1.2) and `apply(app, &old, &new)`: unregister all, then register `ask` unless paused (APP-1.3). If registering fails, restore the old hotkey and return the error, so `update_settings` doesn't save (SEL-1.3). Only **ask** is registered here: snip arrives with spec 10 and push-to-talk with spec 09, so we don't grab keys we don't use yet. |
| `src-tauri/src/capture/mod.rs` | The types, the `Platform` trait, `capture()`, `copy_key()`, `truncate()` and `SourceApp::for_prompt()`. Also `FakePlatform` and the tests. |
| `src-tauri/src/capture/files.rs` | `file_context(paths, limits) -> FileContext` (SEL-5.2, 5.3, 5.5, 5.6). |
| `src-tauri/src/capture/windows.rs` | `WinPlatform`: foreground app plus elevation, UI Automation, `SendInput`, key state, and the clipboard via `clipboard-win`. |
| `src-tauri/src/capture/other.rs` | A stub platform for non-Windows builds that captures nothing, so CI on Ubuntu compiles. Replaced by spec 08. |
| `src/HotkeyInput.tsx` | The Ctrl+G conflict note (SEL-1.5). |
| `src/Settings.tsx` | General → **Advanced**: the Ctrl+C-unsafe app list, per-app copy shortcuts, and "Share window title with AI". |

### Platform trait
```rust
pub trait Platform {
    fn foreground(&self) -> SourceApp;                 // exe, name, title, elevated (target elevated && we are not)
    fn accessible_selection(&self, max: usize) -> Option<String>;
    fn extra_modifiers_released(&self, wait: Duration) -> bool;   // Shift / Alt / Win
    fn clipboard_seq(&self) -> u32;
    fn clipboard_snapshot(&self) -> Snapshot;          // every HGLOBAL format, raw bytes
    fn clipboard_restore(&self, snap: Snapshot);
    fn clipboard_read(&self) -> Clip;                  // Files(Vec<PathBuf>) | Text(String) | Nothing
    fn send_copy(&self, key: CopyKey);
}
```

### Data (sent to the UI as JSON)
```rust
struct Capture { app: SourceApp, method: Method /* uia|copy|none */, content: Content, notice: Option<Notice> }
enum Content { Text { text: String, truncated: bool }, Files(FileContext), None }
enum Notice { Elevated, NoSelection { offer_snip: bool } }   // offer_snip = active model has vision (false until spec 04)
struct FileContext { items: Vec<FileItem>, omitted: usize }   // first 20 items (SEL-5.5)
struct FileItem { path, name, kind /*file|folder|link*/, size, modified, children: Vec<String> /*≤50*/, content: Option<String>, skipped: Option<String> }
```

### Settings added (`capture`)
| Field | Default | Criterion |
|---|---|---|
| `unsafeApps` | the SEL-4.3 list | SEL-4.3 |
| `copyShortcuts` | `{}` (exe → `ctrlC` / `ctrlInsert` / `ctrlShiftC` / `never`) | SEL-4.4 |
| `shareWindowTitle` | `false` | SEL-7.2 |
| `maxChars` | `50000` | SEL-2.4 |
| `copyTimeoutMs` | `150` | SEL-3.3 |
| `fileMaxBytes` | `100000` | SEL-5.3 |
| `filesTotalBytes` | `300000` | SEL-5.3 |

The last four are "configurable" through `settings.json` only, with no UI, since few people will change them.

## Key mechanics (Windows)
- **Threading:** each capture runs on its own thread, so the main thread never blocks. An `AtomicBool` makes a second hotkey press during a capture do nothing; otherwise two captures would fight over the clipboard.
- **UI Automation:** COM (MTA) on the capture thread.
  1. `IUIAutomation2` with connection and transaction timeouts of **60 ms**, so an app that hangs can't stall us.
  2. `GetFocusedElement` → `TextPattern` → `GetSelection` → `GetText(maxChars + 1)`. One extra character tells us the text was truncated.
- **Copy key** (`copy_key`, pure, SEL-4): per-app override → exe on the unsafe list → `CtrlInsert`, otherwise `CtrlC`. A `never` override skips the copy step.
- **Simulated copy**, in this order:
  1. Wait for Shift/Alt/Win release (up to 300 ms), or skip.
  2. Take a snapshot.
  3. Read `seq0`, then `SendInput` the copy key.
  4. Poll `GetClipboardSequenceNumber` every 5 ms, up to `copyTimeoutMs`.
  5. If the number changed, read the clipboard; otherwise treat it as nothing selected (SEL-3.4).
  6. **Always restore** (SEL-3.1).
- **Elevation:** `OpenProcess(QUERY_LIMITED)` → `OpenProcessToken` → `TokenElevation`. If access is denied, treat the process as elevated: a higher-level process is exactly the case where opening its token is refused.
- **Display name:** the exe name without `.exe`. `// shortcut: exe stem, not the product name; read FileDescription if names look wrong in the bubble.`
- **Budget (SEL-2.5):** UI Automation (≤ 60 ms) + copy (≤ 150 ms) puts the worst case at about 220 ms before the bubble shows. When UI Automation succeeds (browsers, Office, Notepad, terminals) it's about 30 ms. We measure in the manual check; if we miss, the copy timeout is the knob.

## Dependencies
- `tauri-plugin-global-shortcut` ("2"): the global hotkey.
- `clipboard-win` (5): raw clipboard formats, sequence number, file list. It's small, widely used (arboard builds on it), and replaces about 80 lines of our own unsafe Win32 code.
- `windows` **0.62** (Windows only, the same version Tauri already compiles), with the features UI Automation, COM, KeyboardAndMouse, WindowsAndMessaging, Threading, Security.

## Known limits (documented, not fixed)
- Windows clipboard history (Win+V) and clipboard managers can record the temporary copy. The target app writes it, so we can't stop that. Prefer UI Automation, which avoids the clipboard entirely.
- When the clipboard is busy, or holds formats that can't be saved as bytes (metafiles such as Excel charts, private or GDI formats), the copy step is **skipped** and the clipboard isn't touched. Only UI Automation can capture then.
- Secure desktops (UAC prompt, lock screen) never fire the hotkey (out of scope, per the requirements).

## Requirement tweaks (approved, applied)
1. **SEL-1.2:** "at least one modifier" becomes **at least one of Ctrl, Alt or Win**. Shift alone would fire on every capital letter.
2. **SEL-2.2:** the default order becomes **accessibility → simulated copy**, not user-configurable. On Windows, files arrive through the copy step (Explorer and the Desktop put a file list on the clipboard). Linux file managers stay as specified in LNX-3.2.
3. **SEL-3.2:** wait only for **Shift, Alt and Win** to be released. Every copy key includes Ctrl anyway, and the registered hotkey swallows its own main key. Waiting for Ctrl too would add up to 300 ms on every capture.
4. **SEL-5.1:** files are read **only through the clipboard file list** (copy, then restore). The shell COM selection API is dropped: one mechanism instead of two, and much less unsafe COM code.
5. **SEL-5.4 moves to spec 10 (VIS):** attaching image files needs image decoding and downscaling, which spec 10 builds anyway. Until then, images are sent as metadata only. SEL-5.4 is retired and re-added as VIS-2.7.

## Test strategy
| Criterion | Test |
|---|---|
| SEL-1.2 | `hotkey::validate`: no Ctrl/Alt/Win, reserved combo, duplicate, unparsable, and only changed fields checked |
| SEL-2.1 | The fake records the call order; `foreground` comes first |
| SEL-2.2 / 2.3 | UI Automation hit → no copy sent; miss → copy; nothing → `Content::None` |
| SEL-2.4 | `truncate` at a char boundary sets `truncated` |
| SEL-3.1 | Restore runs on success **and** on timeout |
| SEL-3.2 | Extra modifier held → `send_copy` never called |
| SEL-3.3 / 3.4 | No sequence-number change → `None`, even though the fake clipboard still holds old text |
| SEL-4.1–4.4 | `copy_key` cases, plus the default list contains the terminals and IDEs |
| SEL-5.2/5.3/5.5/5.6 | `file_context` on a temp dir: text inlined, binary/oversize skipped with a reason, folder children ≤ 50, 25 items → 20 + 5 left out, a symlink is not followed |
| SEL-6.2 | `NoSelection { offer_snip }` follows the vision flag |
| SEL-7.1 / 7.2 | Capture carries app and method; `for_prompt(false)` has no title |
| Manual | SEL-1.1, 1.3, 1.4, 1.5, 2.5, 3.5, 4.5, 5.1, 6.1, 8.1. These run once spec 03's bubble can show a capture; until then `on_capture` only logs metadata (method, length, app), never the text (APP-7.2). |

## Changes during implementation
- **2026-10-12, SEL-5.3:** text files are recognized by content alone (valid UTF-8 without NUL bytes, or UTF-16 with a BOM). The extension allowlist was dropped. It added a list to maintain and missed extensionless files like `README` or `Dockerfile`, while the content check already rejects binaries.
- **2026-10-12:** `clipboard-win` needs its `std` feature for `get_file_list_path`.
- **2026-10-12 (/ponytail-review):** `clipboard_snapshot` returns `Option`. Before this, a busy clipboard produced an empty snapshot, and the restore then wiped the user's clipboard. The snapshot now also refuses unrestorable formats. With no window in front (pid 0), the source is "unknown", not "admin app".
