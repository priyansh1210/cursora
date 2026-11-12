# 27: Fill forms (FIL)

Status: **Approved**¹ (2026-10-30; any kind of box and fitted values added 2026-11-13²).

## Scope
- **In:** **My details**, a tab of its own in Settings: a form of the user's details (name, parents' names, mobile number, address, college, …) and a box for any others. In any app on Windows, with nothing selected, **Fill this form** (or `/fill`) reads the form's boxes (text boxes, date boxes, drop-down lists, radio buttons and check boxes), matches them to My details, fits each value to its box, shows the plan to check, and on **Fill** types each value into its box one letter at a time, or picks its option, with a frame around the box being filled. It works with any model, the small local ones (1.5B to 7B) included: most boxes are matched without one.
- **Out:**
  - Linux: the next step, as its own update (it needs AT-SPI to read forms, [backlog](../backlog.md)). Until then `/fill` says it works on Windows for now.
  - A check box on its own (terms, consent, a newsletter): never ticked for the user.
  - Time, month and week boxes, sliders, file uploads, and lists whose options open somewhere else on the page: left for the user.
  - Boxes that already have text: they are left as they are.
  - Passwords, one-time codes, card numbers, security codes and captchas: never filled.
  - Fitting a value by a model: values are fitted on this computer, by fixed rules (FIL-2.10); a value no rule fits is left for the user.
  - Moving the mouse pointer, and submitting the form.
  - A hotkey of its own: Ctrl+G with nothing selected, then **Fill this form**.

Asked for by the user on 2026-10-30. Any kind of box, with any model, asked for on 2026-11-13.

## Requirements

### FIL-1 My details
User story: As a student who fills many forms (college, jobs, Microsoft Forms), I want to write my details down once, so Cursora can type them into any form for me.
- FIL-1.1 (manual) Settings SHALL have a **My details** tab, after Personality: a form with a box for each of Full name, First name, Last name, Date of birth, Gender (under You), Father's name, Mother's name (Family), Mobile number, Email (Contact), Address line 1, Address line 2, City, State, PIN code, Country (Address), College or company, Course or job title (Study and work), LinkedIn, GitHub and Website (Online), all empty at first; then **More details**, a text box for any other detail, one per line as its name, a colon and the value ("Blood group: O+"). A box SHALL be saved when it loses focus. A line SHALL say that the details stay on this computer and that an AI model sees only their names.
- FIL-1.2 (auto) The details SHALL be the form's boxes with a value; **Address**, the two address lines joined by ", ", for a form with one address box; then each line of More details with a name before its first colon and a value after it, except a bare link ("https://…"). A detail's value SHALL never be sent to a model: only the names of the details that have a value. Only the first 50 details count, with names cut at 60 characters and values at 500, even from a settings file edited by hand.

