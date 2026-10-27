# 18: Design refresh (design)

Status: **Approved** (2026-10-27). Implements [requirements.md](requirements.md). The change is mostly CSS: `styles.css` is rewritten around tokens. The markup changes are small and leave behavior alone: wrappers for groups, `role="switch"` on on/off settings, icons for an answer's secondary actions, and the question box and Send in one field. Rust changes only the Setup window's size.

## Overview
```
styles.css   tokens (type, space, radius, colour × light/dark) → every rule uses them (DSN-1.1)
             design.test.ts reads the file: colours only in tokens, no text < 12 px, contrast pairs (DSN-1.1–1.3)
Bubble       header 40 px · quote with green edge · answer typography
             actions: [Replace] copy · pin · save · regenerate · save as skill   (icons from icons.tsx, names unchanged)
             field:   [ question box ……………… ]
                      [ What will be sent ›        (↑) ]
Settings     <h1>Tab</h1> one line · <h2>Group</h2> <div class="group"> rows… </div>
Setup        step 1: three source cards → provider list only for Cloud / Already have
```

## Tokens
| Kind | Tokens |
|---|---|
| Font | Geist for text, Geist Mono for code: `src/fonts/*.woff2` with `@font-face`, then the system fonts as fallbacks (Nirmala UI draws Indian scripts on Windows). |
| Type | `--text-xs` 12, `--text-sm` 13, `--text-md` 14 (body), `--text-lg` 16, `--text-xl` 20 px. Headings: page title 20/600, group title 14/600. |
| Space | `--sp-1` to `--sp-6`: 4, 8, 12, 16, 24, 32 px. |
| Radius | `--r-sm` 6, `--r-md` 8 (controls), `--r-lg` 12 (cards, the question field), `--r-full`. |
| Colour | `--bg`, `--surface`, `--surface-2`, `--sidebar`, `--hover`, `--selected`, `--fg`, `--muted`, `--line`, `--line-strong`, `--accent` (+ `-hover`, `-text`, `-soft`, `on-accent`), `--focus`, `--error` (+ `-soft`), `--code-bg`, `--hl-*`, `--switch-thumb-on`, and the snip overlay's `--shade`, `--snip-bg`, `--snip-edge`, `--snip-label`. |

Colour changes that fix contrast found by DSN-1.3's test on the current palette:
- Light focus ring `#22c55e` → `#15803d`: it was 2.3:1 on white and needs 3:1.
- Light `--hl-string` `#a16207` → `#92400e`. It was 4.2:1, and 4.3:1 on the new code background.
- Light `--hl-number` was 4.4:1 on the old code background (`#eceeec`), and passes on the new one (`#eef0ee`).

## Components
| File | Change |
|---|---|
| `src/styles.css` | Rewritten around the tokens. New: `.group`, `input[role="switch"]`, `.composer`, `.answer` typography, `.lead`, reduced motion. `button[type="submit"]` is no longer filled; only `.primary` is (DSN-1.4). Text boxes get a lower edge in `--muted` (decision 4). |
| `src/design.test.ts` (new) | DSN-1.1, 1.2 and 1.3, reading `styles.css`. |
| `src/fonts/` (new) | Geist and Geist Mono, Latin, regular and semibold, under the SIL Open Font License, with its licence file. |
| `src/icons.tsx` | New paths: `close`, `chevron`, `copy`, `pin`, `save`, `regenerate`, `send`, `stop`. `Icon` takes a stroke width. |
| `src/Chat.tsx` | Actions as icon buttons. Replace and Insert become `.primary`. The question field holds the "What will be sent" toggle and Send/Stop. `Outgoing` draws only the open panel (its toggle moved into the field). Context's × becomes the `close` icon. `CopyButton` becomes an icon, and shows a check for 1.2 s after copying. |
| `src/Settings.tsx` | `SUMMARY` per tab, `<h1>` and `.lead` above each tab except About. General's groups. Switches. |
| `src/Privacy.tsx`, `History.tsx`, `Personality.tsx`, `SkillsTab.tsx` | Groups and switches. The heading that repeated the tab's title goes (Personality, Skills, History). Personas and skills become `.group` lists. Primary on the forms' Save buttons. |
| `src/Models.tsx` | `AddProvider`: the `KINDS` cards, then the provider list filtered by `needsKey` (Cloud) or local presets plus the built-in engine (Already have). Its Add button is `.primary`. |
| `src/Setup.tsx` | The intro line of step 1 changes (the cards say the rest). Switches on the last step. "Use this model" is `.primary`. |
| `src-tauri/src/lib.rs` | The Setup window opens at 600 × 600 instead of 520 × 480 (DSN-4.2). |

## Decisions
1. **A CSS rewrite, not a component library.** The windows already share one stylesheet and a few plain components, and the problems are in tokens and layout. Rejected: a UI kit such as Fluent UI React. It adds about 200 kB to every window, including the bubble that must open in 200 ms (constitution §8), and it would change every control's markup and accessibility.
2. **The switch is drawn with gradients on `appearance: none`, not with `::before`.** Pseudo-elements on inputs are unreliable in WebKitGTK. A gradient thumb that slides with `background-position` works in both webviews. The input stays a checkbox, so keyboard, labels and `onChange` don't change. `role="switch"` makes screen readers say "on" and "off".
3. **Groups are wrapper `<div>`s; rows and notes keep their markup.** A note stays a separate `<p>` (some named by `aria-describedby`). CSS removes the line between a row and its note. Rejected: putting the note inside the `<label>`, which would add it to the setting's name.
4. **Text boxes get a lower edge in `--muted`, as in Fluent.** Their light outline is about 1.6:1. The darker lower edge gives the 3:1 that WCAG 1.4.11 asks for, without heavy boxes.
5. **The question box grows with `field-sizing: content`.** WebView2 supports it. WebKitGTK ignores it and keeps `rows={2}`. Rejected: measuring in JavaScript on every keystroke for a nicety.
6. **Icon buttons keep `aria-label`, and gain `title`.** Screen readers say the same names as today. Sighted users get a tooltip. The filled Replace/Insert keeps its visible word, since it's the one action that changes another app.
7. **The source cards reuse `fieldset label.row` radio cards** (the engine lifetime already uses them). No new component.
8. **Geist ships inside the app.** Two variable `woff2` files (Latin, about 52 KB together) and their licence, loaded with `@font-face` from the app's own files, so the CSP's `default-src 'self'` already allows them. Rejected: the system fonts (no voice of Cursora's own, and a different look on Linux) and loading from Google Fonts at run time (constitution §3: no network calls but the configured providers and the update check).

## Platform notes
- **WebKitGTK (Linux):** `:has()`, `color-mix()`, container queries and `appearance: none` are already in use. `field-sizing` and the `lh` unit fall back as decision 5 says. No new text-only flex items are added: the cards' text sits in the same `<span>` structure as the existing radio cards (issue 20, R41).
- **Windows:** Geist draws Latin text; Segoe UI Variable is the fallback, and Nirmala UI draws the Indian scripts, since the Geist files carry Latin only. The bubble's rounded corners still come from the window shadow (BUB-2.1).

## Test strategy
| Criteria | How |
|---|---|
| DSN-1.1, 1.2, 1.3 | Unit: `src/design.test.ts` parses `styles.css`. Contrast uses the WCAG luminance formula. A colour with transparency is placed over `--bg` first. |
| DSN-1.4, 1.5, 2.x, 3.x, 4.x | Manual, on Windows and Linux X11: the checklist in [tasks.md](tasks.md). Before and after screenshots of every window, in both themes. |
| Everything else | The existing tests still pass. No behavior changes, so no other criterion changes. |
