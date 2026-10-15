import { describe, expect, it } from "vitest";
import { ERROR_KINDS, errorText, header, keyAction, marks } from "./conversation";

describe("chat helpers", () => {
  it("BUB-4.1 Enter sends; Shift+Enter and IME composition insert a newline", () => {
    expect(keyAction({ key: "Enter", shiftKey: false, isComposing: false })).toBe("send");
    expect(keyAction({ key: "Enter", shiftKey: true, isComposing: false })).toBeNull();
    expect(keyAction({ key: "Enter", shiftKey: false, isComposing: true })).toBeNull();
    expect(keyAction({ key: "a", shiftKey: false, isComposing: false })).toBeNull();
  });

  it("BUB-4.4 every error kind has a plain-language message; auth, billing and config errors offer Models settings", () => {
    for (const kind of ERROR_KINDS) {
      const e = errorText(kind, "raw detail");
      expect(e.text.length).toBeGreaterThan(10);
      expect(e.openSettings).toBe(["auth", "billing", "model_not_found", "blocked_local_only"].includes(kind));
    }
    expect(errorText("unknown", "raw detail").text).toContain("raw detail");
  });

  it("PER-2.3 with personality off the header says so and still names the model", () => {
    const model = { name: "llama3.2", provider: "Ollama", locality: "local" as const, contextTokens: null };
    expect(header({ persona: null, model })).toEqual({ persona: "Personality off", model: "llama3.2", badge: "Local" });
  });

  it("BUB-7.1 the header shows persona or 'Personality off', the model and its badge", () => {
    const model = { name: "llama3.2", provider: "Ollama", locality: "local" as const, contextTokens: null };
    expect(header({ persona: "Cursora", model })).toEqual({ persona: "Cursora", model: "llama3.2", badge: "Local" });
    expect(header({ persona: null, model }).persona).toBe("Personality off");
    expect(header({ persona: null, model: { ...model, locality: "lan" } }).badge).toBe("LAN");
    expect(header({ persona: null, model: { ...model, provider: "OpenAI", locality: "cloud" } }).badge).toBe("Cloud · OpenAI");
  });

  it("PRI-1.2 while local-only mode is on, the badge says so", () => {
    const model = { name: "llama3.2", provider: "Ollama", locality: "local" as const, contextTokens: null };
    expect(header({ persona: null, model }, true).badge).toBe("Local-only");
    expect(header({ persona: null, model: { ...model, locality: "lan" } }, true).badge).toBe("Local-only");
  });

  it("PRI-3.6 the preview marks each placeholder, and only those", () => {
    expect(marks("Mail [EMAIL_1] or [EMAIL_10], not [EMAIL_9].", ["[EMAIL_1]", "[EMAIL_10]"])).toEqual([
      { text: "Mail ", mark: false },
      { text: "[EMAIL_1]", mark: true },
      { text: " or ", mark: false },
      { text: "[EMAIL_10]", mark: true },
      { text: ", not [EMAIL_9].", mark: false },
    ]);
    expect(marks("[CARD_1]", ["[CARD_1]"])).toEqual([{ text: "[CARD_1]", mark: true }]);
    expect(marks("nothing hidden", [])).toEqual([{ text: "nothing hidden", mark: false }]);
  });
});
