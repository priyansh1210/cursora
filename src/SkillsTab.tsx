import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { useEnabledModels } from "./Models";
import type { Fields, Invalid, Skill } from "./skills";
import type { Settings } from "./useSettings";

type Listing = { skills: Skill[]; invalid: Invalid[] };
type Form = { fields: Fields; editing: string | null; key: number };

const EMPTY: Fields = { name: "", description: "", args: "", vision: false, model: "", temperature: null, template: "" };
const VARIABLES = ["input", "app", "language", "date"];
const SAMPLE = "The Eiffel Tower is a wrought-iron lattice tower in Paris, France. It was finished in 1889 for the World's Fair.";
const fieldsOf = (s: Skill): Fields => ({ name: s.name, description: s.description, args: s.args ?? "", vision: s.vision, model: s.model ?? "", temperature: s.temperature, template: s.template });
const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

/** Settings → Skills (SKL-4.1, 4.6). */
export default function SkillsTab({ settings }: { settings: Settings }) {
  const [listing, setListing] = useState<Listing>({ skills: [], invalid: [] });
  const [form, setForm] = useState<Form>();
  const [confirming, setConfirming] = useState<string>();
  const [note, setNote] = useState<string>();
  const opener = useRef<HTMLElement | null>(null);
  const refresh = () => invoke<Listing>("skills_list").then(setListing);
  // Skills are read from the folder on use; this keeps the list within a second of files changed outside Cursora (SKL-4.5).
  useEffect(() => {
    void refresh();
    const timer = setInterval(() => void refresh(), 500);
    return () => clearInterval(timer);
  }, []);
  const open = (fields: Fields, editing: string | null) => {
    opener.current = document.activeElement as HTMLElement | null;
    setNote(undefined);
    setForm({ fields, editing, key: Date.now() });
  };
  // A draft from /new-skill or "Save as skill" in the bubble (SKL-4.2, 4.3).
  useEffect(() => {
    const take = () => invoke<string | null>("skill_draft").then((t) => t !== null && open({ ...EMPTY, template: t }, null));
    void take();
    const off = listen("skill-draft", take);
    return () => void off.then((f) => f());
  }, []);
  const close = () => {
    setForm(undefined);
    void refresh();
    requestAnimationFrame(() => opener.current?.focus()); // the form's buttons are gone; keyboard users keep their place
  };
  const fail = (e: unknown) => setNote(String(e));
  const builtIns = listing.skills.filter((s) => !s.file || s.overrides).map((s) => s.name);

  return (
    <>
      <h2>Skills</h2>
      <p className="muted">Type / in the bubble to use them. Your own skills are Markdown files in the skills folder.</p>
      {form && <SkillForm key={form.key} form={form} settings={settings} builtIns={builtIns} onDone={close} />}
      {listing.skills.map((s) => (
        <section className="card" key={s.name} aria-label={`/${s.name}`}>
          <div className="row">
            <strong>/{s.name}</strong>
            {s.args && <code>{s.args}</code>}
            {!s.file && <span className="badge">Built-in</span>}
            {s.overrides && <span className="badge">Overrides built-in</span>}
            {s.vision && <span className="badge">Needs vision</span>}
            <span className="grow" />
            {s.file ? (
              <button type="button" onClick={() => open(fieldsOf(s), s.file)}>
                Edit
              </button>
            ) : (
              <button type="button" onClick={() => open(fieldsOf(s), null)}>
                Duplicate
              </button>
            )}
            <button type="button" onClick={() => invoke<boolean>("skill_export", { name: s.name }).then((saved) => saved && setNote(`Exported /${s.name}.`), fail)}>
              Export
            </button>
            {s.file && (
              <button
                type="button"
                onClick={() =>
                  confirming === s.file ? invoke("skill_delete", { file: s.file }).then(() => (setConfirming(undefined), refresh()), fail) : setConfirming(s.file ?? undefined)
                }
              >
                {confirming === s.file ? "Confirm delete" : "Delete"}
              </button>
            )}
          </div>
          <p className="muted note">{s.description}</p>
        </section>
      ))}
      {listing.invalid.length > 0 && (
        <>
          <h2>Files with problems</h2>
          {listing.invalid.map((i) => (
            <p key={i.file} className="error note">
              {fileName(i.file)}
              {i.line > 0 && `, line ${i.line}`}: {i.error}
            </p>
          ))}
        </>
      )}
      <div className="row">
        <button type="button" onClick={() => open(EMPTY, null)}>
          New skill
        </button>
        <button type="button" onClick={() => invoke<string | null>("skill_import").then((n) => n && void (setNote(`Imported /${n}.`), refresh()), fail)}>
          Import…
        </button>
        <button type="button" onClick={() => invoke("open_folder", { kind: "skills" }).catch(fail)}>
          Open folder
        </button>
      </div>
      {note && (
        <p className="note" role="status">
          {note}
        </p>
      )}
    </>
  );
}

