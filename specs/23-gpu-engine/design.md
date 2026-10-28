# 23: Faster local answers with a graphics card (design)

Status: **Approved**¹ (2026-10-28). Implements [requirements.md](requirements.md). The pinned llama.cpp build becomes its Vulkan build. Cursora asks the Vulkan driver once whether a dedicated card exists, and tells llama-server to use none when it doesn't, so it never falls back to integrated graphics.

## Overview
```
fetch-llama: llama-b11570-bin-win-vulkan-x64.zip / -ubuntu-vulkan-x64.tar.gz (pinned SHA-256)
             ggml-vulkan.dll / libggml-vulkan.so is a backend llama.cpp loads only if Vulkan is there
engine::ensure_running
  gpu = engine.gpu (the switch) && gpu::dedicated().is_some()           (GPU-1.1, 1.3)
  args(…, gpu): gpu → llama.cpp's own choice (dedicated cards, -ngl auto, --fit on)
                no gpu → --device none
  Files includes gpu, so turning the switch restarts the engine at the next question   (GPU-1.4)
gpu::dedicated(): ash → vkEnumeratePhysicalDevices → the first DISCRETE_GPU's name; asked once
Models → Built-in engine: [Use the graphics card]  "NVIDIA GeForce GTX 1650" / "No dedicated graphics card: the processor answers."
```

## Components
| File | Change |
|---|---|
| `scripts/fetch-llama.mjs` | The Vulkan archives and their SHA-256. |
| `src-tauri/src/llm/gpu.rs` (new) | `dedicated()`, and the `engine_gpu` command for Settings. |
| `src-tauri/src/llm/engine.rs` | `args()` takes whether to use the card; `Files` includes it. |
| `src-tauri/src/llm/mod.rs` | `EngineSettings.gpu`, on by default. |
| `src-tauri/Cargo.toml` | `ash`, which loads the Vulkan library at run time, so no Vulkan is needed to build. |
| `src/Models.tsx`, `src/providers.ts` | The switch and the card's name. |

## Decisions
1. **llama.cpp chooses among dedicated cards, Cursora only says "none".** Started normally on the laptop, llama-server used just the GTX 1650 (`using device Vulkan1`), all 29 layers of Small. It skips integrated graphics when a dedicated card exists. When there's none, it would use the integrated one, so Cursora passes `--device none` then.
2. **The driver's device type, not the name.** Names don't tell: Intel's "Arc Graphics" is integrated in some processors and "Arc A770" is a card, and AMD's processors say "Radeon Graphics". `VkPhysicalDeviceProperties.deviceType == DISCRETE_GPU` does tell.
3. **`ash`, not a GPU library.** It's the plain Vulkan binding. Its loaded mode opens `vulkan-1.dll` / `libvulkan.so.1` at run time, and fails cleanly where Vulkan isn't installed (GPU-1.3).
4. **Asked once per run.** Creating a Vulkan instance takes about 50 ms; cards don't change while Cursora runs.

## Test strategy
- Rust: `args()` with and without the card (GPU-1.1, 1.3); the switch's default and older settings files (GPU-1.1); `dedicated()` runs without panicking anywhere, and names the GTX 1650 on the laptop when run by hand.
- JS: `keep()` keeps the Vulkan backend (GPU-1.1).
- By hand on Windows: the same question with the switch on and off, its time from the log (GPU-1.2); the switch and the name in Settings (GPU-1.4). WSL has no GPU, so on Linux the engine must answer on the processor (GPU-1.3).

¹ Approved under the user's go-ahead of 2026-10-28 (requirements.md).
