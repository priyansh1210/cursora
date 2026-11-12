# 27: Fill forms (design)

Status: **Approved**¹ (2026-10-30; any kind of box, 2026-11-13²). Implements [requirements.md](requirements.md). It reads the form through UI Automation, as the capture reads a selection (SEL-2.2), and types like Replace and Insert, but one letter at a time.

## Overview
```
Ctrl+G, nothing selected → bubble with Insert (WRT-3.1) → "Fill this form" = ask("/fill")
ask("/fill") → refusal? (not Insert, Linux) → Cursora's own reply (FIL-2.6)
             → no detail with a value → own reply + empty plan (the view offers My details)
             → cloud model and confirming on → Asked::Confirm (PRI-3.7)
             → a local turn, streaming; on a worker thread:
                 fill::read(window): the text boxes (FIL-2.2)
                 fill::sort: filled / left for the user / matched by name / to ask (FIL-2.2, 2.3)
                 fill::requests (one per box) + redaction → the model, temperature 0, replies collected (FIL-2.3, 2.4)
                 fill::answer per reply → fill::plan → turn.fill, turn done (FIL-2.5)
Fill (ticked rows) → fill command → targets with today's values (FIL-3.4)
             → bubble::bring_back(window) (WRT-4.2)
             → fill::type_into: read again, check index and label, hide the bubble,
               per box: focus, frame, type letter by letter, stop on Esc or another window (FIL-3.1 to 3.3)
```

## Components
| File | Change |
|---|---|
| `src-tauri/src/fill/mod.rs` | New. The pure parts: `Detail`, `FORM`, `Field`, `usable`, `sort`, `sensitive`, `someone_else`, `messages`, `requests`, `answer`, `match_boxes`, `Plan`, `targets`, `typed`, `summary`, `report`; `refusal`. `read` and `type_into` call the Windows module, and on Linux say it works on Windows for now. |
| `src-tauri/src/fill/windows.rs` | New. UI Automation: the window's text boxes, in its documents if it has any, with one cached `FindAll`; focus, the frame (a layered, click-through, topmost Win32 window shaped by a region), and typing through `SendInput` with `KEYEVENTF_UNICODE`. |
| `src-tauri/src/settings.rs` | `my_details: Vec<fill::Detail>`, as the tab saves them, and `more_details: String`, the More details box as typed, which only the tab reads (FIL-1.1). |
| `src-tauri/src/chat.rs` | `Turn.fill: Option<fill::Plan>`; `Conversation.fill: bool` (Insert on Windows), never read back from History; `/fill` in `ask` (`fill_command`) and in `send_preview`, which becomes `async` so it can read the form off the main thread; the `fill` command. |
| `src-tauri/src/skills.rs` | `FILL`, a system command in `/help` and the command list (FIL-2.6). |
| `src-tauri/src/prompt.rs` | `defuse` also breaks `</form_field`. |
| `src/MyDetails.tsx` (new), `src/Settings.tsx`, `src/icons.tsx`, `src/useSettings.ts` | The My details tab: the form by sections, then More details; `details` makes the list it saves. |
| `src/Chat.tsx`, `src/conversation.ts` | **Fill this form** under "Nothing was selected"; a fill turn's plan with tick boxes and **Fill**, or **Open My details** (the tab). |

## Key mechanics

### My details (FIL-1)
- The tab is a `<form>`: a labelled box for each name of `SECTIONS` in `MyDetails.tsx`, under its section's heading, then More details. `fill::FORM` has the same names in the same order, and a JS test compares the two.
- When a box loses focus, the form is read with `FormData`. `moreDetails` gets the text as typed, so nothing written there is lost or reordered. `myDetails` gets `details`: the boxes with a value, trimmed; "Address", the two address lines joined by ", " (a Google Form often has one address box); then each line of More details split at its first colon, so a link after it stays whole, but not a bare link, whose colon is its scheme's. Everything after the tab, Rust and the plan's view, reads `myDetails` as before, so the lines are split in one place.
- `shortcut:` the form's boxes can't be renamed or removed; a detail it lacks goes in More details. Add a box to the form when one comes up often.

