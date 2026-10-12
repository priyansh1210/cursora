import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type { Save, Settings } from "./useSettings";

type Summary = { id: number; updated: number; title: string; app: string; model: string | null };

const RETENTION: [number | null, string][] = [
  [30, "30 days"],
  [90, "90 days"],
  [365, "1 year"],
  [null, "Forever"],
];

/** Settings → History (BUB-9.2), plus the saving and retention settings (BUB-9.3, 9.4). */
export default function History({ settings, save }: { settings: Settings; save: Save }) {
  const [query, setQuery] = useState("");
  const [items, setItems] = useState<Summary[]>([]);
  const [more, setMore] = useState(false);
  const [error, setError] = useState<string>();
  const [confirming, setConfirming] = useState(false);
  const h = settings.history;
  const set = (patch: Partial<Settings["history"]>) => save({ history: { ...h, ...patch } }).catch(() => {});

  const load = (offset = 0) =>
    invoke<Summary[]>("history_list", { query, offset }).then(
      (page) => {
        setItems((prev) => (offset ? [...prev, ...page] : page));
        setMore(page.length === 50);
        setError(undefined);
      },
      (e) => setError(String(e)),
    );
  const run = (cmd: string, args?: Record<string, unknown>) => invoke(cmd, args).then(() => load(), (e) => setError(String(e)));

  useEffect(() => void load(), [query]);

  return (
    <>
      <h2>History</h2>
      <label className="row">
        <input type="checkbox" checked={h.save} onChange={(e) => set({ save: e.target.checked })} />
        Save conversations on this computer
      </label>
      <label className="row">
        Keep them for
        <select value={String(h.retentionDays)} onChange={(e) => set({ retentionDays: e.target.value === "null" ? null : Number(e.target.value) })}>
          {RETENTION.map(([days, label]) => (
            <option key={label} value={String(days)}>
              {label}
            </option>
          ))}
        </select>
      </label>
      <input type="search" placeholder="Search questions, answers and context" aria-label="Search history" value={query} onChange={(e) => setQuery(e.target.value)} />
      {error && <p role="alert" className="error">{error}</p>}
      <ul className="history">
        {items.map((s) => (
          <li key={s.id}>
            <button type="button" className="link" onClick={() => run("history_open", { id: s.id })}>
              {s.title || "(no question)"}
            </button>
            <span className="muted">
              {s.app || "Unknown app"} · {new Date(s.updated).toLocaleString()}
            </span>
            <button type="button" className="icon" aria-label={`Delete "${s.title}"`} onClick={() => run("history_delete", { id: s.id })}>
              ×
            </button>
          </li>
        ))}
      </ul>
      {items.length === 0 && <p className="muted">{query ? "No matches." : "No conversations yet."}</p>}
      {more && (
        <button type="button" onClick={() => load(items.length)}>
          Show older
        </button>
      )}
      {items.length > 0 &&
        (confirming ? (
          <p className="row">
            Delete every saved conversation?
            <button type="button" onClick={() => (setConfirming(false), run("history_clear"))}>
              Delete all
            </button>
            <button type="button" autoFocus onClick={() => setConfirming(false)}>
              Cancel
            </button>
          </p>
        ) : (
          <button type="button" onClick={() => setConfirming(true)}>
            Clear all…
          </button>
        ))}
    </>
  );
}
