# 26: Web search with sources (WEB)

Status: **Approved**¹ (2026-10-29).

## Scope
- **In:** `/web <question>` in the bubble. Cursora searches the web with **Tavily**, gives the results to whichever model answers (local ones too), and lists the sources as links under the answer.
- **Out:**
  - Searching by itself, without `/web`: the user decides each time that a question goes to the web
  - Reading whole pages: the model gets each result's title, address and a passage
  - Other search services, and the cloud providers' own search tools (they don't work with local models)
  - Web search in local-only mode

Update #7 from the list of 2026-10-28 ([improvements batch 2](../../memory/improvements-batch-2.md)).

## Requirements

### WEB-1 Ask with the web
User story: As a user asking about something recent (a release, an error message, a price), I want the answer to come from current web pages, with links, so I can trust it and read more.
- WEB-1.1 (manual) Settings → Models SHALL have **Web search**: a field for a Tavily API key, kept in the OS keychain like the providers' keys (LLM-6.1), with where to get a free one and a line saying that a `/web` question goes to Tavily.
- WEB-1.2 (auto) WHEN the user asks `/web <question>` THE SYSTEM SHALL search Tavily for the question followed by the selected text, on one line, up to 400 characters, so a question about the selection names its subject; add up to 5 results (title, address, passage) to the request as untrusted data in `<search_results>` (constitution §5); tell the model to answer from them and cite them as [1] to [5]; and list the sources as links under the answer.
- WEB-1.3 (auto) Personal details in the search query SHALL be hidden as for a cloud model (PRI-3.1). "What will be sent" (PRI-3.6) SHALL show the query and that it goes to Tavily. WHILE confirming before cloud requests is on (PRI-3.7), a `/web` question SHALL wait for Send, even with a local model.
- WEB-1.4 (auto) WHILE local-only mode is on, `/web` SHALL send nothing and say that web search is off in local-only mode. Without a key, it SHALL say where to add one. With nothing to search for, it SHALL say to type a question or select text.
- WEB-1.5 (auto) IF the search fails (no connection, a refused key, the month's searches used up) THE turn SHALL show the error like a model's (LLM-1.4), and nothing SHALL be sent to the model.
- WEB-1.6 (auto) `/web` SHALL be listed in `/help` and the command list, and no skill can take its name (SKL-2.6).

## Defaults taken under the go-ahead of 2026-10-28
1. **Tavily**, with the user's own key: its free plan has 1,000 searches a month and needs no credit card (checked on 2026-10-29 on third-party pricing pages), and it returns clean passages made for models. Rejected: DuckDuckGo without a key, which answered the second test query with a bot check; Brave's API, which now needs a credit card even for its free credit.
2. **Only with `/web`**: nothing is searched unless the user asks for it in that question.
3. **Off in local-only mode**, even with a local model: the query leaves the computer.
4. **Constitution §3 amended** (2026-10-29): a search service the user set up counts as a configured provider, used only when the user asks with `/web`.
5. **Up to 5 results of up to 600 characters each**: about 1,000 tokens, which fits the built-in engine's context with room for the answer.
6. **Sources added by Cursora**, not written by the model, so the links are always the real ones.
7. **The query is the question, then the selection**: the first check searched only "when was it released as stable?" for a selected "Tauri 2". Settings says that the start of the selection goes to Tavily too.

## Open questions
None.

¹ Approved under the user's go-ahead of 2026-10-28 to design, build and merge updates 1 and 3 to 9, with the user reviewing everything afterwards. The choices made for the user are listed above.
