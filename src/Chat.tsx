import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useMemo, useRef, useState, type MouseEvent } from "react";
import { errorText, formatSize, header, keyAction, type Conversation, type Delta, type Done, type Turn } from "./conversation";
import { render } from "./markdown";
import { useSettings } from "./useSettings";

const isBubble = getCurrentWindow().label === "bubble";

/** The conversation view, in the bubble and in the expanded window. Rust owns the conversation; this draws it. */
export default function Chat() {
  useSettings(); // follows the theme (BUB-8.2)
  const [conv, setConvState] = useState<Conversation | null>(null);
  const convRef = useRef<Conversation | null>(null);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string>();
  const rootRef = useRef<HTMLDivElement>(null);
  const turnsRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const setConv = (c: Conversation | null) => {
    convRef.current = c;
    setConvState(c);
  };
  const reload = () => invoke<Conversation | null>("get_conversation").then(setConv);

  useEffect(() => {
    if (!isBubble) reload();
    const subscriptions = [
      listen<Conversation>("bubble-open", (e) => {
        setConv(e.payload);
        setDraft("");
        setError(undefined);
      }),
      listen("bubble-hidden", () => setConv(null)), // the next open must not flash this conversation
      listen("chat-load", reload),
      listen<Delta>("chat-delta", (e) => {
        const c = convRef.current;
        const last = c?.turns.at(-1);
        if (!c || !last || c.id !== e.payload.id) return;
        if (last.answer.length !== e.payload.offset) return void reload(); // missed some text: fetch it all
        setConv(withLast(c, { answer: last.answer + e.payload.text }));
      }),
      listen<Done>("chat-done", (e) => {
        const c = convRef.current;
        if (!c || c.id !== e.payload.id) return;
        setConv({ ...withLast(c, { status: e.payload.status, error: e.payload.error }), dropped: e.payload.dropped });
      }),
    ];
    return () => subscriptions.forEach((s) => void s.then((off) => off()));
  }, []);

  // Focus the input on open (BUB-2.2).
  useEffect(() => inputRef.current?.focus(), [conv?.id]);

  // The bubble's content takes its natural height, capped at 520 px by CSS (then the turns scroll); the window follows it (BUB-2.5).
  useEffect(() => {
    const root = rootRef.current;
    if (!isBubble || !root) return;
    const observer = new ResizeObserver(() => void invoke("bubble_resize", { height: Math.ceil(root.getBoundingClientRect().height) }));
    observer.observe(root);
    return () => observer.disconnect();
  }, [conv !== null]);

  // Keep the newest text in view while the user is at the bottom.
  useEffect(() => {
    const t = turnsRef.current;
    if (t && t.scrollHeight - t.scrollTop - t.clientHeight < 60) t.scrollTop = t.scrollHeight;
  }, [conv]);

  if (!conv) return <div ref={rootRef} className="chat" />;

  const streaming = conv.turns.at(-1)?.status === "streaming";
  const h = header(conv);

  const send = (question: string) => {
    const q = question.trim();
    if (!q || streaming) return;
    const turn: Turn = { question: q, answer: "", status: "streaming", error: null, asked: Date.now(), answered: null };
    setConv({ ...conv, turns: [...conv.turns, turn] }); // before the first delta can arrive
    setDraft("");
    setError(undefined);
    invoke("ask", { question: q }).catch((e) => {
      setConv(conv);
      setDraft(q);
      setError(String(e));
    });
  };

  const regenerate = () => {
    setConv(withLast(conv, { answer: "", status: "streaming", error: null }));
    invoke("regenerate").catch((e) => {
      setError(String(e));
      reload();
    });
  };

  // Links open in the default browser, and only on click (BUB-4.2).
  const onAnswerClick = (e: MouseEvent) => {
    const a = (e.target as HTMLElement).closest("a");
    if (!a) return;
    e.preventDefault();
    void openUrl(a.href);
  };

  return (
    <div ref={rootRef} className={isBubble ? "chat bubble" : "chat"}>
      <header data-tauri-drag-region>
        <span data-tauri-drag-region className="who">{h.persona}</span>
        <span data-tauri-drag-region className="muted">{h.model}</span>
        {h.badge && <span data-tauri-drag-region className="badge">{h.badge}</span>}
        <span data-tauri-drag-region className="grow" />
        {isBubble && (
          <button type="button" className="icon" aria-label="Expand to a larger window" title="Expand" onClick={() => void invoke("expand")}>
            ⤢
          </button>
        )}
      </header>

      {!conv.model ? (
        <NoModel />
      ) : (
        <>
          <Context conv={conv} onRemove={() => invoke<Conversation | null>("remove_context").then(setConv)} />
          <div ref={turnsRef} className="turns" role="log" aria-label="Conversation" onClick={onAnswerClick}>
            {conv.dropped > 0 && <p className="muted note">Earlier messages were left out to fit the model.</p>}
            {conv.turns.map((t, i) => (
              <TurnView key={i} turn={t} latest={i === conv.turns.length - 1} onRetry={regenerate} />
            ))}
          </div>
          {error && <p role="alert" className="error note">{error}</p>}
          <footer>
            <textarea
              ref={inputRef}
              rows={2}
              value={draft}
              placeholder={conv.content.kind === "none" ? "Ask anything" : "Ask about the selection"}
              aria-label="Your question"
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => {
                if (keyAction({ key: e.key, shiftKey: e.shiftKey, isComposing: e.nativeEvent.isComposing }) === "send") {
                  e.preventDefault();
                  send(draft);
                }
              }}
            />
            {streaming ? (
              <button type="button" onClick={() => void invoke("stop")} aria-label="Stop the answer">
                Stop
              </button>
            ) : (
              <button type="button" onClick={() => send(draft)} disabled={!draft.trim()} aria-label="Send">
                Send
              </button>
            )}
          </footer>
        </>
      )}
    </div>
  );
}

