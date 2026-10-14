# 10: Vision snip (design)

Status: **Approved** (2026-10-14). Implements [requirements.md](requirements.md). Images already reach every provider (`Message.images`, spec 04) and the cloud-image warning exists (PRI-3.8, spec 07). This spec adds the screen grab, the overlay, the attached image, images from file managers (VIS-2.7) and the snip hotkey. It brings back the Snip field in Setup and Settings → General (issue 37).

## Overview
```
snip hotkey (Ctrl+Shift+G) / `cursora --snip` / `/snip` / "Snip an area instead" (SEL-6.2)
 ─→ start(): paused? ignore · the app in front blocked? → the bubble's blocked notice, no grab (VIS-3.1)
             the model can't see? → the bubble's hint, with "Use <model> for this snip" if another one can (VIS-1.2)
 ─→ the bubble hides; the whole screen (every monitor) is grabbed into memory
 ─→ the overlay: one window over all monitors, showing that frozen grab, dimmed, with a crosshair (VIS-2.1)
       drag a rectangle · Esc or smaller than 8×8 cancels (VIS-2.3)
 ─→ the region is cut from the grab, which is dropped; the overlay closes (VIS-2.5: it was never in the grab)
 ─→ downscaled to 2048 px on the long side, PNG, held in memory; a 256-px thumbnail for the page and history (VIS-2.2, 2.6)
 ─→ a new conversation opens in the bubble at the rectangle's bottom-right corner (BUB-1 rules), the snip as its context
 ─→ the question goes to the model with the image; a cloud model's preview warns it isn't redacted (VIS-3.2)

Wayland: XWayland can't see other apps' windows, so `cursora --snip` takes the screenshot the user copied (VIS-4.2, tweak 1)
```

## Components

| File | Job |
|---|---|
| `src-tauri/src/snip/mod.rs` (new) | `start()`, the frozen grab in memory, `finish(rect)`, `cancel()`; `snip_model()` (pure, VIS-1.2); `too_small()` (pure, VIS-2.3); `prepare()` (crop, downscale, PNG, thumbnail). |
| `src-tauri/src/snip/windows.rs` (new) | Grab the virtual screen: BitBlt from the screen DC (`CAPTUREBLT`), in physical pixels (the app is per-monitor DPI aware). |
| `src-tauri/src/snip/linux.rs` (new) | X11: `GetImage` of the root window through the existing `x11.rs` client. Wayland: the clipboard image (tweak 1). |
| `src-tauri/src/snip/overlay.rs` (new) | The overlay window (label `snip`): created when needed, placed over the virtual screen, on top, no frame, not in the taskbar, focused (on X11 through `bubble::os::focus`). The `snip://` URI scheme serves it the frozen grab as an in-memory BMP. |
| `src-tauri/src/capture/mod.rs` | `Content::Snip { thumbnail, width, height }` with the full image in a field that is never serialized (VIS-2.6). |
| `src-tauri/src/capture/files.rs` | Images among selected files: attached for a vision model, at most 4, downscaled (VIS-2.7). |
| `src-tauri/src/chat.rs` | The first question of a conversation carries its snip or file images; `/snip` runs a snip; the one-time model switch is the conversation's own model (`own_model`), which leaves the default alone (VIS-1.2). The debug Echo model also reports the images it got and their sizes, so end-to-end tests can see an image arrive. |
| `src-tauri/src/skills.rs` | `/snip` joins the commands in autocomplete and `/help`, only for a model that can see (VIS-1.1). |
| `src-tauri/src/hotkey.rs`, `src-tauri/src/lib.rs` | The snip hotkey is registered (not on Wayland, not while paused); `--snip` runs a snip and is ignored while paused. |
| `src-tauri/src/settings.rs` | `snip.maxSide` (2048). |
| `src/Snip.tsx` (new), `src/main.tsx` | The overlay's page: the frozen grab, the dimming, the crosshair and the rectangle with its size. |
| `src/Chat.tsx`, `src/conversation.ts` | The snip in the context row (thumbnail, size, × to remove); the VIS-1.2 hint and its button; "Snip an area instead" does something. |
| `src/HotkeyInput.tsx`, `src/Settings.tsx` | The Snip field comes back; on Wayland, General's shortcut steps gain the `cursora --snip` line (LNX-2.2). |
| `src-tauri/tauri.conf.json` | The CSP lets images come from `snip:` (`http://snip.localhost` on Windows). |

