# Cursora

Select text in any app, press **Ctrl+G**, and ask an AI about it in a small chat bubble right at your cursor. Cursora works with local models (Ollama, LM Studio, any OpenAI-compatible server, or a built-in engine for `.gguf` files) and with cloud API keys (Claude, GPT, Gemini, …). It has a personality you can customize or switch off.

**Status:** pre-alpha. The specs are in [specs/](specs/README.md).

## Develop (Windows)
Prerequisites:
- Rust (rustup, stable-msvc)
- Build Tools for Visual Studio 2022, with the "Desktop development with C++" workload. This is the compiler only, not the IDE.
- Node.js 22

```
npm install
npm run tauri dev
```