### FIL-2 Read and match a form
User story: As a user on a form, I want Cursora to work out which of my details goes in which box, so I only check its plan.
- FIL-2.1 (manual) WHEN nothing is selected AND Cursora can put text into the app (Insert, WRT-3.1) on Windows THE bubble SHALL offer **Fill this form**, which asks `/fill`.
- FIL-2.2 (auto) WHEN the user asks `/fill` THE SYSTEM SHALL read the boxes of the window the bubble opened over (FIL-2.7) (only the web page's, where the window has one) with their labels, in the form's order. Boxes that already have text SHALL be left out, and so SHALL boxes the user can't see (smaller than 4 pixels, or wholly to the left of the window), so a page can't collect more details than it shows. Password boxes, and boxes asking for a one-time code, a card number, a security code or a captcha, SHALL be listed as left for the user, and never filled. So SHALL boxes about someone else (a father, mother, guardian, spouse, emergency contact, reference or nominee), unless named exactly like one of the details.
- FIL-2.3 (auto) A box whose label is a detail's name (ignoring letter case, punctuation, a "required" mark such as Google Forms' "Required question" and Microsoft Forms' "Required to answer", and a question's number), or one of its common names (FIL-2.8), SHALL match it without a model. A box named like one of the form's boxes (FIL-1.1) that has no value SHALL be left for the user without a model, as no other detail fits it: "First name" never gets the full name. Each other box SHALL go to the conversation's model in a request of its own: its label in `<form_field>` as untrusted data (constitution §5), with a question's options, and the names of the details. The reply counts only when it names one detail with a value (FIL-2.11). Without other boxes, nothing is sent.
- FIL-2.4 (auto) That request SHALL follow the privacy rules of every request: personal details in the labels hidden for a cloud model (PRI-3.1), shown by "What will be sent" (PRI-3.6), and waiting for Send while confirming before cloud requests is on (PRI-3.7).
- FIL-2.5 (manual) THE answer SHALL show the plan: a row per box to fill, with the box's label, the detail and its value, and a tick box, ticked; then the boxes left for the user; then how many boxes already had text. IF the model fails, the boxes matched by name SHALL still be offered, with the error.
- FIL-2.6 (auto) `/fill` SHALL say why and read nothing WHEN the app can't be typed into (text is selected, Wayland, a terminal, an app that is blocked, elevated or read from the screen), on Linux, or WHEN no detail has a value (with a button to open My details). It SHALL be listed in `/help` and the command list, and no skill can take its name (SKL-2.6).
- FIL-2.7 (manual) Besides text boxes, `/fill` SHALL read: date boxes; drop-down lists (a page's `select`, a list box like Google Forms'); groups of radio buttons and groups of check boxes, each group as one question, labelled by its group (a fieldset's legend, an ARIA label) or else by the text just before it, with its buttons' names as its options; a date box inside a named group (Google Forms') labelled by that group; and a combo box that takes typing, as a text box. The rules of FIL-2.2 (hidden, sensitive, someone else) SHALL apply to every kind.
- FIL-2.8 (auto) A box whose whole label is a common name of a detail of the My details form SHALL match that detail without a model: "Name", "Your name", "Applicant name" for Full name; "Phone", "Contact no.", "WhatsApp number" for Mobile number; "Email ID", "E-mail address" for Email; "DOB" for Date of birth; "Sex" for Gender; "Pincode", "Postal code", "ZIP" for PIN code; "Surname" for Last name; "Institute", "University", "Organisation", "Employer" for College or company; "Branch", "Degree", "Designation" for Course or job title; "Residential address", "Permanent address" for Address; and the like, Hindi ones too (the full list is in the design). A common name of a detail without a value SHALL be left for the user, as in FIL-2.3.
- FIL-2.9 (auto) A question SHALL count as filled WHEN one of its options is already chosen or ticked, and a list WHEN its choice isn't a placeholder ("Choose", "Select…", "--", nothing). A check-box question with one option (terms, consent) SHALL be left for the user. So SHALL a box Cursora can't fill (a time box).
- FIL-2.10 (auto) Each value SHALL be fitted to its box on this computer, never by a model: First name and Last name come from Full name (its first and last word) when not saved, and Full name from them; a date is typed into a date box as its parts, and into a text box in the format its label or placeholder shows (DD/MM/YYYY, MM/DD/YYYY, YYYY-MM-DD, with "/", "-" or "."), else as saved; a question or list gets the option that is the value (ignoring letter case and punctuation), else the one option the value starts or the option starts with ("M" → "Male"); a check-box question gets one option for each part of the value between commas. A value that doesn't fit (no such option, a date it can't read for a date box) SHALL leave its box for the user. A list whose options show only when it opens SHALL be offered with the value, and its option found when filling.
- FIL-2.11 (auto) A reply SHALL also count WHEN it is a common name of a detail (FIL-2.8), or names exactly one detail in a sentence ("The box asks for Email."). It never counts WHEN it says none.
- FIL-2.12 (auto) The plan SHALL show each row's fitted value, and History SHALL never keep those values.

### FIL-3 Fill
User story: As a user who checked the plan, I want to watch Cursora type my details in, and stop it if something looks wrong.
- FIL-3.1 (manual) WHEN the user presses **Fill** THE SYSTEM SHALL bring the app back to the front (WRT-4.2), read its text boxes again, hide the bubble, and for each ticked box that is still there, empty and with the same label: focus it, draw a green frame around it, and type the value one letter at a time, or pick its option (FIL-3.5).
- FIL-3.2 (auto) A line break in a value SHALL be typed as ", ", and other control characters dropped, so filling never presses Enter or Tab (which could submit the form or move to another box).
- FIL-3.3 (manual) Typing SHALL stop at once WHEN the user presses Esc or another window comes to the front, so nothing is typed into another app.
- FIL-3.4 (auto) Only the ticked rows of the latest plan SHALL be typed, with the values saved at that moment; a detail renamed or emptied since is skipped.
- FIL-3.5 (manual) For a question, Fill SHALL select its radio button, or tick its check boxes (never untick one); for a list, open it, pick the option and close it; for a date box, type the date's digits into its first part; each in the green frame, so the page sees each change as if the user made it.

## Defaults taken (the user's picks of 2026-10-30)
1. **My details is a tab of its own, as a form**, not About you: About you goes with every question, and phone numbers and addresses shouldn't. (First built as a list of rows under About you; the user asked for the tab and the form, with Father's name, Mother's name, two address lines and a text box at the end, while testing on 2026-10-30.)
2. **No hotkey of its own**: Ctrl+G with nothing selected, then **Fill this form** or `/fill`.
3. **The plan is checked first**, with tick boxes and one **Fill** button.
4. **Typing one letter at a time with a frame around the box**; the mouse pointer isn't moved.
5. **Text boxes only** in this first version; the rest is listed as left for the user.
6. **Boxes with text are skipped**, never overwritten.
7. **Windows first**; Linux (X11) is the next step.
8. **ID numbers** (Aadhaar, PAN, roll number) are filled when the user saved them.
9. **Never passwords, one-time codes or card numbers.**

Taken on 2026-11-13, under the user's ask to fill any kind of box with any model (the user reviews them):

10. **Every kind the page offers**: text, date, list, radio and check-box questions; a single check box (terms, consent) is never ticked.
11. **Common names without a model**: a fixed list, so a 1.5B model's mistakes ("Your Name" → First name, "Postal code" → Country) can't happen on the usual boxes.
12. **Values are fitted by fixed rules, never by a model**, so values still never reach one (FIL-1.2): the first and last word of the full name, a date in the box's format, the option the value names.
13. **Overrides R91's rule only where it is safe**: "First name" gets the first word of Full name when First name isn't saved, never the full name.

## Open questions
None.

¹ Approved under the user's go-ahead of 2026-10-30 to build it with the recommended choices listed above, with the user reviewing everything afterwards.

² Approved under the user's ask of 2026-11-13 to fill any kind of box with any model, small local ones included; the choices made are picks 10 to 13, for the user's review.
