# 17: Save and pin answers (SAV)

## Scope
- **In:** two new buttons on every finished answer. **Pin** keeps the answer on screen in a small note. **Save** writes it to a Markdown file in a folder the user chooses.
- **Out:**
  - Pinned notes coming back after a restart (the user's choice): Save keeps an answer
  - Saving straight into Notion, OneNote or Google Docs, which need accounts and their own APIs. Obsidian works, since its vault is a folder.
  - Saving a whole conversation as one file: History keeps conversations
  - Editing a pinned note
  - Keyboard shortcuts for Pin and Save: the buttons are reachable with the keyboard (BUB-8.1)

Idea #10 from the improvement plan ([product direction](../../memory/product-direction.md)), started at the user's request of 2026-10-26.

## Requirements

### SAV-1 Pin an answer
User story: As a user, I want to keep an answer in view (a command I'm typing, a translation I'm copying from) while I go on working and asking other things.
- SAV-1.1 (manual) Each finished answer, in the bubble and in the expanded window, SHALL offer **Pin** next to Copy. Pin SHALL open the answer in a small note beside the bubble, shown as in the bubble (BUB-4.2), with its question as the title. The note stays on top of other windows, can be moved and resized, and has **Copy** and **Close**.
- SAV-1.2 (manual) A pinned note SHALL stay open when the bubble closes or a new conversation starts, and several notes can be open at once. The notes close when Cursora quits, and don't come back after a restart.
- SAV-1.3 (auto) Pin SHALL be offered only on a finished answer: not while it streams, and not on an error. The note keeps the answer as it was pinned, so a later Regenerate doesn't change it.
- SAV-1.4 (manual) A pinned note SHALL follow the theme (BUB-8.2), and screen readers SHALL read its answer and buttons (BUB-8.1).

### SAV-2 Save an answer
User story: As a user, I want to keep a good answer as a file I can find, search and open in my notes app.
- SAV-2.1 (manual) Each finished answer SHALL offer **Save** next to Pin. Save writes the file, the button then says **Saved**, and **Show in folder** opens the folder with the file selected.
- SAV-2.2 (auto) THE file SHALL hold the date and time, the app the text came from, the model, the user's question (or the command, such as `/explain`), and the answer as Markdown, as shown in the bubble (with hidden values put back, PRI-3.3). It SHALL NOT hold the selected text or files, since they may be private. History keeps them.
- SAV-2.3 (auto) THE file name SHALL be the date and the first words of the question, made safe for Windows and Linux: none of `\ / : * ? " < > |`, no dot or space at the end, at most 80 characters. An existing file SHALL never be overwritten: a number is added instead, as in `… (2).md`.
- SAV-2.4 (auto) THE folder SHALL be `Cursora` in the user's Documents folder by default, made when the first answer is saved. IF the file can't be written THE bubble SHALL say why in plain words, and the answer stays in the bubble.
- SAV-2.5 (manual) Settings → General SHALL show the saved-answers folder, with **Change…** and **Open**.
- SAV-2.6 (auto) Save SHALL work while "Save history" is off, since the user asked for that one file. It writes nothing else (BUB-9.3 amended).

## Choices made with the user (2026-10-26)
1. **Pinned notes close when Cursora quits** (the user's choice). Save keeps an answer for later.
2. **A saved file holds the question and the answer, not the selected text** (the user's choice). The selected text may be private, and History keeps it.
3. **The default folder is Documents\Cursora** (`~/Documents/Cursora` on Linux), so the files are easy to find. Change… can point it at an Obsidian vault.

## Open questions
None.
