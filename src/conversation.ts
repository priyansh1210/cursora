// Types mirror src-tauri/src/chat.rs and capture/mod.rs.
import type { ModelRef } from "./useSettings";
export type Locality = "local" | "lan" | "cloud";
export type ModelInfo = { name: string; provider: string; locality: Locality; contextTokens: number | null };
export const ERROR_KINDS = ["auth", "rate_limit", "billing", "model_not_found", "context_too_long", "network", "server", "blocked_local_only", "unknown"] as const;
export type ErrorKind = (typeof ERROR_KINDS)[number];
export type ChatError = { kind: ErrorKind; message: string };
export type TurnStatus = "streaming" | "done" | "cancelled" | "error";
/** `local`: Cursora's own reply to a command such as /persona, never sent to the model (PER-3.1). */
export type Turn = { question: string; answer: string; status: TurnStatus; error: ChatError | null; asked: number; answered: number | null; local: boolean; note?: string | null };
export type FileItem = { path: string; name: string; kind: "file" | "folder" | "link"; size: number; modified: number; children: string[]; content: string | null; skipped: string | null; image?: boolean };
export type Image = { mime: string; base64: string };
/** `snip`: a screen region; the page only ever gets its thumbnail (VIS-2.6). */
export type Content =
  | { kind: "text"; text: string; truncated: boolean }
  | { kind: "files"; items: FileItem[]; omitted: number }
  | { kind: "snip"; thumbnail: Image; width: number; height: number }
  | { kind: "none" };
export type Notice =
  | { kind: "blocked" }
  | { kind: "elevated" }
  | { kind: "noSelection"; offerSnip: boolean }
  | { kind: "unreadable" }
  | { kind: "needsVision"; offer: { model: ModelRef; label: string } | null } // VIS-1.2
  | { kind: "snipFailed"; reason: string };
export type Conversation = {
  id: number;
  app: { exe: string; name: string; title: string; elevated: boolean };
  content: Content;
  notice: Notice | null;
  model: ModelInfo | null;
  persona: string | null;
  turns: Turn[];
  dropped: number;
  /** Why no model can answer (LLM-9.1). */
  unavailable?: string;
  /** The model this conversation answers with instead of the default: a snip's one-time switch (VIS-1.2). */
  ownModel?: ModelRef;
};
export type Delta = { id: number; offset: number; text: string };
export type Done = { id: number; status: TurnStatus; error: ChatError | null; dropped: number };
/** What ask and regenerate did; "confirm": a cloud request waits for Send (PRI-3.7). */
export type Asked = "sent" | "confirm";
/** "What will be sent" (PRI-3.6). No receiver: Cursora answers it itself. */
export type Preview = { to: ModelInfo | null; text: string; placeholders: string[]; imageWarning: boolean };

/** Enter sends; Shift+Enter, or Enter while an IME is composing, makes a newline (BUB-4.1). */
export function keyAction(e: { key: string; shiftKey: boolean; isComposing: boolean }): "send" | null {
  return e.key === "Enter" && !e.shiftKey && !e.isComposing ? "send" : null;
}

const ERROR_TEXT: Record<ErrorKind, string> = {
  auth: "The provider rejected the API key. Check it in Models settings.",
  rate_limit: "The provider is limiting requests right now. Wait a moment, then retry.",
  billing: "Your account with this provider has no credit left, or its plan doesn't cover this. Check billing on the provider's website, or pick another model in Models settings.",
  model_not_found: "This model isn't available from the provider. Pick another in Models settings.",
  context_too_long: "That's too much text for this model. Remove the context or use a model with a larger context window.",
  network: "Couldn't reach the provider. Check your connection, or that the local model server is running.",
  server: "The provider had a problem answering. Try again.",
  blocked_local_only: "Local-only mode is on, so this cloud model can't be used. Pick a local model in Models settings.",
  unknown: "Something went wrong",
};

/** Plain-language text for each failure kind; auth, billing and configuration errors also offer Models settings (BUB-4.4). */
export function errorText(kind: ErrorKind, message: string): { text: string; openSettings: boolean } {
  const text = kind === "unknown" ? `${ERROR_TEXT.unknown}: ${message}` : ERROR_TEXT[kind];
  return { text, openSettings: kind === "auth" || kind === "billing" || kind === "model_not_found" || kind === "blocked_local_only" };
}

/** BUB-7.1 */
export function header(c: { persona: string | null; model: ModelInfo | null }, localOnly = false): { persona: string; model: string; badge: string } {
  const m = c.model;
  return { persona: c.persona ?? "Personality off", model: m?.name ?? "No model", badge: m ? badge(m.locality, m.provider, localOnly) : "" };
}

/** While local-only mode is on, every model that may answer says so (PRI-1.2). */
export function badge(l: Locality, provider: string, localOnly = false): string {
  if (localOnly && l !== "cloud") return "🔒 Local-only";
  return l === "local" ? "🔒 Local" : l === "lan" ? "🏠 LAN" : `☁ ${provider}`;
}

/** The text cut into plain parts and the given placeholders, so the preview can mark them (PRI-3.6). */
export function marks(text: string, placeholders: string[]): { text: string; mark: boolean }[] {
  if (!placeholders.length) return [{ text, mark: false }];
  const any = new RegExp(`(${placeholders.map((p) => p.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|")})`);
  return text
    .split(any)
    .map((part, i) => ({ text: part, mark: i % 2 === 1 }))
    .filter((p) => p.text);
}

export function formatSize(bytes: number): string {
  return bytes < 1024 ? `${bytes} B` : bytes < 1024 * 1024 ? `${(bytes / 1024).toFixed(1)} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
