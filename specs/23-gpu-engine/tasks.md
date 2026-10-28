# 23: Faster local answers with a graphics card (tasks)

Status: **In progress**.

- [x] 1. `fetch-llama`: the Vulkan archives; `keep()` test. Covers GPU-1.1.
- [x] 2. `gpu.rs` with `ash`; `EngineSettings.gpu`; `args()` and `Files`; the `engine_gpu` command; tests. Covers GPU-1.1, GPU-1.3.
- [x] 3. The switch in Models; the manual. Covers GPU-1.4.
- [ ] 4. All checks; by hand on Windows; then `/ponytail-review`. 2026-10-28: the review found that asking for the card left NVIDIA's Vulkan driver in Cursora, taking its private memory from 6 MB to 65 MB (past §8's 60 MB), and that Settings asked on the main thread. Fixed: Cursora asks a child (`cursora --gpu`), and the command is async. Measured again on the test copy after Settings → Models named the card: 7 MB private, no driver files loaded, the child gone.

## Manual checklist
- [ ] GPU-1.2 The same question with the switch on and off: the time from the log, on the laptop's GTX 1650. Windows ✓ (2026-10-28, test copy, Small, the model loaded per question: on the processor 19.7 s to the first word and 4.4 s of writing; on the GTX 1650 12.1 s and 0.85 s, about 5 times faster writing, as llama-bench measured; the log said "on the processor" and "on the graphics card") Linux ☐ (needs a Linux PC with a card; WSL has none, issue 68)
- [ ] GPU-1.4 The switch, the card's name, and the processor line without one. Windows ✓ the switch on with "NVIDIA GeForce GTX 1650: answers come several times faster" (2026-10-28) ☐ the line on a PC without a card Linux X11 ☐
