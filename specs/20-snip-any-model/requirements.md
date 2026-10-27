# 20: Snip with any model (OCR)

Status: **Approved**¹ (2026-10-28).

## Scope
- **In:** snipping with a model that can't see images. The text in the snipped region is read on this computer, by Windows' own text recognition or by Tesseract on Linux, and the model gets that text with the snip.
- **Out:**
  - Pictures, charts and handwriting for models that can't see: they still need a model that can (VIS-1.2)
  - Choosing recognition languages in Cursora: Windows uses the ones installed with Windows, Tesseract its English data
  - Image files selected in a file manager (VIS-2.7): they still go only to models that can see
  - Bundling Tesseract with the Linux packages

Update #3 from the list of 2026-10-28 ([improvements batch 2](../../memory/improvements-batch-2.md)).

## Requirements

### OCR-1 Read the snip as text
User story: As a user with a model that can't see images (such as the free download), I want to snip an error message, a scanned page or a slide and still ask about it.
- OCR-1.1 (auto) WHILE the model that will answer can't see images AND text recognition is available on this computer, snipping SHALL work as it does for a model that can see (VIS-2.1): `/snip` is listed, "Snip an area instead" is offered, and the snip hotkey takes a snip. (VIS-1.1 and VIS-1.2 amended: their notice and offer apply only when text recognition isn't available.)
- OCR-1.2 (auto) WHEN such a snip is taken THE SYSTEM SHALL read the text in the region and give it to the model with the snip, marked as read by text recognition, which can make mistakes. The image SHALL stay in the conversation, so a model that can see still gets it.
- OCR-1.3 (manual) THE bubble SHALL show that the snip was read as text, with the number of characters and Show more to see the text.
- OCR-1.4 (auto) IF no text is found in the region THE bubble SHALL say so, and point to a model that can see images: the one-time switch of VIS-1.2 when one is configured, else Settings → Models.
- OCR-1.5 (auto) Text recognition SHALL run on this computer only, and the image SHALL never be written to disk (VIS-2.6). On Linux it reaches Tesseract through its standard input.
- OCR-1.6 (manual) On Linux without Tesseract (`tesseract-ocr`), snipping with a model that can't see SHALL behave as before (VIS-1.2), and the notice SHALL say that installing `tesseract-ocr` lets any model use snips.

## Defaults taken under the go-ahead of 2026-10-28
1. **The system's own text recognition**: Windows.Media.Ocr, part of Windows 10 and 11, nothing to download. On Linux, Tesseract when it is installed; Cursora doesn't ship it.
2. **The text goes with the snip, not instead of it**: the image stays, so switching to a model that can see in the same conversation still works.
3. **Only when the model can't see**: a model that can see gets the image alone, as before, so its answers don't change.
4. **No text found**: say so and offer a model that can see, rather than send an empty snip.

## Open questions
None.

¹ Approved under the user's go-ahead of 2026-10-28 to design, build and merge updates 1 and 3 to 9, with the user reviewing everything afterwards. The choices made for the user are listed above.
