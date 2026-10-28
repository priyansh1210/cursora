# 24: Smart routing (tasks)

Status: **In progress**.

- [x] 1. `ModelSettings::quick`, `llm::quick`; routing in `resolve()`, with tests. Covers ROU-1.2, ROU-1.3, ROU-1.4.
- [x] 2. The fallback in `begin()` and `health()`, with tests. Covers ROU-2.1, ROU-2.2, ROU-2.3.
- [x] 3. Smart routing in Settings → Models; the manual. Covers ROU-1.1.
- [x] 4. All checks; by hand on Windows; then `/ponytail-review`. 2026-10-28: the first check showed the header naming the default model beside a "Local" badge after a quick job, and the fallback note carrying a long URL; now the header keeps the default model and its badge, each routed answer has a note, and the fallback note names the reason in plain words. The review found that quick jobs in Indian languages went to a small model that can't do them (now English text only, and not while answers come in another language), and that the fallback note claimed an answer even when the quick model failed too (now "was asked instead").

## Manual checklist
- [ ] ROU-1.1 Smart routing in Models: Off by default, only local and LAN models listed, saved. Windows ✓ (2026-10-28, test copy: the list under "Smart routing" with the built-in model chosen, and the line under it) Linux X11 ☐
- [ ] ROU-1.5 A quick job's note and "What will be sent" name the quick model; the header keeps the default. Windows ✓ (2026-10-28: `/fix` previewed "To Local · qwen2.5-1.5b", answered with "Quick job: answered by qwen2.5-1.5b-instruct-q4_k_m."; the header stayed "cloud-x · Cloud") Linux X11 ☐
- [ ] ROU-2.3 With the default model unreachable, the bubble opens with its input, and the quick model answers with the note. Windows ✓ (2026-10-28: a cloud provider at `https://unreachable.invalid/v1` as the default; the bubble kept its input, the question failed with a network error in 15 ms and the built-in model answered "Paris." in 0.4 s with the note) Linux X11 ☐
