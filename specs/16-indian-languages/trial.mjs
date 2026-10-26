// LNG-4.1's trial: Cursora-shaped requests in Hindi, Tamil and Hinglish to an OpenAI-compatible server.
// node specs/16-indian-languages/trial.mjs http://127.0.0.1:18573/v1 [model]   (a cloud key comes from TRIAL_API_KEY, never the command line)
// The wording is read from the app's own sources, so the trial sends what Cursora sends. Results go in evaluation.md.
import { readFileSync } from "node:fs";

const [base = "http://127.0.0.1:18573/v1", model = "x"] = process.argv.slice(2);
const key = process.env.TRIAL_API_KEY;
const src = (path) => readFileSync(new URL(`../../${path}`, import.meta.url), "utf8");
const constant = (file, name) => JSON.parse(`"${src(file).match(new RegExp(`const ${name}: &str = "((?:[^"\\\\]|\\\\.)*)";`))[1]}"`);
const BASE = constant("src-tauri/src/prompt.rs", "BASE");
const UNTRUSTED = constant("src-tauri/src/prompt.rs", "UNTRUSTED");
const REPLACE_RULE = constant("src-tauri/src/skills.rs", "REPLACE_RULE");
/** A built-in skill's instructions, filled in as `skills::instructions` does (SKL-1.2, WRT-5.1). */
const skill = (name, myLanguage) => {
  const lines = src(`src-tauri/skills/${name}.md`).replace(/\r/g, "").split("\n");
  const end = lines.indexOf("---", 1);
  const text = lines.slice(end + 1).join("\n").trim().replaceAll("{{input}}", "").replaceAll("{{my_language}}", myLanguage);
  return lines.slice(1, end).includes("output: replace") ? `${text}\n${REPLACE_RULE}` : text;
};
const HINGLISH = "Hinglish (Hindi written in English letters, the way people type it in chats)"; // prompt::language_name
const EN = "The meeting has moved to Friday at 3 pm. Please bring the signed contract and your laptop.";
const HI = "कल की बैठक रद्द हो गई है। कृपया अपनी रिपोर्ट सोमवार तक भेज दें।";
const CI = "Compound interest is interest calculated on the initial principal and also on the interest added in earlier periods.";
const cases = [
  ["English into Hindi", null, skill("translate", "Hindi"), EN, "/translate"],
  ["English into Tamil", null, skill("translate", "Tamil"), EN, "/translate"],
  ["Hindi into English", null, skill("translate", "Hindi"), HI, "/translate"],
  ["English into Hinglish", null, skill("translate", HINGLISH), EN, "/translate"],
  ["Explain, answering in Hindi", "Hindi", skill("explain", "Hindi"), CI, "/explain"],
  ["Explain, answering in Tamil", "Tamil", skill("explain", "Tamil"), CI, "/explain"],
];
for (const [name, lang, instructions, text, question] of cases) {
  const system = [BASE, ...(lang ? [`Always reply in ${lang}.`] : []), `Instructions for this request:\n${instructions}`, UNTRUSTED].join("\n\n");
  const started = Date.now();
  let out;
  try {
    const r = await fetch(`${base}/chat/completions`, {
      method: "POST",
      headers: { "Content-Type": "application/json", ...(key ? { Authorization: `Bearer ${key}` } : {}) },
      body: JSON.stringify({
        model,
        temperature: 0.3,
        messages: [
          { role: "system", content: system },
          { role: "user", content: `<selected_text source="Notepad">\n${text}\n</selected_text>\n\n${question}` },
        ],
      }),
    });
    out = r.ok ? (await r.json()).choices[0].message.content.trim() : `ERROR ${r.status}: ${await r.text()}`;
  } catch (e) {
    out = `ERROR ${e}`;
  }
  console.log(`=== ${name} (${((Date.now() - started) / 1000).toFixed(1)} s)\n${out}\n`);
}
