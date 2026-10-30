import { Fragment } from "react";
import type { Detail, Save, Settings } from "./useSettings";

/** The boxes of My details, by section (FIL-1.1). `FORM` in src-tauri/src/fill/mod.rs has the same names, in this order. */
export const SECTIONS: [string, string[]][] = [
  ["You", ["Full name", "First name", "Last name", "Date of birth", "Gender"]],
  ["Family", ["Father's name", "Mother's name"]],
  ["Contact", ["Mobile number", "Email"]],
  ["Address", ["Address line 1", "Address line 2", "City", "State", "PIN code", "Country"]],
  ["Study and work", ["College or company", "Course or job title"]],
  ["Online", ["LinkedIn", "GitHub", "Website"]],
];

/** The details the tab saves (FIL-1.2): the boxes with a value; "Address", the two address lines joined, for a form with
 *  one address box; then each line of More details split at its first colon, but not a bare link's ("https://…"). */
export function details(box: (label: string) => string, more: string): Detail[] {
  const form = SECTIONS.flatMap(([, names]) => names).map((label) => ({ label, value: box(label).trim() }));
  const address = form.filter((d) => d.label.startsWith("Address line")).map((d) => d.value).filter(Boolean).join(", ");
  const lines = more.split("\n").map((line) => {
    const colon = line.indexOf(":");
    return { label: colon < 0 ? "" : line.slice(0, colon).trim(), value: line.slice(colon + 1).trim() };
  });
  return [...form, { label: "Address", value: address }, ...lines].filter((d) => d.label && d.value && !d.value.startsWith("//"));
}

/** Settings → My details (FIL-1.1): a form, saved when a box loses focus, like About you. More details is kept as typed
 *  too, for this tab. */
export default function MyDetailsTab({ settings, save }: { settings: Settings; save: Save }) {
  const value = (label: string) => settings.myDetails.find((d) => d.label === label)?.value ?? "";
  const changed = (form: HTMLFormElement) => {
    const data = new FormData(form);
    const moreDetails = String(data.get("more"));
    const myDetails = details((label) => String(data.get(label)), moreDetails);
    if (JSON.stringify(myDetails) !== JSON.stringify(settings.myDetails) || moreDetails !== settings.moreDetails) void save({ myDetails, moreDetails }).catch(() => {});
  };

  return (
    // No spell check: names and addresses aren't words.
    <form spellCheck={false} onSubmit={(e) => e.preventDefault()} onBlur={(e) => changed(e.currentTarget)}>
      <p className="muted">
        In a form, press {settings.hotkeys.ask} with nothing selected, then Fill this form. Your details stay on this computer: an AI model sees only their
        names, never what you write here. Boxes left empty are left for you.
      </p>
      {SECTIONS.map(([section, names]) => (
        <Fragment key={section}>
          <h2>{section}</h2>
          <div className="group">
            {names.map((label) => (
              <label className="row" key={label}>
                <span>{label}</span>
                <input name={label} defaultValue={value(label)} maxLength={500} />
              </label>
            ))}
          </div>
        </Fragment>
      ))}
      <h2>More details</h2>
      <div className="group">
        <div className="row">
          <textarea
            className="more-details"
            name="more"
            aria-label="More details"
            aria-describedby="more-note"
            rows={4}
            maxLength={5000}
            defaultValue={settings.moreDetails}
            placeholder={"Blood group: O+\nRoll number: 21BCE1234"}
          />
        </div>
        <p className="muted" id="more-note">
          Anything else a form may ask, one per line: its name, a colon, then the value.
        </p>
      </div>
    </form>
  );
}
