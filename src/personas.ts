// Mirrors src-tauri/src/persona.rs. Validation and the persona wording live in Rust only (spec 05 design).
export type Persona = {
  id: string;
  name: string;
  gender: string;
  tone: string;
  verbosity: "brief" | "normal" | "detailed";
  emoji: boolean;
  replyLanguage: string | null;
  userExpertise: "beginner" | "intermediate" | "expert";
  customInstructions: string;
};

export const GENDERS = ["female", "male", "neutral"];
export const TONES = ["friendly", "professional", "witty", "calm", "sarcastic"];

export const BLANK: Persona = { id: "", name: "New persona", gender: "neutral", tone: "friendly", verbosity: "normal", emoji: false, replyLanguage: null, userExpertise: "intermediate", customInstructions: "" };

export const isBuiltIn = (p: Persona) => p.id.startsWith("preset:");

/** As the editor saves it: one-line fields trimmed, an empty language meaning "same as my question". */
export function clean(p: Persona): Persona {
  const language = p.replyLanguage?.trim();
  return { ...p, name: p.name.trim(), gender: p.gender.trim(), tone: p.tone.trim(), replyLanguage: language || null };
}

/** One line for lists: "female · professional · detailed answers · for experts". */
export function describePersona(p: Persona): string {
  return [
    p.gender,
    p.tone,
    p.verbosity !== "normal" && `${p.verbosity} answers`,
    p.emoji && "emoji",
    p.userExpertise !== "intermediate" && `for ${p.userExpertise}s`,
    p.replyLanguage && `replies in ${p.replyLanguage}`,
  ]
    .filter(Boolean)
    .join(" · ");
}
