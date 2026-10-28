# 23: Faster local answers with a graphics card (GPU)

Status: **Approved**¹ (2026-10-28).

## Scope
- **In:** the built-in engine answering on a dedicated graphics card (NVIDIA, AMD or Intel Arc) when the PC has one, through llama.cpp's Vulkan build, which replaces the CPU-only build in the installers. A switch to turn it off.
- **Out:**
  - Integrated graphics (Intel Iris and UHD, AMD's Radeon Graphics in processors): on the laptop, Intel Iris Xe was 2.4 times slower than the processor
  - CUDA and ROCm builds: 150 to 400 MB each, against 32 MB for Vulkan, which runs on every vendor's cards
  - Choosing between two dedicated cards: llama.cpp picks
  - GPUs for other local servers (LM Studio, Ollama), which manage their own

Update #4 from the list of 2026-10-28 ([improvements batch 2](../../memory/improvements-batch-2.md)).

## Requirements

### GPU-1 The graphics card
User story: As a user with a gaming laptop, I want local answers as fast as my graphics card allows, without setting anything up.
- GPU-1.1 (auto) WHILE the PC has a dedicated graphics card that Vulkan reports, and "Use the graphics card" is on (the default), THE built-in engine SHALL put as much of the model on it as fits. Integrated graphics SHALL never be used.
- GPU-1.2 (manual) On such a PC, answers from the built-in engine SHALL come several times faster than on the processor.
- GPU-1.3 (auto) WHILE there's no dedicated card, Vulkan is missing, or the switch is off, THE engine SHALL run on the processor as before, never failing because of the graphics card.
- GPU-1.4 (manual) Settings → Models → Built-in engine SHALL have **Use the graphics card**, with the card's name when one was found, or a line saying the processor answers. Turning it off or on SHALL take effect from the next question.

## Defaults taken under the go-ahead of 2026-10-28
1. **Vulkan**, the one build that runs on NVIDIA, AMD and Intel cards, for both Windows and Linux. The installers grow by about 12 MB.
2. **Dedicated cards only**, decided by the Vulkan driver's own device type rather than the card's name. Measured on the laptop (i5-11300H, GTX 1650, Iris Xe), in words written per second: Small 20.8 on the processor, 96.8 on the GTX 1650, 8.7 on the Iris Xe; Medium 8.3, 40.8 and 3.6. llama.cpp's default split across both cards was slower than the processor (14.4 and 5.6).
3. **On by default**, since it only applies when a dedicated card is there.

## Open questions
None.

¹ Approved under the user's go-ahead of 2026-10-28 to design, build and merge updates 1 and 3 to 9, with the user reviewing everything afterwards. The choices made for the user are listed above.
