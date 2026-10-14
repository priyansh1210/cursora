# Cursora Specs

Cursora is built **spec-first**. Read the [constitution](00-constitution.md) before changing anything.

## Status
Each document moves through: `Draft → Approved → In progress → Done`. A spec is **Done** only when every (auto) criterion has a passing test and its manual checklist has been completed on Windows and Linux.

| #  | Spec | Prefix | requirements | design | tasks | Status |
|----|------|--------|--------------|--------|-------|--------|
| 01 | [App shell](01-app-shell/requirements.md) | APP | Approved | [Approved](01-app-shell/design.md) | [In progress](01-app-shell/tasks.md) | In progress |
| 02 | [Trigger & selection capture](02-trigger-and-selection/requirements.md) | SEL | Approved | [Approved](02-trigger-and-selection/design.md) | [In progress](02-trigger-and-selection/tasks.md) | In progress |
| 03 | [Bubble UI](03-bubble-ui/requirements.md) | BUB | Approved | [Approved](03-bubble-ui/design.md) | [In progress](03-bubble-ui/tasks.md) | In progress |
| 04 | [AI providers](04-providers/requirements.md) | LLM | Approved | [Approved](04-providers/design.md) | [In progress](04-providers/tasks.md) | In progress |
| 05 | [Personality](05-personality/requirements.md) | PER | Approved | [Approved](05-personality/design.md) | [In progress](05-personality/tasks.md) | In progress |
| 06 | [Skills](06-skills/requirements.md) | SKL | Approved | [Approved](06-skills/design.md) | [In progress](06-skills/tasks.md) | In progress |
| 07 | [Privacy guard](07-privacy-guard/requirements.md) | PRI | Approved | [Approved](07-privacy-guard/design.md) | [In progress](07-privacy-guard/tasks.md) | In progress |
| 08 | [Linux platform](08-linux-platform/requirements.md) | LNX | Approved | [Approved](08-linux-platform/design.md) | [In progress](08-linux-platform/tasks.md) | In progress |
| 09 | [Voice in/out](09-voice/requirements.md) | VOC | Approved | [Approved](09-voice/design.md) | — | Approved (design); not started, by choice |
| 10 | [Vision snip](10-vision-snip/requirements.md) | VIS | Approved | [Approved](10-vision-snip/design.md) | [In progress](10-vision-snip/tasks.md) | In progress |

Post-v1 ideas: [backlog.md](backlog.md). Known problems, gaps and untested items: [issues.md](issues.md).

## Workflow per spec
1. **Requirements:** write `requirements.md` and get it reviewed. When approved, set its status to Approved.
2. **Design:** write `design.md`, covering interfaces, data models, Tauri commands/events, errors, platform notes and decisions. Get it reviewed and approved.
3. **Tasks:** write `tasks.md` as an ordered checklist. Each task:
   - takes under about half a day,
   - cites the criteria it satisfies, and
   - ends with a **Manual checklist** section.
4. **Implement** task by task, test-first: write a failing test named with the criterion ID, then make it pass.
5. **Finish:** run the manual checklist on Windows and on the Linux VM (X11, then Wayland where relevant), then mark the spec Done.
6. **Changing behavior later:** edit the spec first, add or retire criterion IDs (never renumber), then change the code.

## Templates

**requirements.md**
```markdown
# NN — <Feature> (PREFIX)
## Scope
In: … / Out: …
## Requirements
### PREFIX-1 <Name>
User story: As a <user>, I want <goal>, so that <benefit>.
- PREFIX-1.1 (auto) WHEN … THE SYSTEM SHALL …
- PREFIX-1.2 (manual) WHEN … THE SYSTEM SHALL …
## Open questions
```

**design.md:** Overview · Components & interfaces (Rust traits, Tauri commands, events) · Data models · Error handling · Platform notes · Decisions (with alternatives rejected) · Test strategy (which criteria are unit / integration / manual).

**tasks.md**
```markdown
- [ ] 1. <task> — covers PREFIX-1.1, PREFIX-1.2
## Manual checklist
- [ ] PREFIX-1.2 — Windows ☐ Linux X11 ☐ Wayland ☐
```
