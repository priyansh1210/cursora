# 10: Vision snip (VIS)

## Scope
- **In:** capturing a screen region as an image and asking a vision-capable model about it. This covers images, videos, scanned PDFs and any other text that can't be selected.
- **Out:**
  - Local OCR for text-only models (backlog)
  - Screen recording or video

Snip exists **only** when the active model can read images. Everything else in Cursora works without it.

## Requirements

### VIS-1 Availability
- VIS-1.1 (auto) WHILE the active model is not vision-capable (LLM-5.2) THE SYSTEM SHALL hide `/snip` from autocomplete. Any other path that would start a snip SHALL show the hint below instead (VIS-1.2).
- VIS-1.2 (auto) WHEN a snip is requested and the active model lacks vision:
  - IF another configured model has vision (and is allowed by local-only mode) THE SYSTEM SHALL offer "Use <model> for this snip" as a one-time switch.
  - Otherwise THE SYSTEM SHALL explain that a vision model is needed and link to the Models tab.

### VIS-2 Snipping
User story: As a user, I want to draw a box around anything on screen and ask about it.
- VIS-2.1 (manual) WHEN the user presses the snip hotkey (default **Ctrl+Shift+G**), runs `/snip`, or picks "Snip an area instead" (SEL-6.2) THE SYSTEM SHALL show a dimmed overlay across all monitors with a crosshair. The user drags a rectangle; Esc cancels.
- VIS-2.2 (auto) THE captured region SHALL be encoded as PNG, downscaled so the longest side is ≤ 2048 px (configurable), and attached to a new conversation. The bubble SHALL open next to the rectangle's bottom-right corner, using the placement rules of BUB-1.
- VIS-2.3 (auto) A rectangle smaller than 8×8 px SHALL be treated as a cancel.
- VIS-2.4 (manual) Region coordinates SHALL map correctly to screen pixels on multi-monitor and mixed-DPI setups.
- VIS-2.5 (manual) Cursora's own overlay and bubble SHALL NOT appear in the captured image.
- VIS-2.6 (auto) Snip images SHALL be held only in memory. WHERE history is on, only a thumbnail (≤ 256 px) SHALL be stored with the conversation. (Amended 2026-10-14: a conversation reopened from history has only the thumbnail, and its follow-up questions send that.)
- VIS-2.7 (auto) WHERE files selected in a file manager (SEL-5) include images (png, jpg, jpeg, webp, gif) AND the active model supports vision THE SYSTEM SHALL attach them, downscaled so the longest side is ≤ 2048 px. At most 4 images are attached. Otherwise they are sent as metadata only. (Moved from SEL-5.4.)

### VIS-3 Privacy
- VIS-3.1 (auto) WHEN a snip is requested while the foreground app is blocklisted (PRI-2) THE SYSTEM SHALL refuse and show the blocked notice.
- VIS-3.2 (auto) WHEN a snip is sent to a cloud provider THE preview SHALL warn that images are not redacted (PRI-3.8).

### VIS-4 Linux
- VIS-4.1 (manual) On X11, snip SHALL work as on Windows.
- VIS-4.2 (manual) On Wayland, THE SYSTEM SHALL take the snip from the clipboard (`wl-paste --type image/png`) instead of Cursora's overlay: the user copies a region with the desktop's own screenshot tool, then runs `cursora --snip`. Without an image on the clipboard, the bubble SHALL say how. (Amended 2026-10-14: the XDG Desktop Portal *Screenshot* interface moved to the backlog, since no machine here can test it.)

## Open questions
- _(none)_
