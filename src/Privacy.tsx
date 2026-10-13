import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type FormEvent } from "react";
import type { Rule, Save, Settings as S } from "./useSettings";

type Detector = { id: string; name: string };
type RuleKind = "process" | "title";
const RULE_KINDS: Record<RuleKind, string> = { process: "Process name", title: "Window title contains" };
const kindOf = (r: Rule): RuleKind => ("process" in r ? "process" : "title");
const valueOf = (r: Rule) => ("process" in r ? r.process : r.title);
const rule = (kind: RuleKind, value: string): Rule => (kind === "process" ? { process: value } : { title: value });

/** Settings → Privacy (spec 07). A refused save shows its reason at the top of the tab. */
export default function Privacy({ settings, save }: { settings: S; save: Save }) {
  const [detectors, setDetectors] = useState<Detector[]>([]);
  const [identifiesApps, setIdentifiesApps] = useState(true);
  const [problem, setProblem] = useState<string | null>(null);
  useEffect(() => {
    void invoke<Detector[]>("privacy_detectors").then(setDetectors);
    void invoke<{ identifiesApps: boolean }>("app_info").then((i) => setIdentifiesApps(i.identifiesApps));
  }, []);
  useEffect(() => {
    void invoke<string | null>("model_problem").then(setProblem);
  }, [settings]);

  const p = settings.privacy;
  const rules = settings.capture.blocklist;
  const set = (patch: Partial<S>) => save(patch).catch(() => {});
  const setPrivacy = (patch: Partial<S["privacy"]>) => save({ privacy: { ...p, ...patch } });
  const setRules = (blocklist: Rule[]) => set({ capture: { ...settings.capture, blocklist } });

  return (
    <>
      <h2>Local-only</h2>
      <label className="row">
        <input type="checkbox" checked={settings.localOnly} onChange={(e) => set({ localOnly: e.target.checked })} />
        Local-only: nothing leaves this computer
      </label>
      <p className="muted">Only models on this computer, or on a LAN host you trust, can answer. Cloud models are turned off.</p>
      {settings.localOnly && problem && (
        <p role="alert" className="error">
          {problem}{" "}
          <button type="button" className="link" onClick={() => void invoke("open_settings", { tab: "Models" })}>
            Open Models
          </button>
        </p>
      )}

      <h2>Blocked apps</h2>
      <p className="muted">Cursora reads nothing in these apps: no text, no keys, no clipboard.</p>
      {/* PRI-2.5 */}
      {!identifiesApps && <p role="alert" className="error">Cursora can't tell which app is in front in this session, so the blocklist can't be enforced.</p>}
      {rules.map((r, i) => (
        <RuleRow
          key={`${i}:${kindOf(r)}:${valueOf(r)}`}
          rule={r}
          onChange={(changed) => setRules(changed ? rules.map((x, j) => (j === i ? changed : x)) : rules.filter((_, j) => j !== i))}
        />
      ))}
      <AddRule onAdd={(r) => setRules([...rules, r])} />

      <h2>Hide personal details</h2>
      <label className="row">
        <input type="checkbox" checked={p.redact} onChange={(e) => void setPrivacy({ redact: e.target.checked }).catch(() => {})} />
        Before sending to cloud models
      </label>
      <fieldset disabled={!p.redact}>
        <legend className="muted">What to hide</legend>
        {detectors.map((d) => (
          <label className="row" key={d.id}>
            <input
              type="checkbox"
              checked={!p.detectorsOff.includes(d.id)}
              onChange={(e) => void setPrivacy({ detectorsOff: e.target.checked ? p.detectorsOff.filter((x) => x !== d.id) : [...p.detectorsOff, d.id] }).catch(() => {})}
            />
            {d.name}
          </label>
        ))}
        {p.patterns.map((pat, i) => (
          <div className="row" key={`${i}:${pat.label}`}>
            <code>{pat.label}</code>
            <code>{pat.regex}</code>
            <button type="button" aria-label={`Remove the pattern ${pat.label}`} onClick={() => void setPrivacy({ patterns: p.patterns.filter((_, j) => j !== i) }).catch(() => {})}>
              ×
            </button>
          </div>
        ))}
        {/* Saved directly, so a refused pattern shows its reason here only, not also at the top of the tab. */}
        <AddPattern onAdd={(pat) => invoke<void>("set_settings", { settings: { ...settings, privacy: { ...p, patterns: [...p.patterns, pat] } } })} />
        <label className="row">
          <input type="checkbox" checked={p.redactLocal} onChange={(e) => void setPrivacy({ redactLocal: e.target.checked }).catch(() => {})} />
          For local models too
        </label>
      </fieldset>

      <h2>Before sending</h2>
      <label className="row">
        <input type="checkbox" checked={p.confirmCloud} onChange={(e) => void setPrivacy({ confirmCloud: e.target.checked }).catch(() => {})} />
        Confirm before sending to cloud models
      </label>
      <p className="muted">The bubble shows what will be sent and waits for you to press Send.</p>
    </>
  );
}

