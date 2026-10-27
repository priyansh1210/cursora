# 18: Design refresh (DSN)

Status: **Approved** (2026-10-27). The user approved it with "continue with the work", keeping today's colours ("current color palette is fine") and choosing Geist.

## Scope
- **In:** the look and layout of every window: the bubble and the expanded window, Settings, Setup and a pinned note. One set of design tokens behind them. How the AI source is chosen, in Setup and in Settings → Models → Add a provider.
- **Out:**
  - New features: every button, setting and message keeps doing what it does today
  - The logo and the green brand colour, which stay
  - The snip overlay, apart from its colours moving into tokens
  - The user manual (`user-manual.html`)
  - Wording, apart from the lines named below

Asked for by the user on 2026-10-27: "follow product designing principles to update the whole designing of cursora". Before and after screenshots of every window are in the review page linked from the pull request.

## Principles
Every requirement below comes from one of these.
1. **Content first.** The answer is what the user came for. The header, the captured text and the controls stay quieter than it.
2. **One primary action per view.** Only the action a view exists for is a filled button: Send, Replace, Next, Save, Add.
3. **Recognition over recall.** The choices show their trade-offs, so a new user doesn't need to know provider names.
4. **Platform conventions.** Like Windows 11 Settings: grouped rows with the control at the right edge, and switches for settings that apply at once.
5. **Consistency through tokens.** One scale each for type, space, corner radius and colour, in both themes.
6. **Accessible by default.** WCAG 2.2 AA contrast, the keyboard, unchanged screen-reader names, less motion on request, and the Linux screen-reader rules of issue 20 (R41).

## Requirements

### DSN-1 Design foundation
User story: As a user, I want every Cursora window to look like one product and to be easy to read in light and dark, so that it feels trustworthy.
- DSN-1.1 (auto) THE stylesheet SHALL define colours only as theme tokens, and the dark theme SHALL have the same tokens whether the system or the user picks it.
- DSN-1.2 (auto) No text SHALL be smaller than 12 px.
- DSN-1.3 (auto) In both themes, text SHALL have a contrast of at least 4.5:1 with each background it sits on. Focus rings and the parts of a switch SHALL have at least 3:1 (WCAG 1.4.3, 1.4.11).
- DSN-1.4 (manual) A view SHALL have at most one filled button, for the action the view is for.
- DSN-1.5 (manual) WHERE the system asks for reduced motion, transitions and the "Thinking…" pulse SHALL be off.

### DSN-2 Bubble
User story: As a user, I want the answer to be the clearest thing in the bubble, with its actions easy to hit, so that I can read it and use it quickly.
- DSN-2.1 (manual) THE header SHALL be one row 40 px high, and the model switcher SHALL look like quiet text until the pointer is over it. BUB-7.1, BUB-7.3 and BUB-7.4 still hold.
- DSN-2.2 (manual) Under a finished answer, **Replace** or **Insert** SHALL be a filled button with its label. Copy, Pin, Save, Show in folder, Regenerate and Save as skill SHALL be icon buttons with a tooltip. Each keeps its accessible name ("Save as skill" gains one).
- DSN-2.3 (manual) The question box, the "What will be sent" toggle and Send (or Stop) SHALL sit in one field. Send and Stop SHALL be icon buttons named "Send" and "Stop the answer". WHERE the webview supports it (WebView2), the box SHALL grow with its text up to six lines. Elsewhere it keeps two.
- DSN-2.4 (manual) Answers SHALL style quotes, lists, headings, tables, rules and code for a 380 px column, with no browser-default 40 px indents.
- DSN-2.5 (manual) The captured text SHALL show as a quote with a green edge, quieter than the answer. BUB-3.1 still holds.

### DSN-3 Settings
User story: As a user, I want to scan Settings quickly, so that I can find and change a setting without reading every line.
- DSN-3.1 (manual) Each tab except About SHALL open with its title and one line on what it's for. Its settings SHALL sit in grouped cards, one row per setting, with the control at the right edge and any note under its label.
- DSN-3.2 (manual) An on/off setting that applies at once SHALL be a switch (`role="switch"`). Picking items from a set (what to hide, a provider's models) and options in a form with a Save button SHALL stay checkboxes.
- DSN-3.3 (manual) Personas and skills SHALL each be one list. Their buttons SHALL be quiet (no border), and Delete SHALL use the error colour.

### DSN-4 Setup
User story: As a new user, I want to see my options for an AI source and what each costs me, so that I can start without knowing provider names.
- DSN-4.1 (manual) Setup's first step and Settings → Models → Add a provider SHALL offer the AI source as three cards, each with its trade-off: **Download a free model** (marked Recommended), **A cloud provider** and **A model I already have**. Only the last two then ask which provider or kind. A model already running on this computer is still offered first, as before.
- DSN-4.2 (manual) THE Setup window SHALL open at 600 × 600 px, so that the first step fits without scrolling.

## Choices made for the user (to review)
1. **Icons for an answer's secondary actions.** Five text links didn't fit on one line in the 380 px bubble once "Saved" and "Show in folder" appeared. Each icon has a tooltip and its old screen-reader name. Rejected: keeping text links (they wrap, and every link looks equally important).
2. **"Download a free model" is marked Recommended in Setup:** it needs no account or key, the biggest drop-off in [product direction](../../memory/product-direction.md). Its card says the download size, and the cloud card says it gives the best answers, so the trade-off is honest.
3. **Fluent's look for Windows**, which is most users: switches, grouped rows, and a darker lower edge on text boxes. Linux gets the same look.
4. **The colour palette: today's green (Forest), refined.** Chosen by the user on 2026-10-27 ("current color palette is fine") after reviewing twenty options on the review page, twelve one-colour and eight two-tone. The logo keeps its colours. Another palette later changes only the colour tokens and the logo's five colours, and the icon files are regenerated from `icon.svg`.
5. **The font: Geist and Geist Mono**, chosen by the user on 2026-10-27 from nine options on the review page. They ship inside the app as Latin `woff2` files, regular and semibold, about 50 KB, and are never downloaded at run time (constitution §3). Indian scripts fall back to Nirmala UI on Windows and to the desktop's fonts on Linux (spec 16).

## Open questions
None.
