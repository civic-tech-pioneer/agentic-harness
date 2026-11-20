// TECHNICAL 18.6 Asks, 9.6: Allow once, Allow <reach the kernel names> this session (only with a reach), Deny… with a reason field, Cancel, Esc closes it.
import { useRef, useState } from "react";
import type { Actions, ToolCall } from "../model";

export function AskBar({ call, reach, actions }: { call: ToolCall; reach?: string; actions: Actions }) {
  const [denying, setDenying] = useState(false);
  const [reason, setReason] = useState("");
  const id = call.id;
  const denyBtn = useRef<HTMLButtonElement>(null);
  const closeDeny = () => { setDenying(false); queueMicrotask(() => denyBtn.current?.focus()); };
  return (
    <div className="wa-askbar" role="group" aria-label="Your answer">
      <button type="button" className="wa-primary" data-ask-first onClick={() => actions.answer(id, { outcome: "allow" })}>Allow once</button>
      {reach ? <button type="button" onClick={() => actions.answer(id, { outcome: "allow", for: "session" })}>Allow {reach} this session</button> : null}
      {denying ? (
        <div className="wa-deny">
          <label>Reason
            <input type="text" value={reason} autoFocus
              onChange={(e) => setReason(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Escape") { e.stopPropagation(); closeDeny(); } }} />
          </label>
          <button type="button" onClick={() => { const r = reason.trim(); actions.answer(id, r ? { outcome: "deny", reason: r } : { outcome: "deny" }); }}>Deny</button>
          <button type="button" onClick={closeDeny}>Cancel</button>
        </div>
      ) : (
        <button type="button" ref={denyBtn} onClick={() => setDenying(true)}>Deny…</button>
      )}
    </div>
  );
}