## Data

### Settings
```jsonc
"snip": { "maxSide": 2048 }   // 256 to 8192; also the limit for images from files (VIS-2.2, 2.7). Editable in settings.json only, like the capture limits.
```
`hotkeys.snip` already exists (default Ctrl+Shift+G).

### The snip in a conversation
```jsonc
"content": {
  "kind": "snip",
  "thumbnail": { "mime": "image/png", "base64": "…" },   // ≤ 256 px on the long side: what the page shows and history stores (VIS-2.6)
  "width": 1280, "height": 720                           // the image the model gets
}
```
The full PNG sits beside it in a field that is never serialized, so it reaches neither the page nor the history file. A conversation reopened from history has only the thumbnail, and its follow-up questions send that.

### Images from files (VIS-2.7)
Each `FileItem` that is an image (png, jpg, jpeg, webp, gif, up to 20 MB) gets `image: true` when it's attached; the decoded, downscaled PNGs are held in memory beside the file context, never serialized. History keeps the file list as before.

## Key mechanics

### Start (VIS-1, VIS-3.1)
1. While paused: nothing (the hotkey isn't registered; `--snip` is ignored, like `--ask`).
2. The app in front is read first, as for Ask. If a blocklist rule matches, the bubble opens with the blocked notice ("Cursora is blocked in <app>, so nothing was read."); nothing is grabbed (VIS-3.1).
3. `snip_model(settings)`:
   - the active model can see → go on;
   - else the first other configured model that can see and that local-only allows → the bubble opens with "Snipping needs a model that can see images. **Use <model> for this snip**". The button runs the snip with that model for this one conversation (`own_model`); the default stays;
   - else: "Snipping needs a model that can see images. Add one in **Models**." with a link to the tab.
4. The bubble hides, then about 100 ms later (for the window to be gone from the screen) the virtual screen is grabbed: every monitor, in physical pixels, as BGRA rows in memory.

### The overlay (VIS-2.1, 2.3 to 2.5)
- One window over the whole virtual screen (all monitors). It shows the frozen grab, so nothing on screen changes while the user picks, and transparency isn't needed (an X11 desktop without a compositor would show a transparent window black).
- The page gets the grab from `snip://localhost/screen` (on Windows `http://snip.localhost/screen`): the BGRA rows behind a BMP header, straight from memory, never encoded or written.
- Outside the rectangle the grab is dimmed; the rectangle shows it at full brightness, with its size in pixels. The cursor is a crosshair.
- Mouse up sends the rectangle in CSS pixels; Rust maps it to screen pixels with the overlay's scale factor and origin. On Windows a window over monitors with different scales still maps linearly in physical pixels, so one factor fits all (VIS-2.4; checked by hand, issue 1).
- Esc, a right-click or a rectangle under 8×8 px cancels: the overlay closes, the grab is dropped, nothing opens (VIS-2.3).
- The overlay and the bubble were hidden before the grab, so neither is in it (VIS-2.5).

### The image (VIS-2.2, 2.6)
- `prepare()`: crop the rectangle from the grab, drop the grab, downscale to `maxSide` on the long side (keeping the aspect), encode PNG, make the 256-px thumbnail.
- The bubble opens with a new conversation (an open one is saved to history as usual), placed by BUB-1's rules with the rectangle's bottom-right corner as the cursor.
- `/snip what is this?`: the text after `/snip` is sent with the image as soon as it's taken.
- The first question carries the image; later questions carry it again, since every request sends the whole conversation (BUB-5).
- Nothing is written to disk; the log gets only sizes and times.

### Images from file managers (VIS-2.7)
In `file_context`, when the model can see: up to 4 image files of up to 20 MB are decoded and downscaled to `maxSide`. A file that won't decode, a 5th image, or a model that can't see: metadata only, as now.

### Linux
- **X11:** `GetImage` (ZPixmap) of the root window gives BGRX rows for the whole screen; the overlay covers the root window. Works without a compositor, since the overlay is opaque.
- **Wayland:** XWayland can't see other apps' windows, so the X11 grab would be black. See tweak 1.

## Commands and events
| Name | Kind | Does |
|---|---|---|
| `snip_start(model?)` | command | Starts a snip from the bubble ("Snip an area instead", `/snip`, the one-time switch). |
| `snip_done(rect)` | command | From the overlay: the rectangle in CSS pixels, or `null` to cancel. |
| `snip://localhost/screen` | URI scheme | The frozen grab as a BMP, for the overlay only, while a snip is in progress. |

## Changes to earlier specs' code
- **Spec 01:** the snip hotkey is registered; `--snip` runs a snip and is ignored while paused; the Snip field is back (issue 37).
- **Spec 02:** "Snip an area instead" (SEL-6.2) starts a snip; image files are attached (VIS-2.7, moved from SEL-5.4).
- **Spec 03:** the bubble's snip context; placement at the rectangle's corner.
- **Spec 06:** `/snip` works, and is listed for models that can see.
- **Spec 07:** the blocklist is checked before a grab; the image warning shows for snips.
- **Spec 08:** on Wayland, the shortcut steps gain `cursora --snip`.

## Dependencies
- **`image` 0.25**, with only the `png`, `jpeg`, `webp` and `gif` features: cropping, resizing (triangle filter), PNG encoding, decoding the file formats of VIS-2.7.
- **`base64` 0.22**: `Image.base64`.
- **`windows`**: the `Win32_Graphics_Gdi` feature.
- Linux: nothing new (`x11rb` from spec 08).

## Decisions
1. **Freeze the screen, then pick on the frozen grab**, as Windows' Snipping Tool does. *Rejected:* a transparent overlay over the live screen (black without a compositor on X11; menus and tooltips change under the user; the overlay could land in the grab).
2. **Our own grab, on the `windows` and `x11rb` crates already in use** (about 60 lines). *Rejected:* `xcap` (on Linux it brings xcb, D-Bus and PipeWire build dependencies).
3. **One overlay window over all monitors.** *Rejected:* one per monitor (a rectangle across two monitors, and keeping them in step).
4. **The grab reaches the overlay as an in-memory BMP** through a URI scheme: no encoding, nothing on disk, and the page shows it as a plain `<img>`.
5. **The model's image is never serialized; the thumbnail is.** VIS-2.6 then holds by construction: history and the page only ever see the thumbnail.

## Requirement tweaks (approved, applied)
1. **VIS-4.2, Wayland.** The XDG Desktop Portal *Screenshot* moves to the backlog, for the reasons LNX-2.2's portal did: about 100 lines of D-Bus code that no machine here can run, and Ubuntu 22.04's GNOME portal behaves differently from newer ones. Instead, on Wayland, snip uses the image on the clipboard (`wl-paste --type image/png`): the user takes a region screenshot with the desktop's own tool (on GNOME the Print key opens it, and the region is copied to the clipboard), then presses the bound `cursora --snip` shortcut. Without an image on the clipboard, the bubble says how. This can be tested here with a stand-in `wl-paste`, as spec 08's was.2. **VIS-2.6:** a conversation reopened from history has only the thumbnail, and its follow-up questions send that (the full snip was never stored).
3. **APP-1.3:** while paused, `cursora --snip` is ignored too, as `--ask` is.

## Known limits
- Windows: content shown through hardware overlays or protected video can come out black in the grab.
- Only the app in front is checked against the blocklist (VIS-3.1). A blocked app visible elsewhere in the rectangle isn't.
- Mixed-DPI monitors are covered by reasoning and unit tests only: this PC has one screen (issue 1).
- `// shortcut:` the grab of the whole virtual screen is held during the pick (8 MB for one 1080p screen, about 130 MB for three 4K ones). Upgrade to grabbing per monitor if that bites.

## Test strategy

| Criterion | Test |
|---|---|
| VIS-1.1 | `skills::entries`: `/snip` is listed for a model that can see, and left out otherwise; `/help` matches. |
| VIS-1.2 | `snip_model()`: the active model sees → go; another model sees → offer it; it's a cloud model in local-only → not offered; none → the Models hint. The one-time switch leaves the default model as it was. |
| VIS-2.2 | `prepare()`: a 4000×1000 region → 2048×512 PNG; a small region is not enlarged; the bubble's anchor is the rectangle's bottom-right corner, through BUB-1's placement. |
| VIS-2.3 | `too_small()`: 7×100 and 100×7 cancel; 8×8 doesn't. |
| VIS-2.6 | A conversation with a snip serializes only the thumbnail (≤ 256 px), never the full image; the snip code uses no file API (a source check, as VOC-1.7 plans). |
| VIS-2.7 | `file_context` with five images and a vision model → four attached, each ≤ `maxSide`; a broken image → metadata; no vision → metadata only. |
| VIS-3.1 | `start()` on the fake platform with a blocked app in front → the blocked notice, and the grab is never called. |
| VIS-3.2 | A conversation with a snip, for a cloud model → the preview's image warning. |

Real screens:
- Windows: a 10×10 grab of the virtual screen returns 10×10 pixels (runs in CI).
- X11 (with `CURSORA_XVFB`): a window of a known colour, grabbed through its rectangle, gives that colour.

Manual: VIS-2.1, 2.4, 2.5, 4.1 and 4.2.
- End to end on Windows and in WSL: the hotkey, `/snip`, a drawn rectangle (xdotool or SendInput drags), Esc, a too-small rectangle, the bubble's corner, the image reaching Echo (which will report each image's size), a blocked app, and the hint without vision.
- By the user: several monitors and mixed DPI (issue 1), and a real Wayland desktop (issue 2).

