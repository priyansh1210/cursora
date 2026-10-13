# 06: Skills (evaluation sheet)

Each built-in skill on a sample input, judged as sensible or not. This is a reference, not a gate: SKL-3.2 is retired, because how good an answer is depends on the model the user picks.

## Local: qwen2.5-0.5b-instruct (Q4_K_M), built-in engine, personality off (2026-10-13, Windows)
A 0.5B model is far below what the skills are written for; this run mainly shows the plumbing works. The test text was selected in a PowerShell window, so the source app is "powershell", which the model sometimes takes for the language of the text.

| Skill | Sample | Answer (shortened) | Sensible? |
|---|---|---|---|
| `/summary` | The Eiffel Tower paragraph (4 facts) | not captured by the test script | — |
| `/explain` | "Photosynthesis converts light energy into chemical energy stored in glucose." | Calls it "an example of a PowerShell script … about photosynthesis" | No |
| `/simplify` | "Notwithstanding the aforementioned considerations, the committee has resolved to defer its determination…" | Repeats the untrusted-content rule instead | No |
| `/define` | "ephemeral" | Echoes its instructions, then "something that is temporary or not permanent" | Partly |
| `/translate French` | "Good morning, how are you today?" | "Bonjour, comment ça va aujourd'hui ?" plus a stray note | Yes |
| `/fix` | "Their going too the store tomorow, and they has forgot there list." | Fixes "to", "tomorrow", "forgotten", "their list"; misses "Their going" and "they has"; wraps it in tags | Partly |
| `/rewrite formal` | "hey, can u send me the report asap? thx" | "Hey, can you send me the report ASAP? Thank you!" | No (not formal) |
| `/code` | `for (let i = 0; i < arr.length; i++) { sum += arr[i]; }` | "Sums the elements of an array", but calls it PowerShell | Partly |
| `/bugs` | `avg()` with `i <= a.length` | Repeats the code; no bug named | No |
