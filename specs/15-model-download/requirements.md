# 15: One-click model download (MDL)

## Scope
- **In:** downloading a free model for the built-in engine from Setup and Settings → Models, with one click. There are three sizes; the one that suits this computer is marked. The download shows its progress, can be cancelled and continued later, and is checked before use. The finished file becomes the built-in engine's model, as if chosen with "Choose a .gguf file" (LLM-3.2).
- **Out:**
  - Browsing or searching other models (any `.gguf` can still be chosen by hand)
  - Vision models and their projector files (LLM-3.4 still works by hand)
  - GPU builds of the engine
  - Deleting downloaded models from inside Cursora (they're plain files in the folder shown)
  - Checking free disk space beforehand (a full disk is reported as an error, MDL-1.7)

Idea #3 from the improvement plan ([product direction](../../memory/product-direction.md)), built at the user's request of 2026-10-26. It needs the dated constitution §3 amendment of the same day: a user-started download of a pinned model file.

## Requirements

### MDL-1 Download a model
User story: As someone without a model, I want Cursora to fetch a good one for me, without knowing what a `.gguf` file, a quantization or Hugging Face is.
- MDL-1.1 (manual) THE source list in Setup ("Choose an AI source") and in Settings → Models SHALL offer **Download a free model** first. It SHALL show three choices in plain words, each with its download size: **Small** (fastest), **Medium** (a good balance) and **Large** (best answers, slower). One is marked **Recommended**. It SHALL also show the folder it saves to, with **Change…**, and a **Download** button.
- MDL-1.2 (auto) THE recommended choice SHALL be Small when this computer has less than 8 GB of memory, and Medium otherwise. Large is never recommended, since on a processor without a GPU it is slow for a bubble.
- MDL-1.3 (auto) Each choice's address (on huggingface.co, at a fixed revision), size and SHA-256 SHALL be fixed in the code. A downloaded file whose size or SHA-256 differs SHALL be deleted and not used, with an error saying so.
- MDL-1.4 (manual) WHILE a download runs, a progress bar SHALL show how much is done ("1.2 of 2.5 GB"), with **Cancel**. Only one download runs at a time.
- MDL-1.5 (auto) **Cancel** SHALL keep the part already downloaded, and the next **Download** of the same choice into the same folder SHALL continue from there. If the server sends the whole file instead of the rest, the download SHALL start again from the beginning.
- MDL-1.6 (auto) WHEN the file is complete and checks out, it SHALL be added as the built-in engine source, as with "Choose a .gguf file". A complete file already in the folder SHALL be checked and used without downloading it again.
- MDL-1.7 (auto) A failed download (no network, a server error, a full disk, no data for 60 seconds) SHALL show why in plain words, and keep the part already downloaded.

### MDL-2 Privacy
- MDL-2.1 (auto) THE download SHALL request only the pinned file: no user content, settings or identifiers are sent. It starts only when the user clicks **Download**. Local-only mode doesn't block it, since it carries no user content (constitution §3, amended 2026-10-26).

## Defaults taken while the user was away (2026-10-26)
The user asked for ideas #1 to #5 to be built overnight and their choices listed for review:
1. **The models.** All three are free under the Apache 2.0 license, are instruction models without a separate "thinking" phase, and come in the Q4_K_M size:
   - **Small:** Qwen2.5 1.5B Instruct, 1.1 GB, from the Qwen team's own repository
   - **Medium:** Qwen3 4B Instruct 2507, 2.5 GB, converted by Unsloth (Qwen publishes no file of this kind for it)
   - **Large:** Qwen2.5 7B Instruct, 4.7 GB, converted by bartowski (Qwen's own file comes in two parts)

   Thinking models were left out because Cursora hides thinking text, so the bubble would stay empty for a long time. Llama and Gemma were left out because their licenses ask more of users than Apache 2.0.
2. **Hosted on Hugging Face**, at the models' own repositories, not copied to `cursora-releases`.
3. **The recommendation:** Small under 8 GB of memory, Medium otherwise. The 2026-10-26 backlog table also mapped 16 GB to 7–8B models, but those answer slowly without a GPU.
4. **The default folder** is Cursora's local data folder (`models`), and **Change…** picks another one. The folder isn't remembered: the model's path is saved with the source, as for a chosen file.
5. **No memory figures are shown**; only "Recommended" marks the choice. The user found the old hint's RAM line jargon (LLM-2.4, retired).
6. **The constitution §3 amendment**, dated 2026-10-26, allows this download.

## Open questions
None.
