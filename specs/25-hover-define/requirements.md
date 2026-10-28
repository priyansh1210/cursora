# 25: Hover to define (HOV)

Status: **Approved**¹ (2026-10-28).

## Scope
- **In:** a **Define** hotkey. Point at a word anywhere on the screen, in any app, a PDF or a picture, and press it: the bubble opens at the pointer with the word's meaning, without selecting anything.
- **Out:**
  - Defining by resting the pointer alone, with no key: it would mean reading the screen all the time
  - Holding Alt as the trigger: Alt opens menus in many apps when it is let go
  - A small tooltip of its own: the answer comes in the bubble, where follow-ups, Translate and Pin already work
  - Phrases: one word, the one under the pointer
  - Wayland: hotkeys aren't registered there (LNX-2.2), and Cursora can't read other apps' pixels (VIS-4.2)

Update #9 from the list of 2026-10-28 ([improvements batch 2](../../memory/improvements-batch-2.md)).

## Requirements

### HOV-1 Define the word under the pointer
User story: As a student reading a PDF or a web page, I want to point at a word I don't know and press one key, so I get its meaning without selecting it or leaving the page.
- HOV-1.1 (auto) Settings → General and the first-time setup SHALL have a **Define** hotkey, **Ctrl+Alt+G** by default, checked like the others (SEL-1.2). IF it can't be registered THE SYSTEM SHALL say so, and the Ask and Snip hotkeys SHALL still work.
- HOV-1.2 (auto) WHEN the Define hotkey is pressed THE SYSTEM SHALL read the text in a band of the screen around the pointer (800 × 120 pixels) with the system's text recognition (spec 20), in memory only (OCR-1.5), and take the word whose box holds the pointer, without the punctuation around it.
- HOV-1.3 (manual) THE bubble SHALL open at the pointer with the word as its context, and ask `/define` at once, as if typed, so "What will be sent" confirming still applies (PRI-3.7). An open bubble is closed first, so pointing at another word and pressing again defines that one.
- HOV-1.4 (auto) IF there is no word under the pointer, or text recognition isn't available on this computer, THE bubble SHALL say so, with its input ready for a question.
- HOV-1.5 (auto) In an app on the blocklist (PRI-2.1) nothing SHALL be read, as for Ask and Snip; in an app on the off list (OFF-1.1) the Define hotkey SHALL pass through like the others.
- HOV-1.6 (auto) Replace and Insert (spec 13) SHALL NOT be offered for a word read from the screen: nothing is selected in the app.

## Defaults taken under the go-ahead of 2026-10-28
1. **A hotkey, Ctrl+Alt+G**, next to Ask (Ctrl+G) and Snip (Ctrl+Shift+G). On keyboards where AltGr+G types a character (Czech, Slovak, Hungarian: "]"), this takes it; those users can change the key or put an app on the off list.
2. **Reading the screen, not the app**: the system's text recognition from spec 20 works the same in every app, PDF viewer, picture and video, on Windows and X11, where asking each app for the word under the pointer (UI Automation) works only in some.
3. **The bubble, with `/define`**: the built-in skill gives a one-line meaning that fits the context and an example. Asking it as if typed keeps "What will be sent", smart routing (spec 24) and History as they are.
4. **One word**: the one under the pointer, with a few pixels of slack around it.

## Open questions
None.

¹ Approved under the user's go-ahead of 2026-10-28 to design, build and merge updates 1 and 3 to 9, with the user reviewing everything afterwards. The choices made for the user are listed above.
