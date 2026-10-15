# 03: Bubble UI (BUB)

## Scope
- **In:**
  - The chat bubble's window, its placement next to the cursor, and its open/close behavior
  - Showing the captured context, asking, streaming answers and follow-ups
  - Answer actions, the status badges, keyboard use, and conversation history
- **Out:**
  - Slash-command semantics (SKL)
  - Prompt contents (PER)
  - Provider calls (LLM)
  - The redaction preview contents (PRI)
  - Speech (VOC)

## Requirements

### BUB-1 Placement
User story: As a user, I want the chat box to appear right at my cursor, so that I don't lose my place.
- BUB-1.1 (auto) WHEN the bubble opens THE SYSTEM SHALL place its top-left corner at cursor + (16, 16), in physical pixels.
- BUB-1.2 (auto) IF the bubble would extend beyond the work area of the monitor that contains the cursor THE SYSTEM SHALL first flip it to the other side of the cursor (left and/or above), then clamp it fully inside that work area.
- BUB-1.3 (auto) Placement SHALL use the scale factor of the monitor that contains the cursor, and SHALL be correct on mixed-DPI multi-monitor setups.
- BUB-1.4 (auto) WHERE the cursor position is unavailable (e.g. Wayland) THE SYSTEM SHALL center the bubble on the active monitor, or else on the primary monitor.
- BUB-1.5 (manual) On a multi-monitor setup THE bubble SHALL open on the monitor that contains the cursor, fully visible near every edge and corner.

### BUB-2 Window behavior
- BUB-2.1 (manual) THE bubble window SHALL be:
  - frameless, with rounded corners
  - always on top
  - absent from the taskbar and Alt+Tab
  - created at startup and kept hidden until needed
- BUB-2.2 (manual) WHEN the bubble opens THE input SHALL have keyboard focus.
- BUB-2.3 (manual) THE bubble SHALL stay open when it loses focus or Esc is pressed. It SHALL close **only** when the user presses the ask hotkey (SEL-1.4), and focus SHALL then return to the previously active window.
- BUB-2.4 (manual) THE user SHALL be able to drag the bubble by its header.
- BUB-2.5 (manual) THE bubble SHALL default to 380 px wide and grow with its content up to 520 px tall, then scroll. **Expand** SHALL open the same conversation in a larger, resizable window.
- BUB-2.6 (auto) WHEN the bubble is closed while an answer is streaming THE SYSTEM SHALL cancel the request. The partial answer SHALL be kept, marked **"Cancelled"**, in the conversation and in history.

### BUB-3 Context display
- BUB-3.1 (manual) THE bubble SHALL show what was captured:
  - **Text:** a collapsed two-line preview with an expand toggle and a character count.
  - **Files:** chips showing an icon, name and size.
  - **Images:** thumbnails.
  - **Source:** the source app's name.
- BUB-3.2 (manual) Until the first question is sent, THE user SHALL be able to remove the attached context (×) and ask a general question instead.
- BUB-3.3 (manual) WHEN the capture was truncated or files were left out THE bubble SHALL say so (SEL-2.4, SEL-5.3, SEL-5.5).
- BUB-3.4 (retired) Pasting context into the bubble. Removed: Cursora works on the selection, so text is never copied into the chat by hand.

### BUB-4 Asking and streaming
- BUB-4.1 (auto) WHEN the user presses Enter (Shift+Enter inserts a newline) THE SYSTEM SHALL send the request and render tokens as they arrive.
- BUB-4.2 (auto) THE answer SHALL be rendered as Markdown, with syntax-highlighted code blocks.
  - Raw HTML in model output SHALL NOT be executed or rendered.
  - Links SHALL open in the default browser only when clicked.
  - Images in model output SHALL NOT be loaded; they show as text (PRI-4.1).
- BUB-4.3 (manual) A Stop button SHALL cancel streaming and keep the partial answer.
- BUB-4.4 (auto) IF the request fails THE bubble SHALL show a plain-language message for the error kind (auth, rate limit, out of credit, model not found, context too long, network, server, blocked by local-only), with Retry. For auth, out-of-credit and configuration errors it SHALL also offer "Open Models settings". (Amended 2026-10-15: out of credit, LLM-1.4.)
- BUB-4.5 (manual) A "thinking" indicator SHALL show until the first token arrives.

### BUB-5 Follow-ups
- BUB-5.1 (auto) WHEN the user asks a follow-up THE SYSTEM SHALL send the conversation so far, with the captured context included exactly once.
- BUB-5.2 (auto) WHEN the conversation exceeds the model's context budget THE SYSTEM SHALL drop the oldest question/answer pairs first. It SHALL never drop the system prompt, the captured context or the latest question, and SHALL show that earlier turns were dropped.
- BUB-5.3 (manual) WHEN the hotkey is pressed with a new capture THE SYSTEM SHALL start a new conversation. The previous one stays in History.

### BUB-6 Answer actions
- BUB-6.1 (manual) Each answer SHALL offer:
  - **Copy:** Markdown, or plain text when Shift is held
  - **Regenerate**, on the latest answer only
  - **Speak:** when speech is available (VOC)

  Each question SHALL offer **Save as skill** (SKL-4.3).

### BUB-7 Status header
- BUB-7.1 (auto) THE bubble header SHALL show:
  - the active persona's name, or "Personality off"
  - the model name
  - a badge: "🔒 Local" for a provider on this machine (LLM-8.1), "🏠 LAN" for a trusted LAN host (LLM-8.2), otherwise "☁ <provider name>"
- BUB-7.2 (retired) The quick model switcher. Moved to spec 04 as LLM-5.5, since it needs configured models.
- BUB-7.3 (manual) THE header of the bubble and of the expanded window SHALL start with the Cursora logo. The logo is decorative: screen readers skip it, and it drags the bubble like the rest of the header. (Added 2026-10-15 at the user's request.)

### BUB-8 Keyboard and accessibility
- BUB-8.1 (manual) Every bubble control SHALL be reachable with the keyboard, and all buttons SHALL have accessible labels for screen readers.
- BUB-8.2 (manual) THE bubble SHALL follow the theme setting (system / light / dark).

### BUB-9 Conversation history
- BUB-9.1 (auto) WHERE "Save history" is on (the default) THE SYSTEM SHALL store each conversation locally in SQLite: questions, answers, captured context, source app, model, persona and timestamps.
- BUB-9.2 (manual) THE History tab SHALL:
  - list conversations, newest first
  - search their text
  - reopen a conversation in the expanded window
  - delete one conversation, or clear all
- BUB-9.3 (auto) WHILE "Save history" is off THE SYSTEM SHALL write no conversation data to disk.
- BUB-9.4 (auto) THE SYSTEM SHALL delete conversations older than the retention period (default 90 days; configurable, including "forever").

## Open questions
- _(none.)_

## Resolved
- Closing the bubble mid-answer cancels it, and the partial answer is shown marked "Cancelled" (BUB-2.6).
- The bubble closes only with the ask hotkey. Esc and clicking elsewhere don't close it, so a pin button isn't needed (BUB-2.3, BUB-2.4).