### Reading the form (FIL-2.2)
- `ElementFromHandle(window)`, then the `Document` elements under it. Where there are any (browsers, Electron apps), only their text boxes count, so the address bar and the browser's own boxes are left out. Without one, or with no boxes in it, the whole window counts (a dialog).
- One `FindAllBuildCache(Descendants, ControlType == Edit)` per root caches Name, HelpText, IsPassword, IsEnabled, IsKeyboardFocusable and the Value pattern's Value and IsReadOnly. A disabled, unfocusable or read-only box is not a box to fill, so it is dropped before numbering. So is a box the user can't see: under 4 pixels wide or high, or wholly left of the window (a page hiding a box to collect details, as with browser autofill). A box below the fold still counts, as focusing it scrolls to it.
- A box's label is its Name, or its HelpText (the placeholder) when it has none, without the colon dialog boxes end labels with.
- Chrome builds its accessibility tree once a UI Automation client asks, so a read that finds no box at all is tried once more after 500 ms.
- **Any kind of box (FIL-2.7, 2026-11-13).** One `FindAll` with an OR condition (Edit, ComboBox, List, RadioButton, CheckBox) keeps the form's order. Each becomes a `Field` with a `Kind`:
  - an Edit is `Text`, or `Date(order)` when it has Spinner children whose RangeValue maxima are 12, 31 and more (Chrome's and Edge's `input type=date`, in the locale's order), filled when a part isn't 0 (its Value is empty either way), and labelled by its named group if it has one (Google Forms names the box "Date" and its group "Date of birth"); other spinners (a time box) make it `Other`, left for the user;
  - a ComboBox with an Edit child is skipped (the Edit is read); one with the Text pattern takes typing (`input list=…`): `Text`; else it is a `List` (a `select`), filled when its Value isn't a placeholder; its options show only when it opens, so they are read when filling;
  - a List with the Selection pattern is a `List` too (a Google Forms drop-down), its visible ListItems its options, filled when a selected item isn't a placeholder;
  - RadioButtons and CheckBoxes are grouped: each one's group is its nearest ancestor, up to 4 levels, that is a named Group or List (a fieldset, an ARIA radiogroup or list), else its parent. Buttons next to each other in the form's order with the same group are one question (`Choice` or `Checks`), its label the group's name, else the text just before its first button, its options the buttons' names. A question with an option chosen or ticked is filled.
- Probed in Chrome on 2026-11-13 with a test page (native and ARIA controls, logging the page's own `change` events): radio groups and check-box groups expose their question as the group's name; a `select`'s options aren't in the tree until it opens; `SelectionItem.Select` on a radio, `Toggle` on a check box, `Expand` then `Select` on a `select`'s option, and `Invoke` on a Google-style list's chosen item then `Select` on an option all changed the page's state and fired its events. A date box ignored `SetValue` and RangeValue, but took its digits typed into its first part. Read (only) on four public Google Forms the same day: text boxes, radio questions with their question as label, drop-downs with their options readable before opening, date boxes, and the "Record my email" check box (a lone one, left). Their required boxes end with "Required question", which no name matched until normalizing dropped it.
- `shortcut:` at most 80 boxes go to the model; a longer form gets its first 80. Send them in parts if someone fills forms that long.

### Matching (FIL-2.3)
- `usable` adds what can be worked out (FIL-2.10): First name and Last name from Full name's first and last word, when Full name has two words or more and they aren't saved; Full name from both when it isn't.
- **Common names (FIL-2.8).** `common` drops leading words that only ask ("please", "enter", "what is", "your", …) from the normalized label, then looks the rest up in `COMMON`, a fixed list of whole labels per detail of the form (English and Hindi). Only whole labels count, so "Company name" or "Bank name" never becomes a name. It runs after `sensitive`, and before `someone_else`, so "Father name" finds a saved Father's name. A common name of a detail without a value is left without asking, like the form's own names.
- `sort` goes through the boxes in order: one with text is counted as filled; a password or sensitive box is left for the user; a check-box question with one option, or a box of kind `Other`, is left; one whose normalized label equals a detail's normalized name, or is a common name of it, matches it; one named like a box of the My details form with no value, or about someone else, is left for the user (asked, the built-in 1.5B model put the full name in "First name"); the rest are asked.
- Normalizing: lower case, letters and digits only, words joined by one space, a trailing "required", "required question" (Google Forms) or "required to answer" (Microsoft Forms) dropped, and a leading question number ("1."). The plan shows the label without those marks. "Full Name *" and "full name (required)" both become "full name".
- `sensitive` looks at the label and the placeholder: password, passcode, OTP, one-time, CVV, CVC, captcha, security code, credit card, debit card, card number (but not with Aadhaar, PAN, voter or ration), and PIN unless it's "PIN code" or "pincode" (the Indian postal code).
- `someone_else` looks for whole words: father, mother, parent(s), guardian, spouse, husband, wife, emergency, reference, referee, nominee, sibling, and पिता, माता, अभिभावक. The check comes after the match by name, so a detail the user saved as "Father's name" still fills that box.
- **One request per box**: a system message with the rules, and a user message with the detail names as a list, then `<form_field>` with the box's label, its placeholder as a hint, and a question's options (at most 20, each cut at 60 characters) (cut at 200 characters, defused). No persona, About you or language: the reply is data for Cursora, not an answer. Redaction runs over all the requests with one numbering.
- `answer` takes the reply's first line without `*`, backticks, quotes, a leading "- " and a trailing full stop; it counts when it is the name of a usable detail or a common name of one, else when exactly one detail's name is in it as whole words and "none" isn't (FIL-2.11).
- The rules also say that an age or a year is not a date of birth: both models put "Age" in Date of birth.
- The model is the conversation's (a snip's one-time switch, else the default), at temperature 0. The replies are collected, not streamed into the bubble. Stop cancels the one running and asks no more; a failed request asks no more, and the boxes matched so far stay.
- "What will be sent" shows the system message once, then each box's user message.

### The plan (FIL-2.5)
`Plan { rows: [{ index, field, detail, value }], left: [label], filled: count }`. `value` is the fitted value the view shows (FIL-2.12); `save` clears it before History records the conversation, so History never has a value. A row whose value doesn't fit its box (`fit` is None) goes to the boxes left. The turn is Cursora's own (`local`), so it is never sent to a model later, and Regenerate doesn't apply. Its answer is one line saying what was found; a model failure goes in the turn's note.

### Typing (FIL-3)
- `targets(plan, ticked, details)`: each ticked row with its detail's value from the settings now, through `typed`. A detail that's gone or empty drops the row (FIL-3.4).
- `typed`: CRLF, CR and LF become ", ", other control characters go, so the Enter and Tab keys are never sent (FIL-3.2).
- **Fitting (FIL-2.10)**: `fit(field, value)` gives what to do: `Type(text)` for a text box (a date reformatted where the label or hint shows a format), `Type(digits)` for a date box (its parts in its order), `Pick(options)` for a question or a list whose options are known (`choose`), `Find(value)` for a list whose options show only when it opens; None when nothing fits. It runs when planning, for the value shown, and again on the box as read when filling.
- `date` reads "15/01/2000" (day first, as in India, unless the second number is over 12), "2000-01-15", "15 Jan 2000" and "January 15, 2000"; a year must have four digits.
- `type_into` reads the boxes again and keeps a target only where the box at its index has the same label and no text, since the page may have changed. With none left, it fails before hiding the bubble, so the bubble shows why.
- A question: `SelectionItem.Select` on the radio button, or `Toggle` on each unticked check box to tick; a list: `Expand` (or `Invoke` on its chosen item, for a list without ExpandCollapse), its ListItems read (under it, else the ones in the window that weren't there before it opened), `choose`, `Select`, and `Collapse` if still open. The frame goes around each button or list. A choice that doesn't take (not selected after) counts as missed.
- `shortcut:` a list whose options open where the window's tree doesn't show them (a canvas, another window) is missed; read the popup window too if a real form needs it.
- Per text or date box: `SetFocus`; wait up to 500 ms for UI Automation to report it focused, else skip it; the frame around its rectangle (3 px wider each side, 3 px thick, Cursora's green); then each UTF-16 unit as a `KEYEVENTF_UNICODE` key press, 12 ms apart, pumping the frame's messages while waiting; 150 ms between boxes.
- While typing, a low-level keyboard hook takes Esc, so the app never gets it (in a dialog or a web pop-up, Esc would close the form); before every letter, Esc pressed or another window in front stops it (FIL-3.3). Ctrl, Shift, Alt and Win are waited for first, up to 300 ms, as a held Ctrl would turn letters into shortcuts.
- The result, "Filled 5 boxes. Check them before you send the form.", goes back to the view; the bubble is hidden by then, so the expanded window shows it and the log has the counts.

## Decisions
1. **Our own matching, not the browser's autofill**: it works in any app, and with any labels the model understands (Hindi ones, "Your mobile no.").
2. **Labels to the model, values never**: so the details stay on the computer, whatever model answers. The privacy rules still apply to the labels, which come from the screen.
3. **Exact names first, without a model**: the obvious boxes are always right, and a form whose labels are all detail names needs no request.
4. **Typing, not pasting**: the clipboard is never touched, and the letters show one by one, as the user asked. Pasting into each box would also need a wait per box for the app to read it.
5. **A native frame, not a webview**: a webview window takes about a second and 50 MB to appear and may take the focus; a region-shaped Win32 window does neither.
6. **The bubble hides while typing**, so it covers no box. A failure found before typing shows in it.
7. **"What will be sent" reads the form too** (now `async`): it shows the exact request, as for any question. It asks Send before the form is read, since `ask` can't wait for UI Automation on the main thread; if every box then matches by name, nothing is sent.
9. **Common names and fitting by rules, the model last** (2026-11-13). Scored with `bench` (an ignored test, against a real llama-server) on 48 labels from real forms. Before: Qwen2.5 1.5B 33 right, 7 wrong ("Your Name" → First name, "Postal code" → Country, "पता" → Last name); Qwen3 4B 39 right, 4 wrong. After: 1.5B 46 right, none wrong; 4B 47 right, none wrong. With 20 more labels and questions not in the list (68 in all): 1.5B 62 right, 1 wrong ("What should we call you?" → First name); 4B 64 right, none wrong. The 1.5B takes about 4 s for the 68, the 4B 22 s, on the processor. A wrong detail typed into a box is worse than a box left, so the common names cover the usual boxes and the model gets only the rest.
10. **UI Automation's own patterns pick options, not clicks or keys**: they change the page's state as a click does (probed above), need no mouse, and work in a dialog box too.
8. **One request per box, not one for the form** (found in the first check, 2026-10-30): asked about a whole form, the built-in engine's Qwen2.5 1.5B copied the labels back ("1: Email address") and matched nothing, with or without an example. Asked about one box at a time, it got 12 of 14 labels right, Hindi ones too, in 2.6 s for all 14. Its two misses were boxes about someone else ("Father's name" → Full name), which `someone_else` now leaves for the user without asking.

## Test strategy
| Criterion | How |
|---|---|
| FIL-1.2 | `details` (JS): the boxes with a value, the address lines joined, More details' lines split at their first colon, a bare link and lines without a name or value left out. `usable`: empty names or values dropped, 50 at most, names and values cut; `messages` has the names and none of the values. |
| FIL-2.2 | `visible`: a box of a pixel or far to the left is left out, one below the fold isn't. `sort`: filled boxes counted, a password box and "OTP", "CVV", "Card number" left, "PIN code" and "Aadhaar card number" not; `sensitive` on its own; boxes about someone else left, unless a detail has their name. |
| FIL-2.3 | `sort` on exact names with marks, and on the form's names left empty; `FORM` against the tab's names; `answer` on reply shapes and on replies that aren't a detail's name; `messages` wraps the label in `<form_field>` and defuses it; `match_boxes` with a fake model, and with one that fails. |
| FIL-2.4 | The request for a cloud model hides an email in a label (through `fill::requests`). |
| FIL-2.6 | `refusal` for text, Linux, and no Insert; `/fill` in `/help` and invalid as a skill name. |
| FIL-3.2 | `typed` on line breaks, a tab and Unicode. |
| FIL-2.8 | `common` on labels with asking words, Hindi labels, and whole labels only ("Company name", "Bank name" don't match); `sort` with a common name of an empty detail. |
| FIL-2.9 | `sort`: a question with a choice counted as filled, a single check box and an `Other` box left; `placeholder`. |
| FIL-2.10 | `usable` adds first, last and full names; `date` on its formats; `fit` for each kind; `choose` on exact, prefix and ambiguous options; a row that doesn't fit is left. |
| FIL-2.11 | `answer` on common names and sentences, "none" and two names. |
| FIL-2.12 | `Plan::without_values`; the plan's rows have the fitted value. |
| FIL-3.4 | `targets`: unticked rows, a renamed detail and an emptied one are skipped; today's value is used. |
| Manual | FIL-1.1, 2.1, 2.5, 2.7, 3.1, 3.3 and 3.5 on Windows: a test form in Edge and Chrome (text boxes, a password, an OTP, a filled box, a textarea), a Microsoft Form, a Google Form, and a dialog box of a Windows app. |

¹ Approved under the user's go-ahead of 2026-10-30 (requirements.md).

² Approved under the user's ask of 2026-11-13 (requirements.md, picks 10 to 13).
