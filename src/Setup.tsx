import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState, type ReactNode } from "react";
import { HotkeyFields } from "./HotkeyInput";
import { AddProvider, describe, editModels } from "./Models";
import { describePersona, type Persona } from "./personas";
import { LIFETIMES, type ModelView, type Provider } from "./providers";
import { useSettings, type Save, type Settings } from "./useSettings";
import logo from "../src-tauri/icons/icon.svg";

type StepProps = { settings: Settings; save: Save; setReady: (ready: boolean) => void };
type Step = (props: StepProps) => ReactNode;

/** APP-6.1 step 1: setup goes on only once the chosen model passes Test connection. */
function AiSource({ settings, save, setReady }: StepProps) {
  const [status, setStatus] = useState<string>();
  const [added, setAdded] = useState<Provider>();
  const [listed, setListed] = useState<string[] | null>(); // undefined: loading; null: listing failed
  const [model, setModel] = useState("");
  const [works, setWorks] = useState(false);
  const d = settings.models.default;
  const current = d && settings.models.providers.find((p) => p.id === d.provider);

  const test = (id: string) => {
    setReady(false);
    setWorks(false);
    setStatus("Testing the model…");
    invoke<number>("provider_test", { provider: id }).then(
      () => {
        setStatus("It works.");
        setReady(true);
        setWorks(true);
      },
      (e) => setStatus(describe(e)),
    );
  };
  // Re-running setup with a model that already works can go straight on (APP-6.3).
  useEffect(() => {
    setReady(false);
    if (current) test(current.id);
  }, []);

  const use = (p: Provider, m: string) =>
    editModels(save, (ms) => ({
      ...ms,
      providers: ms.providers.map((x) => (x.id === p.id && !x.models.includes(m) ? { ...x, models: [...x.models, m] } : x)),
      default: { provider: p.id, model: m },
    })).then(() => test(p.id), (e) => setStatus(String(e)));

  const onAdded = (p: Provider) => {
    setAdded(p);
    setStatus(undefined);
    if (p.kind === "builtin") return void use(p, p.models[0]);
    setListed(undefined);
    invoke<ModelView[]>("models_list", { provider: p.id }).then(
      (l) => {
        setListed(l.map((m) => m.id));
        setModel(l[0]?.id ?? "");
      },
      (e) => {
        setListed(null);
        setStatus(`${describe(e)} You can type a model ID instead.`);
      },
    );
  };

  // Going back removes the provider just added (and its key), so a retry doesn't leave duplicates.
  const back = (p: Provider) =>
    editModels(save, (ms) => ({ ...ms, providers: ms.providers.filter((x) => x.id !== p.id), default: ms.default?.provider === p.id ? null : ms.default })).then(() => {
      setAdded(undefined);
      setStatus(undefined);
    });

  return (
    <>
      <h2>Choose an AI source</h2>
      <p className="muted">Cursora needs a working model: one on this computer, or a cloud provider with an API key.</p>
      {current && d && (
        <p>
          Current model: <strong>{d.model}</strong> ({current.name})
        </p>
      )}
      {!added ? (
        <AddProvider save={save} onAdded={onAdded} />
      ) : (
        <>
          {added.kind !== "builtin" && (
            <form
              className="row"
              onSubmit={(e) => {
                e.preventDefault();
                if (model.trim()) use(added, model.trim());
              }}
            >
              <span>Model</span>
              {listed ? (
                <select aria-label="Model" value={model} onChange={(e) => setModel(e.target.value)}>
                  {listed.map((m) => (
                    <option key={m}>{m}</option>
                  ))}
                </select>
              ) : listed === null ? (
                <input aria-label="Model ID" placeholder="Model ID" value={model} onChange={(e) => setModel(e.target.value)} />
              ) : (
                <span className="muted">Loading models…</span>
              )}
              <button type="submit" disabled={!model.trim()}>
                Use this model
              </button>
            </form>
          )}
          <button type="button" className="link" onClick={() => back(added)}>
            Choose a different source
          </button>
        </>
      )}
      {status && (
        <p role="status" className="note">
          {status}
        </p>
      )}
      {works && current?.kind === "builtin" && <EngineLifetime p={current} save={save} />}
    </>
  );
}

