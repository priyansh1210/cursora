# 18: Design refresh (tasks)

Status: **In progress**. Tasks 1 to 4 are built; the manual checklist is open.

- [x] 1. Tokens: rewrite `styles.css` around them, and add `src/design.test.ts`. Covers DSN-1.1, DSN-1.2, DSN-1.3, DSN-1.5. Geist and Geist Mono ship in `src/fonts/`.
- [x] 2. Bubble: icons, the answer's actions, the question field, the captured-text quote and the answer's typography. Covers DSN-2.1 to DSN-2.5, DSN-1.4.
- [x] 3. Settings: titles, groups, switches, and the persona and skill lists. Covers DSN-3.1 to DSN-3.3, DSN-1.4.
- [x] 4. Setup: the three source cards in `AddProvider`, the step's intro line, and the window size. Covers DSN-4.1, DSN-4.2.
- [x] 5. Screenshots of every window in both themes (browser preview; the real app is issue 63), `npm test`, `npm run trace`, `npm run build`, `cargo test`, `npm run test:linux`.

## Manual checklist
- [ ] DSN-1.4: each window has at most one filled button (the bubble, the expanded window, every Settings tab, every Setup step, a pinned note). Windows ☐ Linux X11 ☐
- [ ] DSN-1.5: with "Show animations" off in Windows (or `gtk-enable-animations` off on GNOME), switches jump and "Thinking…" doesn't pulse. Windows ☐ Linux X11 ☐
- [ ] DSN-2.1: the header is one row; the model switcher looks like text until pointed at; a long model name still leaves Expand in view (BUB-7.4). Windows ☐ Linux X11 ☐
- [ ] DSN-2.2: Replace is filled; Copy (and its check), Pin, Save, Show in folder, Regenerate and Save as skill are icons with tooltips; Narrator/Orca read their names; Tab reaches each one. Windows ☐ Linux X11 ☐
- [ ] DSN-2.3: Send and Stop sit inside the field; "What will be sent" opens above it; on Windows the box grows to six lines, on Linux it keeps two rows. Windows ☐ Linux X11 ☐
- [ ] DSN-2.4: an answer with a quote, a list, a heading, a table, a rule and code reads well at 380 px. Windows ☐ Linux X11 ☐
- [ ] DSN-2.5: the captured text is a quote with a green edge; Show more and × still work. Windows ☐ Linux X11 ☐
- [ ] DSN-3.1: every tab but About opens with its title and line; settings are in grouped cards with controls at the right. Windows ☐ Linux X11 ☐
- [ ] DSN-3.2: on/off settings are switches that Space toggles and screen readers call switches; "What to hide", a provider's models, the persona editor's Emoji and the skill form's options are checkboxes. Windows ☐ Linux X11 ☐
- [ ] DSN-3.3: personas and skills are one list each; their buttons are borderless; Delete is red. Windows ☐ Linux X11 ☐
- [ ] DSN-4.1: on a fresh setup, the three cards show; Download shows the sizes; Cloud lists only key providers; Already have lists the .gguf file, LM Studio and Custom; a model found on this computer is offered above them. The same in Settings → Models. Windows ☐ Linux X11 ☐
- [ ] DSN-4.2: Setup opens at 600 × 600 and step 1 fits without scrolling. Windows ☐ Linux X11 ☐
