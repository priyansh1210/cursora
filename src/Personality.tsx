import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { BLANK, clean, describePersona, GENDERS, isBuiltIn, TONES, type Persona } from "./personas";
import { editSettings, type Save, type Settings } from "./useSettings";

/** Settings → Personality (PER-1.2, PER-2.2, PER-4.4). */
export default function PersonalityTab({ settings, save }: { settings: Settings; save: Save }) {
  const [personas, setPersonas] = useState<Persona[]>([]);
  const [preview, setPreview] = useState("");
  const [editing, setEditing] = useState<Persona>();
  const [confirming, setConfirming] = useState<string>();
  const [note, setNote] = useState<string>();
  useEffect(() => {
    void invoke<Persona[]>("personas").then(setPersonas);
    void invoke<string>("prompt_preview").then(setPreview);
  }, [settings]);

  const active = personas.find((p) => p.id === settings.activePersona)?.id ?? "preset:cursora";
  const fail = (e: unknown) => setNote(String(e));
  const add = (p: Persona) =>
    invoke<Persona>("persona_add", { persona: p }).then((added) => {
      setNote(undefined);
      setEditing(added);
    }, fail);
  const remove = (p: Persona) => editSettings(save, "personas", (ps) => ps.filter((x) => x.id !== p.id)).then(() => setConfirming(undefined), fail);
  // The editor's buttons go with it, so focus returns to the card's Edit button (keyboard users keep their place).
  const closeEditor = (p: Persona) => {
    setEditing(undefined);
    requestAnimationFrame(() => document.getElementById(`edit-${p.id}`)?.focus());
  };

  return (
    <>
      <h2>Personality</h2>
      <label className="row">
        <input type="checkbox" checked={settings.personalityOn} onChange={(e) => save({ personalityOn: e.target.checked }).catch(() => {})} />
        <span>
          Personality <span className="muted">Off: the plain model, with no name or character.</span>
        </span>
      </label>

      <h2>Personas</h2>
      {personas.map((p) => (
        <section className="card" key={p.id} aria-label={p.name}>
          <div className="row">
            <strong>{p.name}</strong>
            {isBuiltIn(p) && <span className="badge">Built-in</span>}
            {p.id === active && <span className="badge">Active</span>}
            <span className="grow" />
            {p.id !== active && (
              <button type="button" onClick={() => save({ activePersona: p.id, personalityOn: true }).catch(() => {})}>
                Use
              </button>
            )}
            <button type="button" onClick={() => void add(p)}>
              Duplicate
            </button>
            {!isBuiltIn(p) && (
              <>
                <button type="button" id={`edit-${p.id}`} onClick={() => setEditing(p)}>
                  Edit
                </button>
                <button type="button" onClick={() => invoke<boolean>("persona_export", { id: p.id }).then((saved) => saved && setNote(`Exported ${p.name}.`), fail)}>
                  Export
                </button>
                {confirming === p.id ? (
                  <button type="button" onClick={() => void remove(p)}>
                    Confirm delete
                  </button>
                ) : (
                  <button type="button" onClick={() => setConfirming(p.id)}>
                    Delete
                  </button>
                )}
              </>
            )}
          </div>
          <p className="muted note">{describePersona(p)}</p>
          {editing?.id === p.id && <Editor key={p.id} initial={editing} save={save} onDone={() => closeEditor(p)} />}
        </section>
      ))}
      <div className="row">
        <button type="button" onClick={() => void add(BLANK)}>
          New persona
        </button>
        <button type="button" onClick={() => invoke<Persona | null>("persona_import").then((p) => p && setNote(`Imported ${p.name}.`), fail)}>
          Import…
        </button>
      </div>
      {note && (
        <p className="note" role="status">
          {note}
        </p>
      )}

      <details>
        <summary>Developer</summary>
        <p className="muted">Prompt preview: the exact messages Cursora would send for a sample selection and question.</p>
        <pre className="detail" aria-label="Prompt preview">
          {preview}
        </pre>
      </details>
    </>
  );
}

