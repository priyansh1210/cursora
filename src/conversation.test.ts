import { describe, expect, it } from "vitest";
import { ERROR_KINDS, errorText, header, keyAction } from "./conversation";

describe("chat helpers", () => {
  it("BUB-4.1 Enter sends; Shift+Enter and IME composition insert a newline", () => {
    expect(keyAction({ key: "Enter", shiftKey: false, isComposing: false })).toBe("send");
    expect(keyAction({ key: "Enter", shiftKey: true, isComposing: false })).toBeNull();
    expect(keyAction({ key: "Enter", shiftKey: false, isComposing: true })).toBeNull();
    expect(keyAction({ key: "a", shiftKey: false, isComposing: false })).toBeNull();
  });

  it("BUB-4.4 every error kind has a plain-language message; config errors offer Models settings", () => {
    for (const kind of ERROR_KINDS) {
      const e = errorText(kind, "raw detail");
      expect(e.text.length).toBeGreaterThan(10);
      expect(e.openSettings).toBe(["auth", "model_not_found", "blocked_local_only"].includes(kind));
    }
    expect(errorText("unknown", "raw detail").text).toContain("raw detail");
  });

  it("BUB-7.1 the header shows persona or 'Personality off', the model and its badge", () => {
    const model = { name: "llama3.2", provider: "Ollama", locality: "local" as const, contextTokens: null };
    expect(header({ persona: "Cursora", model })).toEqual({ persona: "Cursora", model: "llama3.2", badge: "🔒 Local" });
    expect(header({ persona: null, model }).persona).toBe("Personality off");
    expect(header({ persona: null, model: { ...model, locality: "lan" } }).badge).toBe("🏠 LAN");
    expect(header({ persona: null, model: { ...model, provider: "OpenAI", locality: "cloud" } }).badge).toBe("☁ OpenAI");
  });
});
