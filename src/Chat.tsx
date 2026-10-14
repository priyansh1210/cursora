import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useMemo, useRef, useState, type MouseEvent } from "react";
import { badge, errorText, formatSize, header, keyAction, marks, type Asked, type Conversation, type Delta, type Done, type Preview, type Turn } from "./conversation";
import { render } from "./markdown";
import { ModelSelect, modelValue, useEnabledModels } from "./Models";
import { listKey, matches, typedCommand, type Entry } from "./skills";
import { useSettings, type Settings } from "./useSettings";

const isBubble = getCurrentWindow().label === "bubble";

/** The conversation view, in the bubble and in the expanded window. Rust owns the conversation; this draws it. */
export default function Chat() {
  const { settings, save } = useSettings(); // also follows the theme (BUB-8.2)
  const models = useEnabledModels(settings);
  const [conv, setConvState] = useState<Conversation | null>(null);
  const convRef = useRef<Conversation | null>(null);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string>();
  // "What will be sent" (PRI-3.6); `pending`: a cloud request waiting for Send (PRI-3.7).
  const [outgoing, setOutgoing] = useState<{ open: boolean; pending: { question: string; retry: boolean } | null }>({ open: false, pending: null });
  const rootRef = useRef<HTMLDivElement>(null);
  const turnsRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  // The / autocomplete (SKL-2.1): Rust gives the entries once a command starts; filtering is local.
  const [entries, setEntries] = useState<Entry[]>([]);
  const [selected, setSelected] = useState(0);
  const [listClosed, setListClosed] = useState(false);
  // `/snip <question>`: sent as if typed once the snip's conversation is here, so confirming still applies.
  const [snipQuestion, setSnipQuestion] = useState<string | null>(null);
  const sendRef = useRef<(question: string) => void>(undefined);
  const typed = typedCommand(draft);
  useEffect(() => {
    if (typed !== null) void invoke<Entry[]>("skills_available").then(setEntries);
  }, [typed !== null]);
  const list = typed !== null && !listClosed ? matches(entries, typed) : [];
  const active = Math.min(selected, Math.max(list.length - 1, 0));
  useEffect(() => {
    // A block body: newer WebView2 returns a Promise from scrollIntoView, which React would take for a cleanup function.
    document.getElementById(`skill-${list[active]?.name}`)?.scrollIntoView({ block: "nearest" });
  }, [active, list.length]);

  const setConv = (c: Conversation | null) => {
    convRef.current = c;
    setConvState(c);
  };
  const reload = () => invoke<Conversation | null>("get_conversation").then(setConv);

  useEffect(() => {
    const subscriptions = [
      listen<Conversation>("bubble-open", (e) => {
        setConv(e.payload);
        setDraft("");
        setError(undefined);
        setOutgoing({ open: false, pending: null });
      }),
      listen("bubble-hidden", () => setConv(null)), // the next open must not flash this conversation
      listen("chat-load", reload),
      listen<string>("snip-ask", (e) => setSnipQuestion(e.payload)),
      // Personality or the persona changed: the header shows who answers the next message (PER-2.2).
      listen<string | null>("chat-persona", (e) => convRef.current && setConv({ ...convRef.current, persona: e.payload })),
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
    // Once listening, fetch what is already there: `cursora --ask` can open the bubble before this page loads (APP-2.3).
    void Promise.all(subscriptions).then(reload);
    return () => subscriptions.forEach((s) => void s.then((off) => off()));
  }, []);

  // Focus the input on open (BUB-2.2).
  useEffect(() => inputRef.current?.focus(), [conv?.id]);

  useEffect(() => {
    if (snipQuestion === null || conv?.content.kind !== "snip") return;
    sendRef.current?.(snipQuestion);
    setSnipQuestion(null);
  }, [snipQuestion, conv?.id]);

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
  const h = header(conv, settings?.localOnly);

  const send = (question: string, confirmed = false) => {
    const q = question.trim();
    if (!q || streaming) return;
    const turn: Turn = { question: q, answer: "", status: "streaming", error: null, asked: Date.now(), answered: null, local: false, note: null };
    setConv({ ...conv, turns: [...conv.turns, turn] }); // before the first delta can arrive
    setDraft("");
    setError(undefined);
    setOutgoing((o) => ({ ...o, pending: null }));
    invoke<Asked>("ask", { question: q, confirmed }).then(
      (asked) => {
        if (asked !== "confirm") return;
        setConv(conv);
        setDraft(q);
        setOutgoing({ open: true, pending: { question: q, retry: false } });
      },
      (e) => {
        setConv(conv);
        setDraft(q);
        setError(String(e));
      },
    );
  };

  sendRef.current = send;

  const regenerate = (confirmed = false) => {
    const question = conv.turns.at(-1)?.question ?? "";
    setConv(withLast(conv, { answer: "", status: "streaming", error: null }));
    setOutgoing((o) => ({ ...o, pending: null }));
    invoke<Asked>("regenerate", { confirmed }).then(
      (asked) => {
        if (asked !== "confirm") return;
        setConv(conv);
        setOutgoing({ open: true, pending: { question, retry: true } });
      },
      (e) => {
        setError(String(e));
        reload();
      },
    );
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
        {settings && (
          <button
            type="button"
            className="icon"
            aria-label="Personality"
            aria-pressed={settings.personalityOn}
            title={settings.personalityOn ? "Personality on. Turn it off for the plain model" : "Personality off. Turn it on"}
            onClick={() => save({ personalityOn: !settings.personalityOn }).catch((e) => setError(String(e)))}
          >
            🎭
          </button>
        )}
        {models.length > 1 && settings ? (
          <ModelSelect
            label="Model"
            models={models}
            value={modelValue(conv.ownModel ?? settings.models.default)}
            onChange={(provider, model) => invoke<Conversation | null>("set_model", { provider, model }).then((c) => c && setConv(c), (e) => setError(String(e)))}
          />
        ) : (
          <span data-tauri-drag-region className="muted">{h.model}</span>
        )}
        {h.badge && <span data-tauri-drag-region className="badge">{h.badge}</span>}
        <span data-tauri-drag-region className="grow" />
        {isBubble && (
          <button type="button" className="icon" aria-label="Expand to a larger window" title="Expand" onClick={() => void invoke("expand")}>
            ⤢
          </button>
        )}
      </header>

      {!conv.model || conv.unavailable ? (
        <NoModel reason={conv.unavailable} />
      ) : (
        <>
          <Context
            conv={conv}
            onRemove={() =>
              invoke<Conversation | null>("remove_context").then((c) => {
                setConv(c);
                inputRef.current?.focus(); // the × button is gone, and focus with it (BUB-8.1)
              })
            }
          />
          <div ref={turnsRef} className="turns" role="log" aria-label="Conversation" onClick={onAnswerClick}>
            {conv.dropped > 0 && <p className="muted note">Earlier messages were left out to fit the model.</p>}
            {conv.turns.map((t, i) => (
              <TurnView key={i} turn={t} latest={i === conv.turns.length - 1} onRetry={() => regenerate()} />
            ))}
          </div>
          {error && <p role="alert" className="error note">{error}</p>}
          <Outgoing
            settings={settings}
            question={outgoing.pending?.question ?? draft}
            retry={outgoing.pending?.retry ?? false}
            open={outgoing.open}
            pending={outgoing.pending !== null}
            localOnly={settings?.localOnly ?? false}
            onToggle={() => setOutgoing((o) => ({ open: !o.open, pending: null }))}
            onSend={() => {
              const p = outgoing.pending;
              if (p?.retry) regenerate(true);
              else if (p) send(p.question, true);
            }}
            onCancel={() => setOutgoing({ open: false, pending: null })}
          />
          {list.length > 0 && (
            <ul id="skill-list" role="listbox" aria-label="Skills and commands" className="complete">
              {list.map((e, i) => (
                <li
                  key={e.name}
                  id={`skill-${e.name}`}
                  role="option"
                  aria-selected={i === active}
                  onMouseDown={(ev) => {
                    ev.preventDefault(); // keeps the focus in the input
                    setDraft(`/${e.name} `);
                  }}
                >
                  <strong>/{e.name}</strong>
                  {e.args && <span className="muted"> {e.args}</span>}
                  <span className="muted"> {e.description}</span>
                </li>
              ))}
            </ul>
          )}
          <footer>
            <textarea
              ref={inputRef}
              rows={2}
              value={draft}
              placeholder={conv.content.kind === "none" ? "Ask anything" : conv.content.kind === "snip" ? "Ask about the snip" : "Ask about the selection"}
              aria-label="Your question"
              aria-autocomplete="list"
              aria-controls={list.length ? "skill-list" : undefined}
              aria-activedescendant={list.length ? `skill-${list[active].name}` : undefined}
              onChange={(e) => {
                setDraft(e.target.value);
                setSelected(0);
                setListClosed(false);
                setOutgoing((o) => ({ ...o, pending: null })); // editing the question cancels a waiting Send
              }}
              onKeyDown={(e) => {
                const k = list.length && !e.shiftKey && !e.nativeEvent.isComposing ? listKey(e.key, active, list.length, list[active].name === typed) : null;
                if (k !== null) {
                  e.preventDefault();
                  if (k === "complete") setDraft(`/${list[active].name} `);
                  else if (k === "close") setListClosed(true);
                  else setSelected(k);
                  return;
                }
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

/** "What will be sent" (PRI-3.6), and the Send that a cloud request waits for while confirming is on (PRI-3.7). */
function Outgoing(props: { settings?: Settings; question: string; retry: boolean; open: boolean; pending: boolean; localOnly: boolean; onToggle: () => void; onSend: () => void; onCancel: () => void }) {
  const { question, retry, open, pending } = props;
  const [preview, setPreview] = useState<Preview>();
  const [error, setError] = useState<string>();
  useEffect(() => {
    if (!open) return;
    const timer = setTimeout(() => {
      invoke<Preview>("send_preview", { question, retry }).then(
        (p) => {
          setPreview(p);
          setError(undefined);
        },
        (e) => setError(String(e)),
      );
    }, 250);
    return () => clearTimeout(timer);
  }, [open, question, retry, props.settings]); // the settings: another model, or other redaction, changes what goes out
  const to = preview?.to;
  // The selection and the question come last, and they are what the user checks.
  const sentRef = useRef<HTMLPreElement>(null);
  useEffect(() => {
    if (sentRef.current) sentRef.current.scrollTop = sentRef.current.scrollHeight;
  }, [preview]);

  return (
    <section className="outgoing" aria-label="What will be sent">
      <button type="button" className="link" aria-expanded={open} onClick={props.onToggle}>
        What will be sent {open ? "▾" : "▸"}
      </button>
      {open && error && <p className="error note">{error}</p>}
      {open && !error && preview && !to && <p className="muted note">Cursora answers this itself; nothing is sent.</p>}
      {open && !error && to && (
        <>
          <p className="muted note">
            To {badge(to.locality, to.provider, props.localOnly)} · {to.name}
          </p>
          {preview.imageWarning && <p role="alert" className="error note">Images are sent as they are: redaction covers text only.</p>}
          <pre ref={sentRef} className="sent">
            {marks(preview.text, preview.placeholders).map((m, i) => (m.mark ? <mark key={i}>{m.text}</mark> : <span key={i}>{m.text}</span>))}
          </pre>
          {pending && (
            <div className="row">
              <button type="button" autoFocus onClick={props.onSend}>
                Send to {to.provider}
              </button>
              <button type="button" onClick={props.onCancel}>
                Cancel
              </button>
            </div>
          )}
        </>
      )}
    </section>
  );
}

function withLast(c: Conversation, patch: Partial<Turn>): Conversation {
  const last = c.turns.at(-1);
  return last ? { ...c, turns: [...c.turns.slice(0, -1), { ...last, ...patch }] } : c;
}

/** LLM-9.1: instead of the input, with the reason. Everything else keeps working. */
function NoModel({ reason }: { reason?: string }) {
  return (
    <div className="nomodel" role="alert">
      <strong>No AI model is available</strong>
      <p className="muted">{reason ?? "No model is set up yet."}</p>
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
  if (notice?.kind === "blocked") notes.push(`Cursora is blocked in ${conv.app.name || conv.app.exe}, so nothing was read.`);
  if (notice?.kind === "elevated") notes.push("Cursora can't read apps running as administrator. Run Cursora as administrator to use it there.");
  if (notice?.kind === "noSelection") notes.push("Nothing was selected, so ask anything.");
  if (notice?.kind === "unreadable") notes.push("No selection could be read in this session."); // LNX-3.4
  if (notice?.kind === "snipFailed") notes.push(notice.reason);
  if (content.kind === "text" && content.truncated) notes.push(`Only the first ${content.text.length.toLocaleString()} characters were captured.`);
  if (content.kind === "files" && content.omitted > 0) notes.push(`${content.omitted} more items were left out.`);
  const offer = notice?.kind === "needsVision" ? notice.offer : null;

  return (
    <section className="context" aria-label="Captured context">
      {content.kind !== "none" && (
        <div className="row">
          <span className="muted">{content.kind === "snip" ? `Snip · ${content.width} × ${content.height}` : `From ${conv.app.name || "an unknown app"}`}</span>
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
            <li key={f.path} title={f.image ? "Attached as an image" : f.skipped ?? f.path}>
              {f.kind === "folder" ? "📁" : f.kind === "link" ? "🔗" : f.image ? "🖼️" : "📄"} {f.name}
              {f.kind === "file" && <span className="muted"> {formatSize(f.size)}</span>}
            </li>
          ))}
        </ul>
      )}
      {content.kind === "snip" && (
        <img className="snip-preview" src={`data:${content.thumbnail.mime};base64,${content.thumbnail.base64}`} alt={`The snip, ${content.width} by ${content.height} pixels`} />
      )}
      {notes.map((n) => (
        <p key={n} className="muted note">{n}</p>
      ))}
      {notice?.kind === "noSelection" && notice.offerSnip && (
        <button type="button" className="link" onClick={() => void invoke("snip_start", {})}>
          Snip an area instead
        </button>
      )}
      {notice?.kind === "needsVision" && (
        <div className="note">
          <p className="muted">Snipping needs a model that can see images.</p>
          {offer ? (
            <button type="button" onClick={() => void invoke("snip_start", { model: offer.model })}>
              Use {offer.label} for this snip
            </button>
          ) : (
            <button type="button" className="link" onClick={() => void invoke("open_settings", { tab: "Models" })}>
              Add one in Models
            </button>
          )}
        </div>
      )}
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
    <article className={turn.local ? "turn local" : "turn"}>
      <p className="question">{turn.question}</p>
      {turn.note && <p className="muted note">{turn.note}</p>}
      {turn.status === "streaming" && !turn.answer && <p className="muted thinking">Thinking…</p>}
      {turn.answer && <div ref={answerRef} className="answer" aria-busy={turn.status === "streaming"} dangerouslySetInnerHTML={{ __html: html }} />}
      {turn.status === "cancelled" && <p className="muted note">Cancelled</p>}
      {err && (
        <div role="alert" className="error note">
          {err.text}
          {/* The provider's own words, e.g. why the built-in engine stopped (LLM-3.6) or the token counts of a too-long request. */}
          {turn.error?.kind !== "unknown" && turn.error?.message && <pre className="detail muted">{turn.error.message}</pre>}
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
      {turn.answer && turn.status !== "streaming" && !turn.local && (
        <div className="actions">
          <button type="button" className="link" onClick={copy} aria-label="Copy the answer. Hold Shift for plain text" title="Copy (Shift: plain text)">
            {copied ? "Copied" : "Copy"}
          </button>
          {latest && (
            <button type="button" className="link" onClick={onRetry} aria-label="Regenerate the answer">
              Regenerate
            </button>
          )}
          {/* SKL-4.3: the question becomes a skill's instructions in Settings → Skills. */}
          {!turn.question.startsWith("/") && (
            <button type="button" className="link" onClick={() => void invoke("skill_new", { question: turn.question })}>
              Save as skill
            </button>
          )}
        </div>
      )}
    </article>
  );
}
