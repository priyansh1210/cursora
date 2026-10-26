import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import History from "./History";
import { HotkeyFields } from "./HotkeyInput";
import ModelsTab from "./Models";
import PersonalityTab from "./Personality";
import SkillsTab from "./SkillsTab";
import Privacy from "./Privacy";
import { useAppInfo, useSettings, type CopyKey, type Save, type Settings as S } from "./useSettings";
import logo from "../src-tauri/icons/icon.svg";
import { Icon, PATHS } from "./icons";
import { MyLanguageField } from "./MyLanguage";

/** LNG-4.1, shown while My language isn't English. */
const MODELS_FOR_LANGUAGES = "The Small download can't write Indian languages: use Medium, or a cloud model such as Gemini.";

// Voice joins between Skills and Privacy once spec 09 is built (APP-5.1).
const TABS = ["General", "Models", "Personality", "Skills", "Privacy", "History", "About"] as const;
type Tab = (typeof TABS)[number];
const isTab = (t: string): t is Tab => (TABS as readonly string[]).includes(t);

export default function Settings() {
  const { settings, save, error } = useSettings();
  // A bubble error can open a given tab (BUB-4.4): by URL fragment, or by event when Settings is already open.
  const [tab, setTab] = useState<Tab>(() => {
    const hash = decodeURIComponent(location.hash.slice(1));
    return isTab(hash) ? hash : "General";
  });
  useEffect(() => {
    const off = listen<string>("settings-tab", (e) => isTab(e.payload) && setTab(e.payload));
    return () => void off.then((f) => f());
  }, []);
  if (!settings) return null;

  return (
    <div className="settings">
      <nav>
        <p className="brand">
          <img src={logo} alt="" width={24} height={24} />
          Cursora
        </p>
        <div role="tablist" aria-label="Settings">
          {TABS.map((t) => (
            <button key={t} role="tab" aria-selected={t === tab} onClick={() => setTab(t)}>
              <Icon d={PATHS[t.toLowerCase() as Lowercase<Tab>]} />
              {t}
            </button>
          ))}
        </div>
      </nav>
      <section role="tabpanel" aria-label={tab}>
        {error && <p role="alert" className="error">{error}</p>}
        {tab === "General" ? (
          <General settings={settings} save={save} />
        ) : tab === "Models" ? (
          <ModelsTab settings={settings} save={save} />
        ) : tab === "Personality" ? (
          <PersonalityTab settings={settings} save={save} />
        ) : tab === "Skills" ? (
          <SkillsTab settings={settings} save={save} />
        ) : tab === "Privacy" ? (
          <Privacy settings={settings} save={save} />
        ) : tab === "History" ? (
          <History settings={settings} save={save} />
        ) : (
          <About />
        )}
      </section>
    </div>
  );
}

