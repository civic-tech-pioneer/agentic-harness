// TECHNICAL 18.6 Asks: Allow once, Allow <reach> this session, Deny… with a reason field, Cancel, Esc closes it.
import { useState } from "react";
import type { Actions, ToolCall } from "../model";

function dir(p: string): string {
  const k = p.lastIndexOf("/");
  return k < 0 ? "." : k === 0 ? "/" : p.slice(0, k);
}

/** What a session allow would cover, from the ask's input and its call. */
export function reachOf(call: ToolCall, input: unknown): string {
  const o = (input && typeof input === "object" ? input : {}) as Record<string, unknown>;
  const path = [o.file_path, o.path, call.locations[0]?.path].find((v) => typeof v === "string") as string | undefined;
  switch (call.kind) {
    case "edit": case "delete": case "move": return path ? `edits in ${dir(path)}` : "edits";
    case "read": return path ? `reads in ${dir(path)}` : "reads";
    case "execute": {
      const c = typeof o.command === "string" ? o.command.trim().split(/\s+/)[0] : "";
      return c ? `${c} commands` : "commands";
    }
    default: return call.title;
  }
}

export function AskBar({ call, input, actions }: { call: ToolCall; input: unknown; actions: Actions }) {
  const [denying, setDenying] = useState(false);
  const [reason, setReason] = useState("");
  const id = call.id;
  return (
    <div className="wa-askbar" role="group" aria-label="Your answer">
      <button type="button" className="wa-primary" data-ask-first onClick={() => actions.answer(id, { outcome: "allow" })}>Allow once</button>
      <button type="button" onClick={() => actions.answer(id, { outcome: "allow", for: "session" })}>Allow {reachOf(call, input)} this session</button>
      {denying ? (
        <div className="wa-deny">
          <label>Reason
            <input type="text" value={reason} autoFocus
              onChange={(e) => setReason(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Escape") { e.stopPropagation(); setDenying(false); } }} />
          </label>
          <button type="button" onClick={() => { const r = reason.trim(); actions.answer(id, r ? { outcome: "deny", reason: r } : { outcome: "deny" }); }}>Deny</button>
          <button type="button" onClick={() => setDenying(false)}>Cancel</button>
        </div>
      ) : (
        <button type="button" onClick={() => setDenying(true)}>Deny…</button>
      )}
    </div>
  );
}
