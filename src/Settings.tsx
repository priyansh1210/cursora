import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { HotkeyFields } from "./HotkeyInput";
import { useSettings, type CopyKey, type Save, type Settings as S } from "./useSettings";

const TABS = ["General", "Models", "Personality", "Skills", "Voice", "Privacy", "History", "About"] as const;

export default function Settings() {
  const { settings, save, error } = useSettings();
  const [tab, setTab] = useState<(typeof TABS)[number]>("General");
  if (!settings) return null;

  return (
    <div className="settings">
      <nav role="tablist" aria-label="Settings">
        {TABS.map((t) => (
          <button key={t} role="tab" aria-selected={t === tab} onClick={() => setTab(t)}>
            {t}
          </button>
        ))}
      </nav>
      <section role="tabpanel" aria-label={tab}>
        {error && <p role="alert" className="error">{error}</p>}
        {tab === "General" ? <General settings={settings} save={save} /> : tab === "About" ? <About /> : <p className="muted">Coming soon.</p>}
      </section>
    </div>
  );
}

function General({ settings, save }: { settings: S; save: Save }) {
  const set = (patch: Partial<S>) => save(patch).catch(() => {});
  return (
    <>
      <h2>Hotkeys</h2>
      <HotkeyFields settings={settings} save={save} />
      <h2>General</h2>
      <label className="row">
        <input type="checkbox" checked={settings.startWithSystem} onChange={(e) => set({ startWithSystem: e.target.checked })} />
        Start with system
      </label>
      <label className="row">
        <input type="checkbox" checked={settings.paused} onChange={(e) => set({ paused: e.target.checked })} />
        Pause Cursora
      </label>
      <label className="row">
        Theme
        <select value={settings.theme} onChange={(e) => set({ theme: e.target.value as S["theme"] })}>
          <option value="system">System</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select>
      </label>
      <button type="button" onClick={() => invoke("open_setup")}>
        Re-run setup
      </button>
      <Advanced settings={settings} save={save} />
    </>
  );
}

const COPY_KEYS: Record<CopyKey, string> = { ctrlC: "Ctrl+C", ctrlInsert: "Ctrl+Insert", ctrlShiftC: "Ctrl+Shift+C", never: "Never copy" };

function Advanced({ settings, save }: { settings: S; save: Save }) {
  const capture = settings.capture;
  const set = (patch: Partial<S["capture"]>) => save({ capture: { ...capture, ...patch } }).catch(() => {});
  const setShortcut = (exe: string, key: CopyKey | null) => {
    const copyShortcuts = { ...capture.copyShortcuts };
    if (key) copyShortcuts[exe] = key;
    else delete copyShortcuts[exe];
    set({ copyShortcuts });
  };
  const [newExe, setNewExe] = useState("");

  return (
    <details>
      <summary>Advanced</summary>
      <label className="row">
        <input type="checkbox" checked={capture.shareWindowTitle} onChange={(e) => set({ shareWindowTitle: e.target.checked })} />
        Share the window title with the AI
      </label>
      <label>
        Apps where Ctrl+C may stop a running process (one per line). Cursora uses Ctrl+Insert there.
        <textarea
          rows={5}
          defaultValue={capture.unsafeApps.join("\n")}
          onBlur={(e) => set({ unsafeApps: e.target.value.split("\n").map((s) => s.trim()).filter(Boolean) })}
        />
      </label>
      <p>Copy shortcut per app</p>
      {Object.entries(capture.copyShortcuts).map(([exe, key]) => (
        <div className="row" key={exe}>
          <span>{exe}</span>
          <select aria-label={`Copy shortcut for ${exe}`} value={key} onChange={(e) => setShortcut(exe, e.target.value as CopyKey)}>
            {Object.entries(COPY_KEYS).map(([k, label]) => (
              <option key={k} value={k}>
                {label}
              </option>
            ))}
          </select>
          <button type="button" aria-label={`Remove ${exe}`} onClick={() => setShortcut(exe, null)}>
            ×
          </button>
        </div>
      ))}
      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          if (newExe.trim()) setShortcut(newExe.trim(), "ctrlInsert");
          setNewExe("");
        }}
      >
        <input placeholder="app.exe" aria-label="App to add" value={newExe} onChange={(e) => setNewExe(e.target.value)} />
        <button type="submit">Add</button>
      </form>
    </details>
  );
}

type AppInfo = { version: string; os: string; session: string; configDir: string; logDir: string };

function About() {
  const [info, setInfo] = useState<AppInfo>();
  useEffect(() => void invoke<AppInfo>("app_info").then(setInfo), []);
  if (!info) return null;
  const folder = (kind: string) => (
    <button type="button" onClick={() => invoke("open_folder", { kind })}>
      Open folder
    </button>
  );
  return (
    <dl>
      <dt>Version</dt>
      <dd>{info.version}</dd>
      <dt>System</dt>
      <dd>
        {info.os} ({info.session})
      </dd>
      <dt>Settings folder</dt>
      <dd>
        <code>{info.configDir}</code> {folder("config")}
      </dd>
      <dt>Logs folder</dt>
      <dd>
        <code>{info.logDir}</code> {folder("log")}
      </dd>
    </dl>
  );
}
