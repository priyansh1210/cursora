import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { badge, errorText } from "./conversation";
import { isIndian, MODELS_FOR_LANGUAGES } from "./MyLanguage";
import { engineProvider, LIFETIMES, newProvider, PRESETS, type ChatError, type Detected, type Enabled, type EngineStatus, type Lifetime, type ModelView, type Provider, type ProviderView } from "./providers";
import { editSettings, type Save, type Settings } from "./useSettings";

type Models = Settings["models"];

export const editModels = (save: Save, f: (m: Models) => Models) => editSettings(save, "models", f);

/** A provider error in plain words, plus the provider's own message (LLM-1.4). */
export function describe(e: unknown): string {
  if (typeof e === "object" && e && "kind" in e) {
    const { kind, message } = e as ChatError;
    return kind === "unknown" ? errorText(kind, message).text : `${errorText(kind, message).text} (${message})`;
  }
  return String(e);
}

export function useEnabledModels(settings: Settings | undefined) {
  const [models, setModels] = useState<Enabled[]>([]);
  useEffect(() => void invoke<Enabled[]>("enabled_models").then(setModels), [settings]);
  return models;
}

/** The chosen models, grouped by provider; cloud models are disabled in local-only mode (LLM-5.4, 5.5). */
export function ModelSelect({ models, value, onChange, label }: { models: Enabled[]; value: string; onChange: (provider: string, model: string) => void; label: string }) {
  const groups = [...new Set(models.map((m) => m.provider))];
  return (
    <select
      aria-label={label}
      value={value}
      onChange={(e) => {
        const [provider, model] = e.target.value.split("\n");
        onChange(provider, model);
      }}
    >
      {!value && <option value="">Choose a model</option>}
      {groups.map((g) => (
        <optgroup key={g} label={models.find((m) => m.provider === g)?.providerName}>
          {models
            .filter((m) => m.provider === g)
            .map((m) => (
              <option key={m.model} value={`${m.provider}\n${m.model}`} disabled={m.disabled}>
                {m.model}
                {m.disabled ? " (local-only)" : ""}
              </option>
            ))}
        </optgroup>
      ))}
    </select>
  );
}

export const modelValue = (d: Models["default"]) => (d ? `${d.provider}\n${d.model}` : "");