## Changes during implementation
- **2026-10-14, build:**
  - **A conversation can have its own model.** `chat::switch_model` turned out to change only what the header shows; the default model answered. So `Conversation.own_model` holds the snip's one-time switch, the answer path uses it before the default (falling back with a note if it's gone), and the header's switcher clears it.
  - **One rectangle type:** the snip uses `bubble::Rect` (screen pixels), not a second one.
  - **`fit()` clamps the long side** to 256–8192 px for every caller, so a typo in `snip.maxSide` can't send a 1-px or a huge image.
  - **Echo can see when asked:** `CURSORA_ECHO=vision` (or `cloud,vision`) makes it a vision model, and its answer lists each image's size, for the end-to-end runs.
  - **Image files are attached right after the capture** (`files::attach_images`, in the Ask flow), so `capture()` stays free of decoding. `// shortcut:` decoding up to 4 images comes before the bubble shows; move it after if large photos feel slow.
  - **`wl-paste` is one helper** for spec 08's PRIMARY text and the copied screenshot.
  - **The X11 grab test calls the X11 grab directly:** WSL also has a Wayland display, which `grab()` would pick.
- **2026-10-14, review:**
  - **One snip at a time:** a double click or X11 repeating a held hotkey could start two snips; the second overwrote the grab and, failing to open a second overlay, threw it away, so the visible overlay picked nothing. A flag now holds from the start until the bubble opens or the snip is cancelled.
  - **The answering model is chosen in one function:** a skill's own model, else the conversation's own, else the default. This also fixed a skill without a model of its own (`/explain` on a snip) going to the default model instead of the one that can see.
  - **The crop is the rectangle's overlap with the grab** on every edge, not only the right and bottom.
