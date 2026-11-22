// TECHNICAL 2.3, 18.6 "Send": what the composer's alert says for a failed `say` or `start`, one sentence for each error case of 2.3.
type Err = { kind?: unknown; payload?: unknown; refusals?: unknown; message?: unknown };

const reasons = (e: Err): string[] => {
  const list = Array.isArray(e.refusals) ? e.refusals : Array.isArray(e.payload) ? e.payload : [];
  return list.flatMap((r) => (r && typeof r === "object" && typeof (r as { reason?: unknown }).reason === "string" ? [(r as { reason: string }).reason] : []));
};

/** The alert for a failed call: a sentence for each 2.3 error the kernel names (`kind` is the case's name), never the JSON. An error with no case (a plain `Error`) says its own message. */
export function errorWords(error: unknown): string {
  const e = (error && typeof error === "object" ? error : {}) as Err;
  const text = typeof e.message === "string" ? e.message : "";
  const arg = typeof e.payload === "string" ? e.payload : "";
  switch (e.kind) {
    case "busy": return "Claude is busy with another session; try again in a moment.";
    case "queue-full": return "The queue is full. Wait for the turn to end, or remove a queued message.";
    case "quota": return "This console has used up its quota of sessions or messages. Delete a session, or try again later.";
    case "paused": return "This session is paused. Try again when it resumes.";
    case "backend": return `The kernel could not do that: ${arg || "the provider failed"}`;
    case "refused": { const r = reasons(e); return r.length ? `The kernel refused that: ${r.join("; ")}` : "The kernel refused that."; }
    case "too-large": return typeof e.payload === "number" ? `That message is too large; the limit is ${e.payload} bytes.` : "That message is too large.";
    case "not-owner": case "unknown-session": return "This console has no such session.";
    case "unknown-input": case "unknown-allow": return "That item no longer exists.";
    case "behind": return "The page is behind the session; reload it and try again.";
    case "host": case "unreadable": return "The page could not reach the kernel. Try again.";
  }
  return text || "Could not send that message.";
}