/** Settings → Models. */
export default function ModelsTab({ settings, save }: { settings: Settings; save: Save }) {
  const [providers, setProviders] = useState<ProviderView[]>([]);
  const refresh = () => invoke<ProviderView[]>("providers_list").then(setProviders);
  useEffect(() => void refresh(), [settings]);
  const enabled = useEnabledModels(settings);
  const m = settings.models;
  const edit = (f: (m: Models) => Models) => editModels(save, f).catch(() => {});
  const t = m.timeouts;

  return (
    <>
      <h2>Default model</h2>
      {enabled.length ? (
        <ModelSelect label="Default model" models={enabled} value={modelValue(m.default)} onChange={(provider, model) => edit((m) => ({ ...m, default: { provider, model } }))} />
      ) : (
        <p className="muted">Add a provider below and choose its models.</p>
      )}
      {settings.localOnly && <p className="muted">Local-only mode is on, so cloud models can't be used.</p>}

      <h2>Providers</h2>
      {providers.map((p) => (
        <ProviderCard
          key={p.id}
          view={p}
          onChange={(next) => edit((m) => ({ ...m, providers: m.providers.map((x) => (x.id === next.id ? next : x)) }))}
          onRemove={() => edit((m) => ({ ...m, providers: m.providers.filter((x) => x.id !== p.id), default: m.default?.provider === p.id ? null : m.default }))}
          onKeyChange={refresh}
        />
      ))}

      <h2>Add a provider</h2>
      <AddProvider save={save} onAdded={() => void refresh()} />

      <h2>Trusted LAN hosts</h2>
      <p className="muted">Computers on your network that count as local: they can answer in local-only mode.</p>
      {m.trustedHosts.length === 0 && <p className="muted">None.</p>}
      {m.trustedHosts.map((h) => (
        <div className="row" key={h}>
          <span>{h}</span>
          <button type="button" onClick={() => edit((m) => ({ ...m, trustedHosts: m.trustedHosts.filter((x) => x !== h) }))}>
            Remove
          </button>
        </div>
      ))}

      <details>
        <summary>Advanced</summary>
        {(
          [
            ["connectSecs", "Connect timeout (s)"],
            ["firstReplyLocalSecs", "First reply, local models (s)"],
            ["firstReplyCloudSecs", "First reply, cloud models (s)"],
            ["pauseSecs", "Pause between parts of an answer (s)"],
          ] as const
        ).map(([k, label]) => (
          <label className="row" key={k}>
            <span>{label}</span>
            <input type="number" min={1} defaultValue={t[k]} onBlur={(e) => Number(e.target.value) > 0 && edit((m) => ({ ...m, timeouts: { ...m.timeouts, [k]: Number(e.target.value) } }))} />
          </label>
        ))}
      </details>
    </>
  );
}

function ProviderCard({ view, onChange, onRemove, onKeyChange }: { view: ProviderView; onChange: (p: Provider) => void; onRemove: () => void; onKeyChange: () => void }) {
  const { locality, hasKey, keyHint, ...p } = view;
  const [result, setResult] = useState<string>();
  const [listed, setListed] = useState<ModelView[]>();
  const [filter, setFilter] = useState("");
  const [typed, setTyped] = useState("");
  const [key, setKey] = useState("");
  const set = (patch: Partial<Provider>) => onChange({ ...p, ...patch });
  const choose = (id: string, on: boolean) => set({ models: on ? [...p.models, id] : p.models.filter((m) => m !== id) });

  const test = () => {
    setResult("Testing…");
    invoke<number>("provider_test", { provider: p.id }).then(
      (n) => setResult(p.kind === "builtin" ? "The engine is running." : `Connected. ${n} model${n === 1 ? "" : "s"} found.`),
      (e) => setResult(describe(e)),
    );
  };
  const load = () => {
    setResult("Loading models…");
    invoke<ModelView[]>("models_list", { provider: p.id }).then(
      (list) => {
        setListed(list);
        setResult(undefined);
      },
      (e) => setResult(`${describe(e)} You can type a model ID instead.`),
    );
  };
  const keyAction = (cmd: string, args: Record<string, string>) => invoke(cmd, args).then(() => (setKey(""), onKeyChange()), (e) => setResult(String(e)));

  return (
    <section className="card" aria-label={p.name}>
      <div className="row">
        <strong>{p.name}</strong>
        <span className="badge">{badge(locality)}</span>
        <span className="grow" />
        <button type="button" onClick={test}>
          Test connection
        </button>
        <button type="button" className="danger" onClick={onRemove}>
          Remove
        </button>
      </div>
      {p.kind === "builtin" ? (
        <p className="muted note">{p.engine?.gguf}</p>
      ) : (
        (["name", "baseUrl"] as const).map((k) => (
          <label className="row" key={k}>
            <span>{k === "name" ? "Name" : "Base URL"}</span>
            <input
              defaultValue={p[k]}
              onBlur={(e) => {
                const v = e.target.value.trim();
                if (v && v !== p[k]) set({ [k]: v });
              }}
            />
          </label>
        ))
      )}
      {result && (
        <p className="note" role="status">
          {result}
        </p>
      )}

      {p.kind !== "builtin" && (
        <form
          className="row"
          onSubmit={(e) => {
            e.preventDefault();
            if (key.trim()) keyAction("key_set", { provider: p.id, key });
          }}
        >
          <span>API key</span>
          {/* The saved key's last characters sit in the empty box, so the row fits on one line. */}
          <input
            type="password"
            aria-label={hasKey ? `API key for ${p.name}, saved, ending in ${keyHint}` : `API key for ${p.name}`}
            placeholder={hasKey ? `Saved  •••• ${keyHint}` : "Paste the key"}
            value={key}
            onChange={(e) => setKey(e.target.value)}
          />
          <button type="submit" disabled={!key.trim()}>
            {hasKey ? "Replace" : "Save key"}
          </button>
          {hasKey && (
            <button type="button" className="danger" onClick={() => keyAction("key_delete", { provider: p.id })}>
              Remove key
            </button>
          )}
        </form>
      )}

      {p.kind === "builtin" ? (
        <Engine p={p} set={set} />
      ) : (
        <>
          <div className="row">
            <span>Models</span>
            <button type="button" onClick={load}>
              {listed ? "Reload" : "Choose models"}
            </button>
          </div>
          {listed && (
            <>
              {listed.length > 10 && <input placeholder="Filter" aria-label="Filter models" value={filter} onChange={(e) => setFilter(e.target.value)} />}
              <ul className="models">
                {listed
                  .filter((m) => m.id.toLowerCase().includes(filter.toLowerCase()))
                  .map((m) => (
                    <li key={m.id}>
                      <label>
                        <input type="checkbox" checked={p.models.includes(m.id)} onChange={(e) => choose(m.id, e.target.checked)} /> {m.id}
                        {m.vision && <span className="muted"> · images</span>}
                      </label>
                    </li>
                  ))}
              </ul>
            </>
          )}
          <form
            className="row"
            onSubmit={(e) => {
              e.preventDefault();
              if (typed.trim() && !p.models.includes(typed.trim())) choose(typed.trim(), true);
              setTyped("");
            }}
          >
            <input placeholder="Model ID" aria-label="Type a model ID" value={typed} onChange={(e) => setTyped(e.target.value)} />
            <button type="submit" disabled={!typed.trim()}>
              Add model
            </button>
          </form>
        </>
      )}

      {p.models.length > 0 && (
        <ul className="models">
          {p.models.map((id) => (
            <li key={id} className="row">
              <span>{id}</span>
              <select
                aria-label={`Can ${id} read images?`}
                value={id in p.visionOverride ? String(p.visionOverride[id]) : "auto"}
                onChange={(e) => {
                  const visionOverride = { ...p.visionOverride };
                  if (e.target.value === "auto") delete visionOverride[id];
                  else visionOverride[id] = e.target.value === "true";
                  set({ visionOverride });
                }}
              >
                <option value="auto">Images: auto</option>
                <option value="true">Images: yes</option>
                <option value="false">Images: no</option>
              </select>
              {p.kind !== "builtin" && (
                <button type="button" aria-label={`Stop using ${id}`} onClick={() => choose(id, false)}>
                  ×
                </button>
              )}
            </li>
          ))}
        </ul>
      )}

      <label className="row">
        <span>Context size</span>
        <input
          type="number"
          min={512}
          placeholder={p.kind === "builtin" ? "8192" : "as the provider reports"}
          defaultValue={p.contextSize ?? ""}
          onBlur={(e) => set({ contextSize: Number(e.target.value) || null })}
        />
      </label>
    </section>
  );
}

const ENGINE_STATES: Record<EngineStatus["state"], string> = { stopped: "not running", starting: "starting…", running: "running", failed: "couldn't start" };

/** The built-in engine's files, lifetime and state (LLM-3). */
function Engine({ p, set }: { p: Provider; set: (patch: Partial<Provider>) => void }) {
  const [status, setStatus] = useState<EngineStatus>({ state: "stopped" });
  useEffect(() => {
    invoke<EngineStatus>("engine_status").then(setStatus);
    const off = listen<EngineStatus>("engine-status", (e) => setStatus(e.payload));
    return () => void off.then((f) => f());
  }, []);
  const e = p.engine!;
  const setEngine = (patch: Partial<NonNullable<Provider["engine"]>>) => set({ engine: { ...e, ...patch } });
  return (
    <>
      <div className="note engine" role="status">
        <span className={`dot ${status.state}`} aria-hidden="true" />
        Engine: {ENGINE_STATES[status.state]}{" "}
        {(status.state === "running" || status.state === "starting") && (
          <button type="button" onClick={() => void invoke("engine_stop")}>
            Stop the model
          </button>
        )}
        {status.state === "failed" && (
          <div className="error">
            <pre className="detail">{status.reason}</pre>
          </div>
        )}
      </div>
      <div className="row">
        <span>Vision projector (mmproj)</span>
        <span className="muted">{e.mmproj ?? "none"}</span>
        <button type="button" onClick={() => invoke<string | null>("pick_model_file").then((f) => f && setEngine({ mmproj: f }))}>
          Choose…
        </button>
        {e.mmproj && (
          <button type="button" onClick={() => setEngine({ mmproj: null })}>
            Remove
          </button>
        )}
      </div>
      <label className="row">
        <span>Keep the model loaded</span>
        <select value={e.lifetime} onChange={(ev) => setEngine({ lifetime: ev.target.value as Lifetime })}>
          {LIFETIMES.map((l) => (
            <option key={l.value} value={l.value}>
              {l.label}
            </option>
          ))}
        </select>
      </label>
      <p className="muted note">{LIFETIMES.find((l) => l.value === e.lifetime)?.hint}</p>
      {e.lifetime === "idle" && (
        <label className="row">
          <span>Unload after (minutes)</span>
          <input type="number" min={1} defaultValue={e.idleMinutes} onBlur={(ev) => Number(ev.target.value) > 0 && setEngine({ idleMinutes: Number(ev.target.value) })} />
        </label>
      )}
    </>
  );
}

/** Adding a provider: LM Studio if it is running, the built-in engine, a preset, or any OpenAI-compatible server. Setup uses it too. */
export function AddProvider({ save, onAdded }: { save: Save; onAdded: (p: Provider) => void }) {
  const [source, setSource] = useState("");
  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const [key, setKey] = useState("");
  const [error, setError] = useState<string>();
  const [detected, setDetected] = useState<Detected[]>([]);
  const [trust, setTrust] = useState<{ host: string; provider: Provider }>();
  useEffect(() => {
    invoke<Detected[]>("runtimes_probe").then(setDetected);
  }, []);
  const preset = PRESETS.find((p) => p.id === source);

  async function add(p: Provider, keyText = "") {
    setError(undefined);
    try {
      await editModels(save, (m) => ({ ...m, providers: [...m.providers, p] }));
      if (keyText.trim()) await invoke("key_set", { provider: p.id, key: keyText });
      const host = p.baseUrl ? await invoke<string | null>("lan_host", { url: p.baseUrl }) : null;
      const trusted = (await invoke<Settings>("get_settings")).models.trustedHosts;
      setSource("");
      setKey("");
      if (host && !trusted.includes(host)) setTrust({ host, provider: p }); // LLM-8.3
      else onAdded(p);
    } catch (e) {
      setError(String(e));
    }
  }

  if (trust) {
    const done = () => (setTrust(undefined), onAdded(trust.provider));
    return (
      <div role="alertdialog" aria-label="Trust this host?" className="note">
        <p>
          Trust <strong>{trust.host}</strong>? Requests to it will count as local: they are allowed in local-only mode and treated like this computer.
        </p>
        <button type="button" onClick={() => editModels(save, (m) => ({ ...m, trustedHosts: [...m.trustedHosts, trust.host] })).then(done, (e) => setError(String(e)))}>
          Trust it
        </button>{" "}
        <button type="button" onClick={done}>
          Don't trust
        </button>
      </div>
    );
  }

  return (
    <>
      {error && <p role="alert" className="error">{error}</p>}
      {detected.map((d) => (
        <div className="row" key={d.baseUrl}>
          <span>
            Found on this computer: <strong>{d.name}</strong> with {d.models.length} model{d.models.length === 1 ? "" : "s"}
          </span>
          <button type="button" onClick={() => add(newProvider(PRESETS.find((p) => p.id === d.preset)!, d.name, d.baseUrl))}>
            Use it
          </button>
        </div>
      ))}
      <label className="row">
        <span>Source</span>
        <select
          value={source}
          onChange={(e) => {
            const p = PRESETS.find((x) => x.id === e.target.value);
            setSource(e.target.value);
            setName(p?.name ?? "");
            setUrl(p?.baseUrl ?? "");
          }}
        >
          <option value="">Choose…</option>
          <option value="download">Download a free model</option>
          <option value="builtin">Built-in engine (a .gguf model file)</option>
          {PRESETS.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </label>
      {source === "download" && <DownloadModel onReady={(path) => void add(engineProvider(path, null))} />}
      {source === "builtin" && (
        <button type="button" onClick={() => invoke<string | null>("pick_model_file").then((f) => void (f && add(engineProvider(f, null))))}>
          Choose a .gguf file
        </button>
      )}
      {preset && (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            add(newProvider(preset, name, url), key);
          }}
        >
          <label className="row">
            <span>Name</span>
            <input value={name} onChange={(e) => setName(e.target.value)} />
          </label>
          <label className="row">
            <span>Base URL</span>
            <input value={url} placeholder="http://192.168.1.20:8080/v1" onChange={(e) => setUrl(e.target.value)} required />
          </label>
          <label className="row">
            <span>API key</span>
            <input type="password" value={key} placeholder={preset.needsKey ? "" : "optional"} onChange={(e) => setKey(e.target.value)} required={preset.needsKey} />
          </label>
          <button type="submit">Add</button>
        </form>
      )}
    </>
  );
}