/** One rule; an emptied value removes it (PRI-2.4). */
function RuleRow({ rule: r, onChange }: { rule: Rule; onChange: (r: Rule | null) => void }) {
  const kind = kindOf(r);
  const value = valueOf(r);
  return (
    <div className="row">
      <select aria-label={`Kind of rule for ${value}`} value={kind} onChange={(e) => onChange(rule(e.target.value as RuleKind, value))}>
        {Object.entries(RULE_KINDS).map(([k, label]) => (
          <option key={k} value={k}>
            {label}
          </option>
        ))}
      </select>
      <input
        aria-label={`${RULE_KINDS[kind]}: ${value}`}
        defaultValue={value}
        onBlur={(e) => {
          const v = e.target.value.trim();
          if (v !== value) onChange(v ? rule(kind, v) : null);
        }}
      />
      <button type="button" aria-label={`Remove ${value}`} onClick={() => onChange(null)}>
        ×
      </button>
    </div>
  );
}

function AddRule({ onAdd }: { onAdd: (r: Rule) => void }) {
  const [kind, setKind] = useState<RuleKind>("process");
  const [value, setValue] = useState("");
  const submit = (e: FormEvent) => {
    e.preventDefault();
    if (value.trim()) onAdd(rule(kind, value.trim()));
    setValue("");
  };
  return (
    <form className="row" onSubmit={submit}>
      <select aria-label="Kind of rule to add" value={kind} onChange={(e) => setKind(e.target.value as RuleKind)}>
        {Object.entries(RULE_KINDS).map(([k, label]) => (
          <option key={k} value={k}>
            {label}
          </option>
        ))}
      </select>
      <input aria-label="App or title to block" placeholder={kind === "process" ? "app.exe" : "a word in the title"} value={value} onChange={(e) => setValue(e.target.value)} />
      <button type="submit">Add</button>
    </form>
  );
}

/** A label and a regex; a bad one is refused by Rust with the reason, and stays here to fix (PRI-3.4). */
function AddPattern({ onAdd }: { onAdd: (p: { label: string; regex: string }) => Promise<void> }) {
  const [label, setLabel] = useState("");
  const [regex, setRegex] = useState("");
  const [error, setError] = useState<string>();
  const submit = (e: FormEvent) => {
    e.preventDefault();
    onAdd({ label, regex }).then(
      () => {
        setLabel("");
        setRegex("");
        setError(undefined);
      },
      (err) => setError(String(err)),
    );
  };
  return (
    <form onSubmit={submit}>
      <div className="row">
        <input aria-label="Pattern label" placeholder="LABEL" value={label} onChange={(e) => setLabel(e.target.value.toUpperCase())} />
        <input aria-label="Pattern (regular expression)" placeholder="EMP-\d{6}" value={regex} onChange={(e) => setRegex(e.target.value)} />
        <button type="submit" disabled={!label || !regex}>
          Add pattern
        </button>
      </div>
      {error && <p role="alert" className="error note">{error}</p>}
    </form>
  );
}
