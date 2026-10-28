# 24: Smart routing (ROU)

Status: **Approved**¹ (2026-10-28).

## Scope
- **In:** a **Quick model** on the user's own computer (or a trusted LAN host). Small, routine jobs go to it, and it answers when the default model can't be reached, such as when the PC is offline.
- **Out:**
  - Judging how hard a free-typed question is (by its words, or by asking a model first)
  - Routing to a second cloud model, or by cost
  - Choosing per skill which jobs are quick (the list is fixed for now)
  - Usage and cost tracking (backlog)

Update #8 from the list of 2026-10-28 ([improvements batch 2](../../memory/improvements-batch-2.md)).

## Requirements

### ROU-1 Quick jobs on the quick model
User story: As a user with a cloud default model, I want small edits such as Fix and Define answered on my own computer, so they're fast, free and private, while real questions still get the stronger model.
- ROU-1.1 (manual) Settings → Models SHALL have **Smart routing**, with a **Quick model** list of the local and trusted-LAN models, Off by default, and a line saying which jobs go to it.
- ROU-1.2 (auto) WHILE a quick model is set, WHEN a question runs Define, Fix, Rewrite or Simplify (typed, or from a quick action button) THE SYSTEM SHALL answer it with the quick model, unless the skill names its own model (SKL-2.5) or the conversation has one of its own (a snip's, VIS-1.2). Other questions, follow-ups included, go to the default model as before.
- ROU-1.3 (auto) IF the selection is longer than 4,000 characters, or is files or a snip, or the selection or the text typed after the skill's name has letters outside the English alphabet, or answers are set to come in another language (My language, LNG-1.3, or a persona's own language), THE quick job SHALL go to the default model.
- ROU-1.4 (auto) A quick model that is no longer one of its provider's chosen models, or that is a cloud model (a settings file edited by hand), SHALL count as Off.
- ROU-1.5 (manual) A quick job's answer SHALL say under it which model answered, and "What will be sent" (PRI-3.6) SHALL name that model. The header and its Local or Cloud badge SHALL keep naming the model other questions go to (BUB-7.1).

### ROU-2 The quick model as a fallback
User story: As a user on a train with no connection, I want Cursora to still answer, from the model on my computer, and to tell me so.
- ROU-2.1 (auto) WHILE a quick model is set, IF another model fails before writing anything because it can't be reached or is overloaded (a network, server or rate-limit error), THE SYSTEM SHALL ask the quick model instead, with the request built for it, and say under the answer which model couldn't answer and why, in plain words.
- ROU-2.2 (auto) An error after part of the answer was written, and any other error (a wrong key, billing, a model that doesn't exist, a request too long), SHALL be shown as before, without the fallback.
- ROU-2.3 (manual) WHILE a quick model is set, the check at the hotkey (LLM-9.1) SHALL NOT replace the bubble's input with "No AI model is available" because the default model can't be reached, so a question still reaches the quick model.

## Defaults taken under the go-ahead of 2026-10-28
1. **Quick jobs are a fixed list: Define, Fix, Rewrite and Simplify**, including the quick action buttons that run them (Fix, Rewrite, Shorter, More polite). They are short, edit-sized jobs that a small model does well. Translate stays with the default model, since small models can't do Indian languages (issue 52); Explain, Summarize and the code skills need the stronger model.
2. **Up to 4,000 characters of selected text** (about 1,000 tokens), so a quick job fits the built-in engine's context and stays quick.
3. **English only.** Small local models can't do Indian languages (issue 52), and Medium and Large weren't reliable in spec 16's trial, so text with other letters (accented Latin too, to keep the rule simple) and answers in another language stay with the default model. Found in the review.
4. **Off by default**: nothing changes until the user picks a quick model.
5. **Only local and trusted-LAN models can be the quick model**, so the fallback works offline and quick jobs never leave the user's network.
6. **The fallback covers any model that isn't the quick model**, not only cloud ones: a LAN server that is switched off falls back too.
7. **A short note under each routed answer** ("Quick job: answered by …"). The header keeps showing the default model and its badge, since that's where the next typed question goes, and a badge that turned "Local" after a quick job would mislead about privacy. Found in the first check, where the header said "cloud-x" beside a "Local" badge.

## Open questions
None.

¹ Approved under the user's go-ahead of 2026-10-28 to design, build and merge updates 1 and 3 to 9, with the user reviewing everything afterwards. The choices made for the user are listed above.