/** The skill's fields, Rust's verdict and the instructions as they will read, and a Test run (SKL-4.1). */
function SkillForm({ form, settings, builtIns, onDone }: { form: Form; settings: Settings; builtIns: string[]; onDone: () => void }) {
  const [f, setF] = useState(form.fields);
  const [check, setCheck] = useState<{ ok: boolean; text: string }>();
  const [sample, setSample] = useState(SAMPLE);
  const [result, setResult] = useState<string>();
  const models = useEnabledModels(settings);
  const template = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    invoke<string>("skill_check", { fields: f }).then(
      (text) => setCheck({ ok: true, text }),
      (e) => setCheck({ ok: false, text: String(e) }),
    );
  }, [f]);
  const set = (patch: Partial<Fields>) => setF({ ...f, ...patch });
  const insert = (v: string) => {
    const t = template.current;
    const [from, to] = [t?.selectionStart ?? f.template.length, t?.selectionEnd ?? f.template.length];
    const text = `{{${v}}}`;
    set({ template: f.template.slice(0, from) + text + f.template.slice(to) });
    requestAnimationFrame(() => {
      t?.focus();
      t?.setSelectionRange(from + text.length, from + text.length);
    });
  };
  const save = () => invoke("skill_save", { fields: f, editing: form.editing }).then(onDone, (e) => setCheck({ ok: false, text: String(e) }));
  const test = () => {
    setResult("Running…");
    invoke<string>("skill_test", { fields: f, sample }).then(setResult, (e) => setResult(String(e)));
  };
  const modelChoices = models.map((m) => `${m.providerName}/${m.model}`);

  return (
    <form
      className="card editor"
      aria-label={form.editing ? `Edit /${form.fields.name}` : "New skill"}
      onSubmit={(e) => {
        e.preventDefault();
        void save();
      }}
    >
      <h3>{form.editing ? `Edit /${form.fields.name}` : "New skill"}</h3>
      <label className="row">
        <span>Name</span>
        <input value={f.name} placeholder="eli5" onChange={(e) => set({ name: e.target.value })} autoFocus />
      </label>
      <label className="row">
        <span>Description</span>
        <input value={f.description} placeholder="Explain like I'm five" onChange={(e) => set({ description: e.target.value })} />
      </label>
      <label className="row">
        <span>Args hint</span>
        <input value={f.args} placeholder="<topic>" onChange={(e) => set({ args: e.target.value })} />
      </label>
      <label className="row">
        <span>Needs vision</span>
        <input type="checkbox" checked={f.vision} onChange={(e) => set({ vision: e.target.checked })} />
      </label>
      <label className="row">
        <span>Model</span>
        <select value={f.model} onChange={(e) => set({ model: e.target.value })}>
          <option value="">The active model</option>
          {modelChoices.map((m) => (
            <option key={m}>{m}</option>
          ))}
          {f.model && !modelChoices.includes(f.model) && <option>{f.model}</option>}
        </select>
      </label>
      <label className="row">
        <span>Temperature</span>
        <input
          type="number"
          min={0}
          max={2}
          step={0.1}
          value={f.temperature ?? ""}
          placeholder="Model default"
          onChange={(e) => set({ temperature: e.target.value === "" ? null : Number(e.target.value) })}
        />
      </label>
      <label>
        Instructions
        <textarea ref={template} rows={5} value={f.template} onChange={(e) => set({ template: e.target.value })} />
      </label>
      <div className="row" role="group" aria-label="Insert a variable">
        {VARIABLES.map((v) => (
          <button type="button" key={v} onClick={() => insert(v)}>{`{{${v}}}`}</button>
        ))}
      </div>
      <p className="muted note">The selected text or files are always attached; {"{{input}}"} is what is typed after the command.</p>
      {check && (check.ok ? <pre className="detail" aria-label="The instructions as they will read">{check.text}</pre> : <p role="alert" className="error">{check.text}</p>)}
      {builtIns.includes(f.name.trim()) && !form.editing && <p className="muted note">Saving makes your own /{f.name.trim()}, which replaces the built-in one.</p>}
      <label>
        Sample text for Test
        <textarea rows={3} value={sample} onChange={(e) => setSample(e.target.value)} />
      </label>
      <button type="button" onClick={test} disabled={!check?.ok}>
        Test
      </button>
      {result && (
        <pre className="detail" role="status" aria-label="Test answer">
          {result}
        </pre>
      )}
      <div className="row">
        <button type="submit" disabled={!check?.ok}>
          Save
        </button>
        <button type="button" onClick={onDone}>
          Cancel
        </button>
      </div>
    </form>
  );
}
