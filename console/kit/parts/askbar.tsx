// TECHNICAL 18.6 Asks, 9.6: the question, Deny… (with a reason row, Cancel, Esc closes it), Allow <reach the kernel names> this session (only with a reach), Allow once.
import { useRef, useState } from "react";
import type { Actions, ToolCall } from "../model";

const QUESTION: Record<string, string> = { edit: "Allow this edit?", execute: "Allow this command?" };

export function AskBar({ call, reach, actions }: { call: ToolCall; reach?: string; actions: Actions }) {
  const [denying, setDenying] = useState(false);
  const [reason, setReason] = useState("");
  const id = call.id;
  const denyBtn = useRef<HTMLButtonElement>(null);
  const closeDeny = () => { setDenying(false); queueMicrotask(() => denyBtn.current?.focus()); };
  return (
    <>
      <div className="wa-askbar askbar" role="group" aria-label="Your answer">
        <span className="askq">{QUESTION[call.kind] ?? "Allow this call?"}</span>
        <button type="button" ref={denyBtn} className="btn quiet danger" onClick={() => (denying ? closeDeny() : setDenying(true))}>Deny…</button>
        {reach ? <button type="button" className="btn" onClick={() => actions.answer(id, { outcome: "allow", for: "session" })}>Allow {reach} this session</button> : null}
        <button type="button" className="btn primary wa-primary" data-ask-first onClick={() => actions.answer(id, { outcome: "allow" })}>Allow once</button>
      </div>
      {denying ? (
        <form className="wa-deny denyrow" onSubmit={(e) => { e.preventDefault(); const r = reason.trim(); actions.answer(id, r ? { outcome: "deny", reason: r } : { outcome: "deny" }); }}>
          <label className="tag" htmlFor={`deny-${id}`}>Tell the agent why</label>
          <input id={`deny-${id}`} type="text" value={reason} autoFocus
            onChange={(e) => setReason(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Escape") { e.stopPropagation(); closeDeny(); } }} />
          <button type="button" className="btn quiet" onClick={closeDeny}>Cancel</button>
          <button type="submit" className="btn danger">Deny</button>
        </form>
      ) : null}
    </>
  );
}
