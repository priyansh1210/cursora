# 23: Faster local answers with a graphics card (tasks)

Status: **In progress**.

- [ ] 1. `fetch-llama`: the Vulkan archives; `keep()` test. Covers GPU-1.1.
- [ ] 2. `gpu.rs` with `ash`; `EngineSettings.gpu`; `args()` and `Files`; the `engine_gpu` command; tests. Covers GPU-1.1, GPU-1.3.
- [ ] 3. The switch in Models; the manual. Covers GPU-1.4.
- [ ] 4. All checks; by hand on Windows; then `/ponytail-review`.

## Manual checklist
- [ ] GPU-1.2 The same question with the switch on and off: the time from the log, on the laptop's GTX 1650. Windows ☐ Linux ☐ (needs a Linux PC with a card)
- [ ] GPU-1.4 The switch, the card's name, and the processor line without one. Windows ☐ Linux X11 ☐
