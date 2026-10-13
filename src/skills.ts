// Mirrors src-tauri/src/skills.rs. Parsing and validation live in Rust only; this filters the autocomplete.
export type Entry = { name: string; args: string; description: string; command: boolean };
export type Skill = {
  name: string;
  description: string;
  args: string | null;
  vision: boolean;
  model: string | null;
  temperature: number | null;
  template: string;
  file: string | null;
  overrides: boolean;
};
export type Invalid = { file: string; line: number; error: string };
export type Fields = { name: string; description: string; args: string; vision: boolean; model: string; temperature: number | null; template: string };

/** The command being typed: the text after "/" while there is no space yet, or null. */
export function typedCommand(draft: string): string | null {
  return /^\/\S*$/.test(draft) ? draft.slice(1).toLowerCase() : null;
}

/** SKL-2.1: names that start with the typed text, then names that contain its letters in order; each group alphabetical. */
export function matches(entries: Entry[], typed: string): Entry[] {
  const byName = (a: Entry, b: Entry) => a.name.localeCompare(b.name);
  const prefix = entries.filter((e) => e.name.startsWith(typed)).sort(byName);
  const fuzzy = entries.filter((e) => !e.name.startsWith(typed) && inOrder(e.name, typed)).sort(byName);
  return [...prefix, ...fuzzy];
}

function inOrder(name: string, typed: string): boolean {
  let i = 0;
  for (const c of name) if (c === typed[i]) i++;
  return i === typed.length;
}

/** The open list's keys: ↑ and ↓ move (wrapping), Tab or Enter complete, Esc closes. Other keys belong to the input.
 *  Enter on a name already typed in full (`exact`) is left to the input, so `/help` runs with one Enter. */
export function listKey(key: string, selected: number, count: number, exact = false): number | "complete" | "close" | null {
  if (key === "Enter" && exact) return null;
  switch (key) {
    case "ArrowDown":
      return (selected + 1) % count;
    case "ArrowUp":
      return (selected - 1 + count) % count;
    case "Tab":
    case "Enter":
      return "complete";
    case "Escape":
      return "close";
    default:
      return null;
  }
}