/** A user persona's fields, with Rust's verdict and the generated instructions live beside them (PER-1.2). */
function Editor({ initial, save, onDone }: { initial: Persona; save: Save; onDone: () => void }) {
  const [p, setP] = useState(initial);
  const [preview, setPreview] = useState<{ ok: boolean; text: string }>();
  useEffect(() => {
    invoke<string>("persona_preview", { persona: clean(p) }).then(
      (text) => setPreview({ ok: true, text }),
      (e) => setPreview({ ok: false, text: String(e) }),
    );
  }, [p]);
  const set = (patch: Partial<Persona>) => setP({ ...p, ...patch });
  const done = () => editSettings(save, "personas", (ps) => ps.map((x) => (x.id === p.id ? clean(p) : x))).then(onDone, (e) => setPreview({ ok: false, text: String(e) }));

  return (
    <form
      className="editor"
      aria-label={`Edit ${initial.name}`}
      onSubmit={(e) => {
        e.preventDefault();
        void done();
      }}
    >
      <label className="row">
        <span>Name</span>
        <input value={p.name} onChange={(e) => set({ name: e.target.value })} autoFocus />
      </label>
      <Choice label="Gender" value={p.gender} choices={GENDERS} onChange={(gender) => set({ gender })} />
      <Choice label="Tone" value={p.tone} choices={TONES} onChange={(tone) => set({ tone })} />
      <label className="row">
        <span>Length</span>
        <select value={p.verbosity} onChange={(e) => set({ verbosity: e.target.value as Persona["verbosity"] })}>
          <option value="brief">Brief</option>
          <option value="normal">Normal</option>
          <option value="detailed">Detailed</option>
        </select>
      </label>
      <label className="row">
        <span>Emoji</span>
        <input type="checkbox" checked={p.emoji} onChange={(e) => set({ emoji: e.target.checked })} />
      </label>
      <label className="row">
        <span>Reply language</span>
        <input value={p.replyLanguage ?? ""} placeholder="Same as my question" onChange={(e) => set({ replyLanguage: e.target.value })} />
      </label>
      <label className="row">
        <span>Your expertise</span>
        <select value={p.userExpertise} onChange={(e) => set({ userExpertise: e.target.value as Persona["userExpertise"] })}>
          <option value="beginner">Beginner</option>
          <option value="intermediate">Intermediate</option>
          <option value="expert">Expert</option>
        </select>
      </label>
      <label>
        Custom instructions <span className="muted">({p.customInstructions.length}/2000)</span>
        <textarea rows={4} value={p.customInstructions} onChange={(e) => set({ customInstructions: e.target.value })} />
      </label>
      {preview && (preview.ok ? <pre className="detail" aria-label="Persona instructions">{preview.text}</pre> : <p role="alert" className="error">{preview.text}</p>)}
      <div className="row">
        <button type="submit" disabled={!preview?.ok}>
          Save
        </button>
        <button type="button" onClick={onDone}>
          Cancel
        </button>
      </div>
    </form>
  );
}

/** A preset value or the user's own words (custom gender, custom tone). */
function Choice({ label, value, choices, onChange }: { label: string; value: string; choices: string[]; onChange: (v: string) => void }) {
  const custom = !choices.includes(value);
  return (
    <div className="row">
      <span>{label}</span>
      <select aria-label={label} value={custom ? "" : value} onChange={(e) => onChange(e.target.value)}>
        {choices.map((c) => (
          <option key={c} value={c}>
            {c[0].toUpperCase() + c.slice(1)}
          </option>
        ))}
        <option value="">Custom…</option>
      </select>
      {custom && <input aria-label={`Custom ${label.toLowerCase()}`} value={value} onChange={(e) => onChange(e.target.value)} />}
    </div>
  );
}
