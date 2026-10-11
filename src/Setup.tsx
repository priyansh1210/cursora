import { getCurrentWindow } from "@tauri-apps/api/window";
import { useState, type ReactNode } from "react";
import { HotkeyFields } from "./HotkeyInput";
import { useSettings, type Save, type Settings } from "./useSettings";

type Step = (props: { settings: Settings; save: Save }) => ReactNode;

function Hotkeys({ settings, save }: { settings: Settings; save: Save }) {
  return (
    <>
      <h2>Choose your hotkeys</h2>
      <p className="muted">Select text anywhere and press Ask to chat about it. You can change these later in Settings.</p>
      <HotkeyFields settings={settings} save={save} />
    </>
  );
}

function Startup({ settings, save }: { settings: Settings; save: Save }) {
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

// Later specs add their steps here: AI source (04), Personality (05), Try it (03).
const STEPS: Step[] = [Hotkeys, Startup];

export default function Setup() {
  const { settings, save, error } = useSettings();
  const [i, setI] = useState(0);
  if (!settings) return null;

  const Current = STEPS[i];
  const last = i === STEPS.length - 1;
  async function next() {
    if (!last) return setI(i + 1);
    try {
      await save({ setupDone: true });
      await getCurrentWindow().close();
    } catch {
      // The error is shown by useSettings; stay open so the user can retry.
    }
  }

  return (
    <main className="setup">
      <p className="muted">
        Step {i + 1} of {STEPS.length}
      </p>
      {error && <p role="alert" className="error">{error}</p>}
      <Current settings={settings} save={save} />
      <footer>
        {i > 0 && (
          <button type="button" onClick={() => setI(i - 1)}>
            Back
          </button>
        )}
        <button type="button" onClick={next}>
          {last ? "Finish" : "Next"}
        </button>
      </footer>
    </main>
  );
}