/** LLM-3.5: how long the built-in model stays loaded, with "Always on" preselected. Settings → Models has the same choice. */
function EngineLifetime({ p, save }: { p: Provider; save: Save }) {
  const choose = (lifetime: (typeof LIFETIMES)[number]["value"]) =>
    editModels(save, (ms) => ({ ...ms, providers: ms.providers.map((x) => (x.id === p.id && x.engine ? { ...x, engine: { ...x.engine, lifetime } } : x)) })).catch(() => {});
  return (
    <fieldset>
      <legend>How long should the model stay loaded?</legend>
      {LIFETIMES.map((l) => (
        <label key={l.value} className="row">
          <input type="radio" name="lifetime" checked={p.engine?.lifetime === l.value} onChange={() => void choose(l.value)} />
          <span>
            <strong>{l.label}</strong> <span className="muted">{l.hint}</span>
          </span>
        </label>
      ))}
    </fieldset>
  );
}

/** APP-6.1 step 2: a persona, Cursora preselected, or personality off. */
function Personality({ settings, save }: StepProps) {
  const [personas, setPersonas] = useState<Persona[]>([]);
  useEffect(() => void invoke<Persona[]>("personas").then(setPersonas), []);
  const active = personas.find((p) => p.id === settings.activePersona)?.id ?? "preset:cursora";
  const choose = (patch: Partial<Settings>) => save(patch).catch(() => {});
  const option = (key: string, checked: boolean, onChange: () => void, name: string, hint: string) => (
    <label key={key} className="row">
      <input type="radio" name="persona" checked={checked} onChange={onChange} />
      <span>
        <strong>{name}</strong> <span className="muted">{hint}</span>
      </span>
    </label>
  );
  return (
    <>
      <h2>Choose a personality</h2>
      <p className="muted">Your cursor's name and style. You can change it or make your own in Settings → Personality, or type /persona in the bubble.</p>
      <fieldset>
        <legend>Personality</legend>
        {personas.map((p) => option(p.id, settings.personalityOn && p.id === active, () => void choose({ activePersona: p.id, personalityOn: true }), p.name, describePersona(p)))}
        {option("off", !settings.personalityOn, () => void choose({ personalityOn: false }), "Personality off", "The plain model, with no name or character.")}
      </fieldset>
    </>
  );
}

function Hotkeys({ settings, save }: StepProps) {
  return (
    <>
      <h2>Choose your hotkeys</h2>
      <p className="muted">Select text anywhere and press Ask to chat about it. You can change these later in Settings.</p>
      <HotkeyFields settings={settings} save={save} />
    </>
  );
}

function Startup({ settings, save }: StepProps) {
  return (
    <>
      <h2>Start with your computer?</h2>
      <label className="row">
        <input type="checkbox" checked={settings.startWithSystem} onChange={(e) => save({ startWithSystem: e.target.checked }).catch(() => {})} />
        Start Cursora when I log in
      </label>
    </>
  );
}

// Later specs add their steps here: Try it (03).
const STEPS: Step[] = [AiSource, Personality, Hotkeys, Startup];

export default function Setup() {
  const { settings, save, error } = useSettings();
  const [i, setI] = useState(0);
  const [ready, setReady] = useState(true);
  if (!settings) return null;

  const Current = STEPS[i];
  const last = i === STEPS.length - 1;
  const go = (to: number) => {
    setReady(true); // a step that needs something first says so when it opens
    setI(to);
  };
  async function next() {
    if (!last) return go(i + 1);
    try {
      await save({ setupDone: true });
      await getCurrentWindow().close();
    } catch {
      // The error is shown by useSettings; stay open so the user can retry.
    }
  }

  return (
    <main className="setup">
      <p className="brand">
        <img src={logo} alt="" width={24} height={24} />
        Cursora
      </p>
      <p className="muted step">
        Step {i + 1} of {STEPS.length}
      </p>
      <div className="progress" aria-hidden="true">
        <span style={{ width: `${((i + 1) / STEPS.length) * 100}%` }} />
      </div>
      {error && <p role="alert" className="error">{error}</p>}
      <Current settings={settings} save={save} setReady={setReady} />
      <footer>
        {i > 0 && (
          <button type="button" onClick={() => go(i - 1)}>
            Back
          </button>
        )}
        <button type="button" className="primary" onClick={next} disabled={!ready}>
          {last ? "Finish" : "Next"}
        </button>
      </footer>
    </main>
  );
}
