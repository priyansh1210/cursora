# 27: Fill forms (FIL)

Status: **Approved**¹ (2026-10-30).

## Scope
- **In:** **My details** in Settings: a list of the user's details (name, email, phone, address, college, …). In any app on Windows, with nothing selected, **Fill this form** (or `/fill`) reads the form's text boxes, matches them to My details, shows the plan to check, and on **Fill** types each value into its box, one letter at a time, with a frame around the box being filled.
- **Out:**
  - Linux: the next step, as its own update (it needs AT-SPI to read forms, [backlog](../backlog.md)). Until then `/fill` says it works on Windows for now.
  - Drop-down lists, tick boxes, radio buttons and date pickers: only text boxes are filled; the others stay for the user.
  - Boxes that already have text: they are left as they are.
  - Passwords, one-time codes, card numbers, security codes and captchas: never filled.
  - Making a value fit a box (a first name out of a full name, another date format): the saved value is typed as it is.
  - Moving the mouse pointer, and submitting the form.
  - A hotkey of its own: Ctrl+G with nothing selected, then **Fill this form**.

Asked for by the user on 2026-10-30.

## Requirements

### FIL-1 My details
User story: As a student who fills many forms (college, jobs, Microsoft Forms), I want to write my details down once, so Cursora can type them into any form for me.
- FIL-1.1 (manual) Settings → Personality SHALL have **My details**, under About you: rows of a name and a value, starting with Full name, First name, Last name, Email, Phone, Date of birth, Gender, Address, City, State, PIN code, Country, College or company, Course or job title, LinkedIn, GitHub and Website, all empty. Each row's name and value can be changed and the row removed, and **Add a detail** adds a row. A line SHALL say that they stay on this computer and that an AI model sees only their names.
- FIL-1.2 (auto) A detail's value SHALL never be sent to a model: only the names of the details that have a value. Only the first 50 details with a name and a value count, with names cut at 60 characters and values at 500, even from a settings file edited by hand.

### FIL-2 Read and match a form
User story: As a user on a form, I want Cursora to work out which of my details goes in which box, so I only check its plan.
- FIL-2.1 (manual) WHEN nothing is selected AND Cursora can put text into the app (Insert, WRT-3.1) on Windows THE bubble SHALL offer **Fill this form**, which asks `/fill`.
- FIL-2.2 (auto) WHEN the user asks `/fill` THE SYSTEM SHALL read the text boxes of the window the bubble opened over (only the web page's, where the window has one) with their labels, in the form's order. Boxes that already have text SHALL be left out, and so SHALL boxes the user can't see (smaller than 4 pixels, or wholly to the left of the window), so a page can't collect more details than it shows. Password boxes, and boxes asking for a one-time code, a card number, a security code or a captcha, SHALL be listed as left for the user, and never filled. So SHALL boxes about someone else (a father, mother, guardian, spouse, emergency contact, reference or nominee), unless named exactly like one of the details.
- FIL-2.3 (auto) A box whose label is a detail's name (ignoring letter case, punctuation and a "required" mark) SHALL match it without a model. A box named like one of the starting details (FIL-1.1) that has no value SHALL be left for the user without a model, as no other detail fits it: "First name" never gets the full name. Each other box SHALL go to the conversation's model in a request of its own: its label in `<form_field>` as untrusted data (constitution §5), and the names of the details. The reply counts only when it is the name of a detail with a value. Without other boxes, nothing is sent.
- FIL-2.4 (auto) That request SHALL follow the privacy rules of every request: personal details in the labels hidden for a cloud model (PRI-3.1), shown by "What will be sent" (PRI-3.6), and waiting for Send while confirming before cloud requests is on (PRI-3.7).
- FIL-2.5 (manual) THE answer SHALL show the plan: a row per box to fill, with the box's label, the detail and its value, and a tick box, ticked; then the boxes left for the user; then how many boxes already had text. IF the model fails, the boxes matched by name SHALL still be offered, with the error.
- FIL-2.6 (auto) `/fill` SHALL say why and read nothing WHEN the app can't be typed into (text is selected, Wayland, a terminal, an app that is blocked, elevated or read from the screen), on Linux, or WHEN no detail has a value (with a button to open My details). It SHALL be listed in `/help` and the command list, and no skill can take its name (SKL-2.6).

### FIL-3 Fill
User story: As a user who checked the plan, I want to watch Cursora type my details in, and stop it if something looks wrong.
- FIL-3.1 (manual) WHEN the user presses **Fill** THE SYSTEM SHALL bring the app back to the front (WRT-4.2), read its text boxes again, hide the bubble, and for each ticked box that is still there, empty and with the same label: focus it, draw a green frame around it, and type the value one letter at a time.
- FIL-3.2 (auto) A line break in a value SHALL be typed as ", ", and other control characters dropped, so filling never presses Enter or Tab (which could submit the form or move to another box).
- FIL-3.3 (manual) Typing SHALL stop at once WHEN the user presses Esc or another window comes to the front, so nothing is typed into another app.
- FIL-3.4 (auto) Only the ticked rows of the latest plan SHALL be typed, with the values saved at that moment; a detail renamed or emptied since is skipped.

## Defaults taken (the user's picks of 2026-10-30)
1. **My details is a list of its own**, not About you: About you goes with every question, and phone numbers and addresses shouldn't.
2. **No hotkey of its own**: Ctrl+G with nothing selected, then **Fill this form** or `/fill`.
3. **The plan is checked first**, with tick boxes and one **Fill** button.
4. **Typing one letter at a time with a frame around the box**; the mouse pointer isn't moved.
5. **Text boxes only** in this first version; the rest is listed as left for the user.
6. **Boxes with text are skipped**, never overwritten.
7. **Windows first**; Linux (X11) is the next step.
8. **ID numbers** (Aadhaar, PAN, roll number) are filled when the user saved them.
9. **Never passwords, one-time codes or card numbers.**

## Open questions
None.

¹ Approved under the user's go-ahead of 2026-10-30 to build it with the recommended choices listed above, with the user reviewing everything afterwards.
