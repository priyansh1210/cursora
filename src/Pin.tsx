import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useRef, useState } from "react";
import { CopyButton, openLink } from "./Chat";
import { render } from "./markdown";
import { useSettings } from "./useSettings";

/** Mirrors `keep::Kept`. */
type Kept = { question: string; answer: string; app: string; model: string | null };

/** A pinned answer in its own window, "pin-<n>" (SAV-1.1): shown as in the bubble, with Copy. Its title bar holds the question and Close. */
export default function Pin() {
  useSettings(); // follows the theme (SAV-1.4)
  const [note, setNote] = useState<Kept | null>();
  const answerRef = useRef<HTMLDivElement>(null);
  useEffect(() => void invoke<Kept | null>("pinned").then(setNote), []);
  const html = useMemo(() => render(note?.answer ?? ""), [note]);
  if (note === undefined) return null;
  return (
    <main className="pin">
      {note ? (
        <>
          <div ref={answerRef} className="answer" onClick={openLink} dangerouslySetInnerHTML={{ __html: html }} />
          <div className="actions">
            <CopyButton markdown={note.answer} plain={() => answerRef.current?.innerText ?? ""} />
          </div>
        </>
      ) : (
        <p className="muted">This note's answer is gone. Close it and pin the answer again.</p>
      )}
    </main>
  );
}