function General({ settings, save }: { settings: S; save: Save }) {
  const set = (patch: Partial<S>) => save(patch).catch(() => {});
  const info = useAppInfo();
  return (
    <>
      <h2>Hotkeys</h2>
      {info?.session === "wayland" ? <WaylandShortcut launcher={info.launcher} /> : info && <HotkeyFields settings={settings} save={save} />}
      <h2>Behavior</h2>
      <label className="row">
        <input type="checkbox" checked={settings.paused} onChange={(e) => set({ paused: e.target.checked })} />
        Pause Cursora
      </label>
      <label className="row">
        <input type="checkbox" checked={settings.startWithSystem} onChange={(e) => set({ startWithSystem: e.target.checked })} />
        Start with system
      </label>
      <label className="row">
        <input type="checkbox" checked={settings.updates.check} onChange={(e) => set({ updates: { ...settings.updates, check: e.target.checked } })} />
        Check for updates automatically
      </label>
      {info?.canRunAsAdmin && (
        <>
          <label className="row">
            <input
              type="checkbox"
              checked={settings.runAsAdmin}
              disabled={!info.inProgramFiles && !settings.runAsAdmin}
              aria-describedby="admin-note"
              onChange={(e) => set({ runAsAdmin: e.target.checked })}
            />
            Run as administrator
          </label>
          <p className="muted" id="admin-note">
            {!info.inProgramFiles
              ? "Only when Cursora is installed for all users, in Program Files. Anywhere else, other programs could change its files and get admin rights."
              : info.runningAsAdmin && !settings.runAsAdmin
              ? "Cursora runs as a normal app from its next start."
              : "Lets Cursora read apps running as administrator. Windows asks once; after that, Cursora and its built-in engine run with admin rights."}
          </p>
        </>
      )}
      <h2>Language</h2>
      <MyLanguageField settings={settings} save={save} />
      <label className="row">
        <input type="checkbox" checked={settings.answerInMyLanguage} onChange={(e) => set({ answerInMyLanguage: e.target.checked })} />
        Always answer in my language
      </label>
      <p className="muted">
        {settings.answerInMyLanguage ? "Every answer comes in your language, unless the persona has its own." : "Answers come in the language you write in. Translate uses your language."}
        {/* LNG-4.1: from the trial in tasks.md, task 5. */}
        {settings.myLanguage.toLowerCase() !== "english" && ` ${MODELS_FOR_LANGUAGES}`}
      </p>
      <h2>Appearance</h2>
      <label className="row">
        <span>Theme</span>
        <select value={settings.theme} onChange={(e) => set({ theme: e.target.value as S["theme"] })}>
          <option value="system">{info?.os === "linux" ? "Same as the system" : "Same as Windows"}</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select>
      </label>
      <h2>Setup</h2>
      <div className="row">
        <span className="muted">Choose the AI source, personality and hotkeys again.</span>
        <span className="grow" />
        <button type="button" onClick={() => invoke("open_setup")}>
          Re-run setup
        </button>
      </div>
      <Advanced settings={settings} save={save} windows={info?.os === "windows"} />
    </>
  );
}

/** Wayland doesn't let apps listen for hotkeys, so the user binds `cursora --ask` and `--snip` in the desktop's settings (LNX-2.2, VIS-4.2). */
function WaylandShortcut({ launcher }: { launcher: string }) {
  const app = /^[\w/.+-]+$/.test(launcher) ? launcher : `'${launcher.replaceAll("'", "'\\''")}'`;
  return (
    <>
      <p>Wayland doesn't let apps listen for hotkeys. Add shortcuts in your desktop's settings that run:</p>
      <CommandRow label="Ask" command={`${app} --ask`} />
      <CommandRow label="Snip" command={`${app} --snip`} />
      <ul>
        <li>GNOME: Settings → Keyboard → View and Customize Shortcuts → Custom Shortcuts → +</li>
        <li>KDE: System Settings → Keyboard → Shortcuts → Add New → Command or Script</li>
      </ul>
      <p>
        Selections are read with <code>wl-paste</code>, from the wl-clipboard package. To snip, copy part of the screen with your desktop's
        screenshot tool first (on GNOME, the Print key), then press the Snip shortcut.
      </p>
    </>
  );
}

function CommandRow({ label, command }: { label: string; command: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="row">
      <span>{label}</span>
      <code>{command}</code>
      <button type="button" aria-label={`Copy the ${label} command`} onClick={() => void navigator.clipboard.writeText(command).then(() => setCopied(true))}>
        {copied ? "Copied" : "Copy"}
      </button>
    </div>
  );
}

const COPY_KEYS: Record<CopyKey, string> = { ctrlC: "Ctrl+C", ctrlInsert: "Ctrl+Insert", ctrlShiftC: "Ctrl+Shift+C", never: "Never copy" };

/** `windows`: the Ctrl+C-unsafe list applies only there; Linux sends keys to file managers only (LNX-3.3). */
function Advanced({ settings, save, windows }: { settings: S; save: Save; windows: boolean }) {
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
      {windows && (
        <label>
          Apps where Ctrl+C may stop a running process (one per line). Cursora uses Ctrl+Insert there.
          <textarea
            rows={5}
            defaultValue={capture.unsafeApps.join("\n")}
            onBlur={(e) => set({ unsafeApps: e.target.value.split("\n").map((s) => s.trim()).filter(Boolean) })}
          />
        </label>
      )}
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
        <input placeholder={windows ? "app.exe" : "process name"} aria-label="App to add" value={newExe} onChange={(e) => setNewExe(e.target.value)} />
        <button type="submit">Add</button>
      </form>
    </details>
  );
}