type Choice = { id: string; label: string; note: string; size: number };
type Progress = { done: number; total: number };
const gb = (bytes: number) => (bytes / 1e9).toFixed(1);

/** One-click model download (MDL-1): three sizes, one recommended, the folder, then progress and Cancel. */
function DownloadModel({ onReady }: { onReady: (path: string) => void }) {
  const [choices, setChoices] = useState<{ models: Choice[]; recommended: string }>();
  const [id, setId] = useState("");
  const [folder, setFolder] = useState("");
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<Progress>();
  const [note, setNote] = useState<string>();
  const [error, setError] = useState<string>();
  const [indian, setIndian] = useState(false); // LNG-4.2
  useEffect(() => {
    void invoke<Settings>("get_settings").then((s) => setIndian(isIndian(s.myLanguage)));
    void invoke<{ models: Choice[]; recommended: string; folder: string }>("model_choices").then((c) => {
      setChoices(c);
      setId(c.recommended);
      setFolder(c.folder);
    });
    const off = listen<Progress>("model-progress", (e) => setProgress(e.payload));
    return () => void off.then((f) => f());
  }, []);
  if (!choices) return null;
  const download = () => {
    setBusy(true);
    setProgress(undefined);
    setNote(undefined);
    setError(undefined);
    invoke<string | null>("model_download", { id, folder })
      .then(
        (path) => (path ? onReady(path) : setNote("Paused. Download again to continue where it stopped.")),
        (e) => setError(String(e)),
      )
      .finally(() => {
        setBusy(false);
        setProgress(undefined);
      });
  };
  // Only this page's own download: a late progress event must not bring the bar back after it ends.
  const running = busy;
  const percent = progress ? Math.floor((progress.done / progress.total) * 100) : 0;

  return (
    <div className="download">
      <div role="radiogroup" aria-label="Model size">
        {choices.models.map((m) => (
          <label className="row" key={m.id}>
            <input type="radio" name="model-size" checked={id === m.id} disabled={running} onChange={() => setId(m.id)} />
            <span>
              <strong>{m.label}</strong> · {gb(m.size)} GB <span className="muted">· {m.note}</span>
            </span>
            {m.id === choices.recommended && <span className="badge">Recommended</span>}
          </label>
        ))}
      </div>
      <div className="row">
        <span className="muted folder" title={folder}>
          Saves to {folder}
        </span>
        <button type="button" disabled={running} onClick={() => invoke<string | null>("model_folder").then((f) => f && setFolder(f))}>
          Change…
        </button>
      </div>
      {running ? (
        <div className="row">
          <div className="progress" role="progressbar" aria-label="Download progress" aria-valuemin={0} aria-valuemax={100} aria-valuenow={percent}>
            <span style={{ width: `${percent}%` }} />
          </div>
          <span>{!progress ? "Starting…" : progress.done < progress.total ? `${gb(progress.done)} of ${gb(progress.total)} GB` : "Checking the file…"}</span>
          <button type="button" onClick={() => void invoke("model_download_cancel")}>
            Cancel
          </button>
        </div>
      ) : (
        <button type="button" className="primary" onClick={download}>
          Download
        </button>
      )}
      <p className="muted note">Free Qwen models (Apache 2.0 license) from Hugging Face. They run on this computer, so your questions stay on it.</p>
      {indian && <p className="muted note">{MODELS_FOR_LANGUAGES}</p>}
      {note && (
        <p className="note" role="status">
          {note}
        </p>
      )}
      {error && <p role="alert" className="error">{error}</p>}
    </div>
  );
}