function withLast(c: Conversation, patch: Partial<Turn>): Conversation {
  const last = c.turns.at(-1);
  return last ? { ...c, turns: [...c.turns.slice(0, -1), { ...last, ...patch }] } : c;
}

/** LLM-9.1 wording; the health check itself is spec 04. */
function NoModel() {
  return (
    <div className="nomodel" role="alert">
      <strong>No AI model is available</strong>
      <p className="muted">No model is set up yet.</p>
      <button type="button" autoFocus onClick={() => void invoke("open_setup")}>
        Open setup
      </button>
    </div>
  );
}

/** What was captured (BUB-3.1), how to drop it (BUB-3.2) and what was cut (BUB-3.3). */
function Context({ conv, onRemove }: { conv: Conversation; onRemove: () => void }) {
  const [open, setOpen] = useState(false);
  const { content, notice } = conv;
  const notes: string[] = [];
  if (notice?.kind === "elevated") notes.push("Cursora can't read apps running as administrator. Run Cursora as administrator to use it there.");
  if (notice?.kind === "noSelection") notes.push("Nothing was selected, so ask anything.");
  if (content.kind === "text" && content.truncated) notes.push(`Only the first ${content.text.length.toLocaleString()} characters were captured.`);
  if (content.kind === "files" && content.omitted > 0) notes.push(`${content.omitted} more items were left out.`);

  return (
    <section className="context" aria-label="Captured context">
      {content.kind !== "none" && (
        <div className="row">
          <span className="muted">From {conv.app.name || "an unknown app"}</span>
          {content.kind === "text" && <span className="muted">· {content.text.length.toLocaleString()} characters</span>}
          <span className="grow" />
          {conv.turns.length === 0 && (
            <button type="button" className="icon" aria-label="Remove the captured context" title="Remove" onClick={onRemove}>
              ×
            </button>
          )}
        </div>
      )}
      {content.kind === "text" && (
        <>
          <p className={open ? "preview" : "preview clamped"}>{content.text}</p>
          <button type="button" className="link" aria-expanded={open} onClick={() => setOpen(!open)}>
            {open ? "Show less" : "Show more"}
          </button>
        </>
      )}
      {content.kind === "files" && (
        <ul className="chips">
          {content.items.map((f) => (
            <li key={f.path} title={f.skipped ?? f.path}>
              {f.kind === "folder" ? "📁" : f.kind === "link" ? "🔗" : "📄"} {f.name}
              {f.kind === "file" && <span className="muted"> {formatSize(f.size)}</span>}
            </li>
          ))}
        </ul>
      )}
      {notes.map((n) => (
        <p key={n} className="muted note">{n}</p>
      ))}
    </section>
  );
}

function TurnView({ turn, latest, onRetry }: { turn: Turn; latest: boolean; onRetry: () => void }) {
  const html = useMemo(() => render(turn.answer), [turn.answer]);
  const [copied, setCopied] = useState(false);
  const answerRef = useRef<HTMLDivElement>(null);
  const err = turn.error && errorText(turn.error.kind, turn.error.message);

  // Markdown by default; Shift copies the rendered plain text (BUB-6.1).
  const copy = (e: MouseEvent) => {
    const text = e.shiftKey ? answerRef.current?.innerText ?? "" : turn.answer;
    void navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    });
  };

  return (
    <article className="turn">
      <p className="question">{turn.question}</p>
      {turn.status === "streaming" && !turn.answer && <p className="muted thinking">Thinking…</p>}
      {turn.answer && <div ref={answerRef} className="answer" aria-busy={turn.status === "streaming"} dangerouslySetInnerHTML={{ __html: html }} />}
      {turn.status === "cancelled" && <p className="muted note">Cancelled</p>}
      {err && (
        <div role="alert" className="error note">
          {err.text}
          <div className="row">
            <button type="button" onClick={onRetry}>Retry</button>
            {err.openSettings && (
              <button type="button" onClick={() => void invoke("open_settings", { tab: "Models" })}>
                Open Models settings
              </button>
            )}
          </div>
        </div>
      )}
      {turn.answer && turn.status !== "streaming" && (
        <div className="actions">
          <button type="button" className="link" onClick={copy} aria-label="Copy the answer. Hold Shift for plain text" title="Copy (Shift: plain text)">
            {copied ? "Copied" : "Copy"}
          </button>
          {latest && (
            <button type="button" className="link" onClick={onRetry} aria-label="Regenerate the answer">
              Regenerate
            </button>
          )}
        </div>
      )}
    </article>
  );
}
