# 13: Write back into apps (WRT)

## Scope
- **In:** putting an answer into the app the bubble opened over: **Replace** swaps it in for the selected text, **Insert** puts it where the cursor was when nothing was selected. Skills whose answer is meant for that ("replacement skills"), and making answers clean enough to paste.
- **Out:**
  - A diff view of the old and new text: the answer itself shows the new text, and the selection is the old one
  - Cursora's own undo: the app's own Ctrl+Z undoes a paste
  - Writing into apps on Wayland, where Cursora can't press keys in other apps
  - Writing into a conversation reopened from History, which is no longer connected to its app

Ideas #1 and #4 from the improvement plan ([product direction](../../memory/product-direction.md)), built at the user's request of 2026-10-26.

## Requirements

### WRT-1 Replace
User story: As a user, I want to fix or rewrite the text I selected and put the result back in one click, without copying and pasting.
- WRT-1.1 (manual) WHEN an answer is complete and the conversation's context is text selected in an app THE answer SHALL offer **Replace**, first among its actions. Choosing it SHALL hide the bubble, bring that app back to the front and paste the answer over the selection. Ctrl+Z in the app then undoes it, as with any paste.
- WRT-1.2 (auto) THE text pasted SHALL be the answer as plain text: no Markdown marks (emphasis, headings, code fences, link syntax, quote marks at line starts). Its line breaks and list dashes stay.
- WRT-1.3 (manual) WHEN the app isn't back in front within one second THE SYSTEM SHALL NOT paste. The bubble then says the answer wasn't put in, and the answer stays in the bubble to copy.

### WRT-2 Insert at the cursor
User story: As a user writing in an email, a chat or a form, I want to ask Cursora to write something and have it typed where I was.
- WRT-2.1 (manual) WHEN an answer is complete, nothing was selected, and the bubble opened over an app THE answer SHALL offer **Insert**, which works as Replace and puts the answer where the cursor was.

### WRT-3 Where it's offered
- WRT-3.1 (auto) Replace and Insert SHALL NOT be offered when:
  - the app is blocked (PRI-2), or runs as administrator while Cursora doesn't (SEL-6.1)
  - the app is a terminal or another app on the "Ctrl+C may stop a process" list (SEL-4.3): a pasted line break can run a command there
  - the context is files or a snip
  - the session is Wayland
  - the conversation was reopened from History
  
  **Copy** is always there instead.

### WRT-4 Safety
- WRT-4.1 (auto) THE user's clipboard SHALL be restored after the paste, as after a capture (SEL-3.1). If it can't be saved exactly, nothing is pasted, and the bubble says so.
- WRT-4.2 (auto) THE paste SHALL go only to the window the bubble opened over: if another window is in front, nothing is pasted (WRT-1.3).

### WRT-5 Answers made for replacing
User story: As a user, I want "fix this" to give me just the fixed text, ready to put back.
- WRT-5.1 (auto) A skill file MAY say `output: replace`. Its prompt then tells the model to reply with the new text only: no introduction, no quotes around it, no explanation.
- WRT-5.2 (auto) THE built-in `/fix`, `/rewrite`, `/simplify` and `/translate` SHALL be replacement skills. (`/fix` no longer adds a list of what changed; asking "what changed?" afterwards gives it.)

## Defaults taken while the user was away (2026-10-26)
The user asked for ideas #1 to #5 to be built overnight and their choices listed for review. Each can be changed:
1. **Paste, not typing:** the answer goes in through the clipboard and Ctrl+V, which every app understands. Typing it key by key would be slow, and would break on long text and other keyboard layouts.
2. **Terminals get Copy only** (WRT-3.1), for safety.
3. **`/fix` gives only the fixed text** (WRT-5.2), so Replace puts in exactly that.
4. **The bubble closes after Replace or Insert**, as if the hotkey had been pressed. The conversation stays in History.
5. **Replace is offered on every finished answer**, not only those of replacement skills, so "make this more formal" typed as a question can be put back too.

## Open questions
None.
