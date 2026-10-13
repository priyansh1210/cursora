import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { Persona } from "./personas";
import type { Provider } from "./providers";

// Mirrors src-tauri/src/settings.rs.
export type Settings = {
  schemaVersion: number;
  setupDone: boolean;
  paused: boolean;
  startWithSystem: boolean;
  theme: "system" | "light" | "dark";
  hotkeys: { ask: string; snip: string; pushToTalk: string | null };
  personalityOn: boolean;
  activePersona: string;
  personas: Persona[];
  localOnly: boolean;
  capture: {
    unsafeApps: string[];
    copyShortcuts: Record<string, CopyKey>;
    shareWindowTitle: boolean;
    maxChars: number;
    copyTimeoutMs: number;
    fileMaxBytes: number;
    filesTotalBytes: number;
    blocklist: Rule[];
  };
  privacy: { redact: boolean; redactLocal: boolean; detectorsOff: string[]; patterns: { label: string; regex: string }[]; confirmCloud: boolean };
  history: { save: boolean; retentionDays: number | null };
  models: {
    providers: Provider[];
    default: { provider: string; model: string } | null;
    trustedHosts: string[];
    timeouts: { connectSecs: number; firstReplyLocalSecs: number; firstReplyCloudSecs: number; pauseSecs: number };
  };
};

export type CopyKey = "ctrlC" | "ctrlInsert" | "ctrlShiftC" | "never";
/** A blocklist rule (PRI-2.2). */
export type Rule = { process: string } | { title: string };

export type Save = (patch: Partial<Settings>) => Promise<void>;

/** Changes one section starting from the latest saved settings, so changes made one after another don't undo each other. */
export async function editSettings<K extends keyof Settings>(save: Save, key: K, f: (v: Settings[K]) => Settings[K]) {
  const s = await invoke<Settings>("get_settings");
  await save({ [key]: f(s[key]) } as Partial<Settings>);
}

export function useSettings() {
  const [settings, setSettings] = useState<Settings>();
  const [error, setError] = useState<string>();

  useEffect(() => {
    invoke<Settings>("get_settings").then(setSettings);
    const unlisten = listen<Settings>("settings-changed", (e) => setSettings(e.payload));
    return () => void unlisten.then((f) => f());
  }, []);

  useEffect(() => {
    if (settings) document.documentElement.dataset.theme = settings.theme;
  }, [settings?.theme]);

  const save: Save = (patch) =>
    invoke<void>("set_settings", { settings: { ...settings, ...patch } }).then(
      () => setError(undefined),
      (e) => {
        setError(String(e));
        throw e;
      },
    );

  return { settings, save, error };
}
