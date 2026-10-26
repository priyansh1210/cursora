# 14: Quick actions (QAC)

## Scope
- **In:** one-click buttons in the bubble for the common jobs on a selection, chosen to suit the app the text came from, plus the user's pinned skills. Two new built-in skills those buttons need: `/reply` and `/error`.
- **Out:**
  - Buttons when nothing is selected, or for files and snips
  - A settings page for editing each app's buttons: they come from the app's kind, with pinned skills first
  - Detecting the kind of a web page (Gmail in Chrome counts as a browser)

Ideas #2 and #5 from the improvement plan ([product direction](../../memory/product-direction.md)), built at the user's request of 2026-10-26.

## Requirements

### QAC-1 The buttons
User story: As a user, I want the usual jobs (fix, shorten, explain, translate) one click away, without typing a command.
- QAC-1.1 (manual) WHEN the bubble opens with text selected in an app, and before the first question, THE bubble SHALL show up to six buttons above the input. A button runs its command as if typed, e.g. **Fix** runs `/fix`, and **Shorter** runs `/rewrite shorter`. Text already in the input goes after the command, so **Rewrite** with "like a pirate" typed runs `/rewrite like a pirate`. A replacement skill's answer then offers Replace first (spec 13).
- QAC-1.2 (manual) Alt+1 to Alt+6 SHALL press the buttons in order. Each button's tooltip shows its key.
- QAC-1.3 (auto) A button whose skill doesn't exist, or can't run with the active model (a skill that needs vision, SKL-2.4), SHALL NOT be shown.

### QAC-2 Buttons that suit the app
User story: As a user, I want different buttons in my terminal, my code editor, my email and my documents.
- QAC-2.1 (auto) THE kind of app SHALL come from its program name:
  - **terminal:** the "Ctrl+C may stop a process" list (SEL-4.3), and known terminals
  - **code:** VS Code, Cursor, Visual Studio, JetBrains IDEs, Notepad++, Sublime Text
  - **mail and chat:** Outlook, Thunderbird, Teams, Slack, Discord, WhatsApp, Telegram, Signal
  - **everything else**
- QAC-2.2 (auto) THE buttons SHALL be:

  | Kind | Buttons |
  |---|---|
  | Terminal | Explain error (`/error`) · Explain command (`/code`) · Explain (`/explain`) |
  | Code | Explain (`/code`) · Find bugs (`/bugs`) · Explain error (`/error`) · Summarize (`/summary`) |
  | Mail and chat | Reply (`/reply`) · Fix (`/fix`) · More polite (`/rewrite more polite`) · Shorter (`/rewrite shorter`) · Translate (`/translate`) · Explain (`/explain`) |
  | Everything else | Fix (`/fix`) · Rewrite (`/rewrite`) · Shorter (`/rewrite shorter`) · Explain (`/explain`) · Summarize (`/summary`) · Translate (`/translate`) |

### QAC-3 Pinned skills
User story: As a user with my own skills, I want my favourites among the buttons.
- QAC-3.1 (auto) A skill pinned in Settings → Skills (a **Pin** button on its card, built-ins too) SHALL come first among the buttons, in every app, with no duplicates. There are still at most six.
- QAC-3.2 (manual) Settings → Skills SHALL have "Show quick actions in the bubble" (on by default); off hides the buttons.

### QAC-4 The two new skills
- QAC-4.1 (auto) THE built-in `/reply` SHALL draft a reply to the selected message, in its language and tone. It's an ordinary answer (`output: chat`), since the selection is the message received, not text to replace.
- QAC-4.2 (auto) THE built-in `/error` SHALL explain the selected error message and how to fix it, in steps.

## Defaults taken while the user was away (2026-10-26)
The user asked for ideas #1 to #5 to be built overnight and their choices listed for review:
1. **The buttons show only before the first question**, so the bubble stays small afterwards. Commands can still be typed.
2. **Alt+1 to Alt+6**, not 1 to 6: a question may start with a number.
3. **The kinds and button sets in QAC-2**, with "everything else" as the general set.
4. **Pinning lives in Settings → Skills**, not in each skill file, so built-ins can be pinned too.
5. **Browsers count as "everything else"**, even for webmail: the program name can't tell which site is open.

## Open questions
None.