const SESSIONS = { windows: "", x11: " (X11)", wayland: " (Wayland)", unknown: " (unknown session)" };

/** Mirrors `update::Status` in src-tauri/src/update.rs. */
type UpdateStatus =
  | { state: "unknown" | "upToDate" | "waiting" }
  | { state: "available"; version: string; notes: string; installable: boolean }
  | { state: "failed"; reason: string }
  | { state: "downloading"; done: number; total: number | null };

/** The version, Check for updates and its result; an available update's notes, Update now and Skip (UPD-1.3, 2.2, 2.5). */
function Updates({ version }: { version: string }) {
  const [status, setStatus] = useState<UpdateStatus>({ state: "unknown" });
  const [checking, setChecking] = useState(false);
  useEffect(() => {
    invoke<UpdateStatus>("update_status").then(setStatus);
    const off = listen<UpdateStatus>("update-status", (e) => setStatus(e.payload));
    return () => void off.then((f) => f());
  }, []);
  const busy = checking || status.state === "downloading" || status.state === "waiting";
  const check = () => {
    setChecking(true);
    invoke<UpdateStatus>("update_check")
      .then(setStatus)
      .finally(() => setChecking(false));
  };
  const failed = (e: unknown) => setStatus({ state: "failed", reason: String(e) });
  const mb = (n: number) => (n / 1048576).toFixed(1);
  return (
    <>
      <dd className="folder">
        <span>{version}</span>
        <button type="button" disabled={busy} onClick={check}>
          {checking ? "Checking…" : "Check for updates"}
        </button>
      </dd>
      <dd role="status" className={status.state === "failed" ? "error" : "muted"}>
        {status.state === "upToDate" && "Cursora is up to date."}
        {status.state === "failed" && status.reason}
        {status.state === "waiting" && "Cursora will update when the answer it's writing is finished."}
        {status.state === "downloading" && `Downloading the update: ${mb(status.done)}${status.total ? ` of ${mb(status.total)}` : ""} MB…`}
      </dd>
      {status.state === "available" && (
        <dd className="card">
          <p>
            <strong>Version {status.version} is available.</strong>
          </p>
          {status.notes && <p className="notes">{status.notes}</p>}
          <div className="row">
            <button type="button" className="primary" onClick={() => invoke("update_install").catch(failed)}>
              {status.installable ? "Update now" : "Open the download page"}
            </button>
            <button type="button" onClick={() => invoke("update_skip", { version: status.version }).catch(failed)}>
              Skip this version
            </button>
          </div>
        </dd>
      )}
    </>
  );
}

function About() {
  const info = useAppInfo();
  if (!info) return null;
  const folder = (kind: string) => (
    <button type="button" onClick={() => invoke("open_folder", { kind })}>
      Open folder
    </button>
  );
  return (
    <>
      <p className="brand about">
        <img src={logo} alt="" width={64} height={64} />
        Cursora
      </p>
      <dl>
        <dt>Version</dt>
        <Updates version={info.version} />
        <dt>System</dt>
        <dd>
          {info.os === "windows" ? "Windows" : "Linux"}
          {SESSIONS[info.session]}
        </dd>
        {info.os === "linux" && (
          <>
            <dt>Tray icon</dt>
            <dd>
              No Cursora icon in the top bar? GNOME needs the AppIndicator extension (Ubuntu includes it). Everything works without it: your hotkey or
              shortcut asks, and <code>cursora --settings</code> opens this window.
            </dd>
          </>
        )}
        <dt>Settings folder</dt>
        <dd className="folder">
          <code>{info.configDir}</code>
          {folder("config")}
        </dd>
        <dt>Logs folder</dt>
        <dd className="folder">
          <code>{info.logDir}</code>
          {folder("log")}
        </dd>
      </dl>
    </>
  );
}
