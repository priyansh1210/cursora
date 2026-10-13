# Backlog (post-v1)

These are ideas, not commitments. An item gets a numbered spec folder only when it is promoted, and nothing here is built before then (see constitution §9).

| Idea | Notes |
|------|-------|
| **Disable Cursora per app** | Turn Cursora off for chosen apps. Unlike the privacy blocklist (PRI), the hotkey is *passed through*, so Ctrl+G keeps working as "Go to line" in VS Code. Reuses the PRI app-matching logic. Toggle it from the tray with "Disable in <current app>". |
| Native Ollama support | Detect Ollama, pull models with progress and cancel, and use Ollama's own API (`/api/chat`) so Cursora can set the context size. Through its OpenAI-style API, Ollama runs models at its small default window and cuts longer requests without saying so. Was part of spec 04 (LLM-2.1, LLM-2.3); moved here on 2026-10-12 because Ollama can't be installed on the development PC to test it. Until then, Ollama can be added as an OpenAI-compatible server (`http://localhost:11434/v1`), with that limit. |
| Wayland global-shortcuts portal | Register the hotkey through the XDG Desktop Portal *GlobalShortcuts* interface where the desktop has it (GNOME 48 and later, KDE Plasma), so users don't bind `cursora --ask` by hand. Was LNX-2.2; moved here on 2026-10-13: Ubuntu 22.04 and 24.04 don't have it, and it can't be tested on any machine available. |
| Replace in place | Skill option `output: replace` writes the answer back over the selection, with a diff preview and undo. |
| Context-aware mode | Detect the active app (IDE, browser, mail client) and suggest matching skills or tone. |
| Cursor companion avatar | An animated character that follows the cursor, with thinking and mood states. Skins match the persona. |
| Hover to define | Hold Alt and hover over a word to get a tooltip definition or translation. No selection needed. |
| Memory | A user profile ("I'm a CS student") plus local RAG over saved notes, using embeddings from a local embedding model. |
| Smart routing | Quick skills go to a small local model and hard questions go to the cloud. Falls back to local when offline or when the API fails. |
| Sticky answers | Pin an answer on screen, or save it to Markdown, Obsidian or Notion. |
| Tools & MCP | Web search, calculator and opening links. MCP server support so any tool can be plugged in. |
| Usage & cost dashboard | Per-provider token and cost tracking with a monthly budget cap. |
| Profiles | Work, Study and Personal profiles, each with its own persona, model, skills and privacy rules. |
| `/fix-error` | Explain a selected stack trace or error, with web search. |
| Model compare | Two models answer side by side. |
| Learning mode | Translations with pronunciation, plus flashcards from looked-up words. |
| Accessibility | Read any selection aloud, a dyslexia-friendly font, high contrast and a resizable bubble. |
| Do Not Disturb | Turn off automatically during full-screen games or presentations. |
| Local OCR fallback | Windows OCR or Tesseract, so text-only models can also use snip. |
| Encrypted settings sync | Optional sync of settings, personas and skills across devices. |
