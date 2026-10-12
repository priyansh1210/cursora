import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

// Mirrors src-tauri/src/settings.rs.
export type Settings = {
  schemaVersion: number;
  setupDone: boolean;
  paused: boolean;
  startWithSystem: boolean;
  theme: "system" | "light" | "dark";
  hotkeys: { ask: string; snip: string; pushToTalk: string | null };
  personalityOn: boolean;
  localOnly: boolean;
};

export type Save = (patch: Partial<Settings>) => Promise<void>;

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
