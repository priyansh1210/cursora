// Types mirror src-tauri/src/chat.rs and capture/mod.rs.
export type Locality = "local" | "lan" | "cloud";
export type ModelInfo = { name: string; provider: string; locality: Locality; contextTokens: number | null };
export const ERROR_KINDS = ["auth", "rate_limit", "model_not_found", "context_too_long", "network", "server", "blocked_local_only", "unknown"] as const;
export type ErrorKind = (typeof ERROR_KINDS)[number];
export type ChatError = { kind: ErrorKind; message: string };
export type TurnStatus = "streaming" | "done" | "cancelled" | "error";
export type Turn = { question: string; answer: string; status: TurnStatus; error: ChatError | null; asked: number; answered: number | null };
export type FileItem = { path: string; name: string; kind: "file" | "folder" | "link"; size: number; modified: number; children: string[]; content: string | null; skipped: string | null };
export type Content = { kind: "text"; text: string; truncated: boolean } | { kind: "files"; items: FileItem[]; omitted: number } | { kind: "none" };
export type Notice = { kind: "elevated" } | { kind: "noSelection"; offerSnip: boolean };
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
};
export type Delta = { id: number; offset: number; text: string };
export type Done = { id: number; status: TurnStatus; error: ChatError | null; dropped: number };

/** Enter sends; Shift+Enter, or Enter while an IME is composing, makes a newline (BUB-4.1). */
export function keyAction(e: { key: string; shiftKey: boolean; isComposing: boolean }): "send" | null {
  return e.key === "Enter" && !e.shiftKey && !e.isComposing ? "send" : null;
}

const ERROR_TEXT: Record<ErrorKind, string> = {
  auth: "The provider rejected the API key. Check it in Models settings.",
  rate_limit: "The provider is limiting requests right now. Wait a moment, then retry.",
  model_not_found: "This model isn't available from the provider. Pick another in Models settings.",
  context_too_long: "That's too much text for this model. Remove the context or use a model with a larger context window.",
  network: "Couldn't reach the provider. Check your connection, or that the local model server is running.",
  server: "The provider had a problem answering. Try again.",
  blocked_local_only: "Local-only mode is on, so this cloud model can't be used. Pick a local model in Models settings.",
  unknown: "Something went wrong",
};

/** Plain-language text for each failure kind; auth and configuration errors also offer Models settings (BUB-4.4). */
export function errorText(kind: ErrorKind, message: string): { text: string; openSettings: boolean } {
  const text = kind === "unknown" ? `${ERROR_TEXT.unknown}: ${message}` : ERROR_TEXT[kind];
  return { text, openSettings: kind === "auth" || kind === "model_not_found" || kind === "blocked_local_only" };
}

/** BUB-7.1 */
export function header(c: { persona: string | null; model: ModelInfo | null }): { persona: string; model: string; badge: string } {
  const m = c.model;
  return { persona: c.persona ?? "Personality off", model: m?.name ?? "No model", badge: m ? badge(m.locality, m.provider) : "" };
}

export function badge(l: Locality, provider: string): string {
  return l === "local" ? "🔒 Local" : l === "lan" ? "🏠 LAN" : `☁ ${provider}`;
}

export function formatSize(bytes: number): string {
  return bytes < 1024 ? `${bytes} B` : bytes < 1024 * 1024 ? `${(bytes / 1024).toFixed(1)} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
