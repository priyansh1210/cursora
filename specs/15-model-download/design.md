# 15: One-click model download (design)

Status: **Approved** (2026-10-26, under the user's go-ahead to build ideas #1 to #5 while away, choices listed in requirements.md for review). Implements [requirements.md](requirements.md). Rust owns the download: a pinned table, a resumable fetch that checks the SHA-256, and progress events. The page shows the choices and, once the file is ready, adds it as the built-in engine source the same way "Choose a .gguf file" does.

## Overview
```
Setup / Models → AddProvider, source "Download a free model"
   model_choices() → the three choices, the recommended one (MDL-1.2), the default folder
   [Change…] → model_folder(): a folder picker
   [Download] → model_download(id, folder):
       <folder>/<file> complete? → check SHA-256 → path (MDL-1.6)
       else <file>.part: GET with Range from its length (MDL-1.5) → append, emit "model-progress"
            → cancel / error: keep the .part (MDL-1.5, 1.7)
            → done: size and SHA-256 match? rename to <file> → path; else delete (MDL-1.3)
   → add(engineProvider(path)) (LLM-3.2)
[Cancel] → model_download_cancel()
```

## Components
| File | Change |
|---|---|
| `src-tauri/src/llm/download.rs` (new) | `MODELS`, `recommended()`, `total_ram()` (restored from LLM-2.4), `fetch()`, and the commands `model_choices`, `model_folder`, `model_download`, `model_download_cancel`. |
| `src-tauri/Cargo.toml` | `sha2 = "0.10"`, already compiled for Tauri. |
| `src-tauri/src/lib.rs` | Registers the commands and the `Download` state (two flags: running, cancel). |
| `src/Models.tsx` | `AddProvider` gains the source "Download a free model", first in the list, and a `DownloadModel` part. Setup uses `AddProvider` too. |
| `specs/00-constitution.md` | The 2026-10-26 §3 amendment. |

## Key mechanics
- **The table** (`MODELS`): `id` (`small`, `medium`, `large`), the plain label and note, the file name, the URL `https://huggingface.co/<repo>/resolve/<commit>/<file>`, the size in bytes and the SHA-256. All of these come from the Hugging Face API on 2026-10-26.
- **`recommended(total)`:** `small` under 8 GB, rounded to whole GB like LLM-2.4 did (a 16 GB computer reports a little less); otherwise `medium`. If the total can't be read, `medium`.
- **`fetch(url, size, sha256, path, cancel, progress)`**, async:
  1. If `path` exists with the pinned size, it is checked and returned. If its size is wrong, it's deleted.
  2. If `path.part` exists, the request asks for `Range: bytes=<len>-`. A `206` reply is appended to it; a `200` reply starts the file again (MDL-1.5). Any other status is an error naming it.
  3. Chunks are written as they arrive. Waiting for the next one races the cancel flag, so Cancel works at once even when no data comes. `progress(done, total)` is called at most 4 times a second. 60 seconds without data is an error (MDL-1.7).
  4. At the end, a wrong size or SHA-256 deletes the `.part` (MDL-1.3); otherwise it's renamed to `path`.
- **The client** follows redirects (Hugging Face sends files from its CDN) and uses the system proxy. It sends Cursora's user agent and nothing else (MDL-2.1). It doesn't go through `net::guard`, which exists to stop user content in local-only mode.
- **One at a time:** `model_download` fails with "A model is already downloading." while the running flag is set.
- **Progress** is the event `model-progress { done, total }`. The page shows it only while its own download runs, so a late event can't bring the bar back. A window opened mid-download that clicks **Download** gets "A model is already downloading."
- **Errors** in plain words: "Couldn't reach the download server. Check the internet connection and try again." for a connection failure; the OS message for a disk error (e.g. a full disk); "The download server answered <status>."; "The download stopped: no data for a minute."; "The downloaded file didn't match, so it was deleted. Try again."

  `shortcut:` no free-space check beforehand. A full disk is reported when it happens, and the part is kept. Add a check if users report half-finished downloads.

## Decisions
1. **Rust downloads, not the page**: the file is gigabytes, needs the disk and a hash, and must survive the page being closed.
2. **The `.part` file is the resume state.** There's no download record in the settings; a file that's there is enough.
3. **The SHA-256 is checked once, at the end**, by reading the file. That's simpler than hashing a resumed stream, and costs a few seconds on a 2.5 GB file.

## Test strategy
| Criterion | How |
|---|---|
| MDL-1.2 | `recommended()` at 4, 7.8, 8, 15.7 and 64 GB, and unknown. |
| MDL-1.3 | Every entry has an https huggingface.co URL ending in its file name, a 64-digit SHA-256 and a size. A served file with the wrong bytes is deleted and fails. |
| MDL-1.5 | A local test server that honours `Range`: cancel midway, then fetch again; only the rest is requested, and the file checks out. A server that ignores `Range` gets a fresh, correct file. |
| MDL-1.6 | A complete, correct file in the folder is returned without a request. |
| MDL-1.7 | A `404` is an error, and the `.part` stays. |
| MDL-2.1 | The test server sees one GET for the file, with a `Range` header and no body. |
| Manual | MDL-1.1, 1.4: a real download of Small on Windows, cancelled and continued, then a question answered. |
