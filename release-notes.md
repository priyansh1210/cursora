Cursora 0.3.0 fills in forms for you, searches the web, listens, and answers faster.

New
- Fill forms (Windows): save your details once in Settings → My details. In any form, with nothing selected, press Ctrl+G and choose Fill this form (or type /fill). Cursora matches the form's text boxes to your details, shows you the plan, and types them in on Fill. Passwords, one-time codes and card numbers are never filled.
- Web search: /web <question> searches the web with your free Tavily key and lists the sources under the answer. It works with local models too, and stays off in local-only mode.
- Voice input: the mic button in the bubble turns what you say into your question, on your own computer. Two speech models to download in Settings → General; Accurate is better for Hindi and the other Indian languages.
- Define: point at any word on screen, in any app, a PDF or a picture, and press Ctrl+Alt+G for its meaning. Nothing to select. (Not on Linux with Wayland.)
- Snip with any model: when your model can't see images, the text in a snip is read on your computer and sent instead. On Linux this needs tesseract-ocr.
- Faster local answers: the built-in engine uses your dedicated graphics card (NVIDIA, AMD or Intel Arc) when you have one. A switch in Settings → Models turns it off.
- Smart routing: choose a Quick model on your computer in Settings → Models. Short jobs like Define and Fix go to it, and it answers when your main model can't be reached, such as offline.
- About you: a few lines about yourself in Settings → Personality, which every answer takes into account.
- Turn Cursora off in an app: from the tray menu or Settings → General, so its hotkeys reach that app instead (Ctrl+G stays "Go to line" in VS Code).

Fixed
- Replace keeps code exactly as written, and a quick second click on Replace no longer loses what was on your clipboard.
- A pinned note opens beside the bubble even when the bubble is at the top of the screen.
- A conversation reopened from History names the model that will answer your next question.
- /translate hinglish answers in English letters, not Devanagari.
- A damaged history file is moved aside and a new one started, instead of history silently not saving.
- The Gemini model list leaves out image models that can't chat.
- "Show more" only shows under text long enough to need it.
- An answer can no longer be made to freeze the bubble (an update of the Markdown library).
- Smaller Linux packages: each engine library is included once.
