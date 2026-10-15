import { useState, type KeyboardEvent } from "react";
import type { Save, Settings } from "./useSettings";

const MODIFIERS = ["Control", "Shift", "Alt", "Meta"];

type Props = { label: string; value: string | null; onChange: (v: string | null) => void };

/** Click, then press a key combination; it is stored like "Ctrl+Shift+G". Validation comes with spec 02 (SEL-1.2). */
export function HotkeyInput({ label, value, onChange }: Props) {
  const [recording, setRecording] = useState(false);

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Tab") return setRecording(false); // never trap keyboard users
    e.preventDefault();
    if (e.key === "Escape") return setRecording(false);
    if (MODIFIERS.includes(e.key)) return;
    const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
    onChange([...mods, e.code.replace(/^(Key|Digit)/, "")].join("+"));
    setRecording(false);
  }

  return (
    <div className="row">
      <span>{label}</span>
      <button type="button" className="hotkey" aria-label={`${label} hotkey`} onClick={() => setRecording(true)} onBlur={() => setRecording(false)} onKeyDown={recording ? onKeyDown : undefined}>
        {recording ? "Press keys…" : value ?? "Not set"}
      </button>
    </div>
  );
}

export function HotkeyFields({ settings, save }: { settings: Settings; save: Save }) {
  const set = (patch: Partial<Settings["hotkeys"]>) => save({ hotkeys: { ...settings.hotkeys, ...patch } }).catch(() => {});
  return (
    <>
      <HotkeyInput label="Ask" value={settings.hotkeys.ask} onChange={(ask) => ask && set({ ask })} />
      {settings.hotkeys.ask === "Ctrl+G" && (
        <p className="muted">Ctrl+G replaces "Go to line" in VS Code and Notepad, and "Find next" in browsers.</p>
      )}
      <HotkeyInput label="Snip" value={settings.hotkeys.snip} onChange={(snip) => snip && set({ snip })} />
      {/* Push-to-talk appears when spec 09 is built: until then it would do nothing (issue 37). */}
    </>
  );
}
