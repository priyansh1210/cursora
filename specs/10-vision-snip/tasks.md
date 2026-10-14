# 10: Vision snip (tasks)

Status: **In progress**.

- [x] 1. `snip/mod.rs`, the pure parts: `snip.maxSide` in settings, `too_small()`, `snip_model()`, `prepare()` (crop, downscale, PNG, the 256-px thumbnail); `image` and `base64` added (tests first). Covers VIS-1.2, 2.2, 2.3.
- [x] 2. `Content::Snip` with the full image never serialized; the first question carries the conversation's images; Echo reports the images it got (tests first). Covers VIS-2.6, 3.2.
- [x] 3. The grab: `snip/windows.rs` (BitBlt of the virtual screen) and `snip/linux.rs` (X11 `GetImage` of the root window), each with a test on a real screen. Covers part of VIS-2.4, 4.1.
- [x] 4. The overlay: one window over the virtual screen, the `snip://` scheme serving the frozen grab as a BMP, `Snip.tsx`, `snip_done` mapping the rectangle to screen pixels; `start()` with the blocklist and vision checks; the bubble at the rectangle's corner (tests first for `start()` on the fake platform). Covers VIS-1.2, 2.1, 2.3, 2.5, 3.1.
- [x] 5. The ways in: the snip hotkey (not on Wayland, not while paused), `--snip` (ignored while paused), `/snip [question]`, "Snip an area instead"; `/snip` listed only for a model that can see; the Snip field back (issue 37); `cursora --snip` in General's Wayland steps. Covers VIS-1.1, 2.1, APP-1.3.
- [x] 6. Wayland: the clipboard image through `wl-paste --type image/png`, with a timeout and a size cap, and the hint when there's none. Covers VIS-4.2.
- [x] 7. Images from file managers: up to 4, downscaled, for a model that can see (tests first). Covers VIS-2.7.
- [x] 8. The bubble: the snip in the context row (thumbnail, size, ×); the VIS-1.2 hint with its one-time switch and the Models link.
- [x] 9. End-to-end runs on Windows and in WSL; `/ponytail-review` (2026-10-14; its 4 findings fixed: one snip at a time, the answering model chosen in one tested function, the crop clipped on every edge, an unused derive removed).

## Manual checklist
"WSL" is Xvfb + Openbox with the Linux debug build; its Wayland session is simulated. Windows and WSL runs on 2026-10-14, with Echo posing as a model that can see.

- [x] VIS-2.1 The snip hotkey, `/snip` and "Snip an area instead" show the dimmed overlay over every monitor with a crosshair; dragging picks a region; Esc cancels. Windows ✓ the hotkey; the overlay over the virtual screen (1920×1080); a 400×150 drag; Esc and a 4×40 drag cancel (auto-verified) X11 ✓ the hotkey, `/snip describe it` (asked at once); Esc, a right-click and a 5×40 drag cancel; holding the key makes one snip (auto-verified in WSL). "Snip an area instead" shows; its command is the one the one-time switch runs, which was clicked.
- [ ] VIS-2.4 The region maps to the right screen pixels with several monitors and mixed DPI. ☐ (one monitor here: issue 1; the mapping is unit-tested at scale 1.5 with a monitor left of the primary)
- [x] VIS-2.5 Neither the overlay nor the bubble is in the image. Windows ✓ the thumbnail shows the test window's text, undimmed (auto-verified) X11 ✓ the thumbnail shows the terminal region (auto-verified in WSL)
- [x] VIS-4.1 On X11, snip works as on Windows. ✓ (auto-verified in WSL; real Ubuntu: issue 2)
- [ ] VIS-4.2 On Wayland, a screenshot copied with the desktop's tool, then `cursora --snip`, opens the bubble with it; with no image copied, the bubble says how. WSL ✓ with a stand-in `wl-paste`: a 640×360 copy opens a centred bubble; nothing copied gives the steps. A real compositor: issue 2.
