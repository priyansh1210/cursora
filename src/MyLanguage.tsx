import { useEffect, useState } from "react";
import type { Save, Settings } from "./useSettings";

/** LNG-1.1: suggested first, any other language can be typed. Hinglish is labelled, since its name alone doesn't say what it is. */
const LANGUAGES: [string, string?][] = [
  ["English"],
  ["Hindi"],
  ["Hinglish", "Hindi in English letters"],
  ["Bengali"],
  ["Marathi"],
  ["Telugu"],
  ["Tamil"],
  ["Gujarati"],
  ["Urdu"],
  ["Kannada"],
  ["Odia"],
  ["Malayalam"],
  ["Punjabi"],
  ["Assamese"],
];

/** The suggestions for any language field: `<input list="languages">`. */
export function LanguageSuggestions() {
  return (
    <datalist id="languages">
      {LANGUAGES.map(([value, label]) => (
        <option key={value} value={value} label={label} />
      ))}
    </datalist>
  );
}

/** My language, in Settings → General and Setup (LNG-1.1). Saved on leaving the field, on Enter or on picking a suggestion,
 * so a half-typed name isn't checked; Rust refuses a bad one, and the page shows why. */
export function MyLanguageField({ settings, save }: { settings: Settings; save: Save }) {
  const [draft, setDraft] = useState(settings.myLanguage);
  const [refused, setRefused] = useState(false); // then saving the old name again clears the page's error
  useEffect(() => setDraft(settings.myLanguage), [settings.myLanguage]);
  const commit = (value: string) => {
    const language = value.trim() || "English";
    setDraft(language);
    if (language !== settings.myLanguage || refused) save({ myLanguage: language }).then(() => setRefused(false), () => setRefused(true));
  };
  return (
    <label className="row">
      <span>My language</span>
      <input
        list="languages"
        value={draft}
        onChange={(e) => {
          setDraft(e.target.value);
          if ((e.nativeEvent as InputEvent).inputType === "insertReplacementText") commit(e.target.value); // picked from the list
        }}
        onBlur={(e) => commit(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && commit(e.currentTarget.value)}
      />
      <LanguageSuggestions />
    </label>
  );
}
