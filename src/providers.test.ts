import { describe, expect, it } from "vitest";
import { engineProvider, fileStem, newProvider, PRESETS } from "./providers";

describe("providers", () => {
  it("LLM-4.1 presets fill in the kind and base URL for every listed provider, plus Custom", () => {
    const names = PRESETS.map((p) => p.name);
    for (const n of ["OpenAI", "Anthropic", "Google Gemini", "OpenRouter", "Groq", "DeepSeek", "Mistral", "Custom (OpenAI-compatible)"]) {
      expect(names).toContain(n);
    }
    for (const p of PRESETS.filter((p) => p.id !== "custom")) {
      expect(p.baseUrl).toMatch(/^https?:\/\/[^/]+/);
      expect(["openai-compatible", "anthropic", "gemini"]).toContain(p.kind);
    }
    expect(PRESETS.find((p) => p.id === "anthropic")?.kind).toBe("anthropic");
    expect(PRESETS.find((p) => p.id === "gemini")?.kind).toBe("gemini");
    const custom = PRESETS.find((p) => p.id === "custom")!;
    expect([custom.kind, custom.baseUrl]).toEqual(["openai-compatible", ""]);
    const p = newProvider(PRESETS[0], " ", " https://api.openai.com/v1 ", 42);
    expect([p.id, p.name, p.kind, p.baseUrl, p.preset]).toEqual(["p-42", "OpenAI", "openai-compatible", "https://api.openai.com/v1", "openai"]);
  });

  it("LLM-3.2 a chosen .gguf becomes a built-in provider named after the file", () => {
    expect(fileStem("D:\\models\\Qwen2.5-0.5B-Instruct-Q4_K_M.gguf")).toBe("Qwen2.5-0.5B-Instruct-Q4_K_M");
    const p = engineProvider("/home/me/llama.gguf", null, 7);
    expect([p.kind, p.models, p.engine?.gguf, p.engine?.idleMinutes]).toEqual(["builtin", ["llama"], "/home/me/llama.gguf", 10]);
    expect(p.engine?.lifetime).toBe("always"); // LLM-3.5: the default
  });
});
