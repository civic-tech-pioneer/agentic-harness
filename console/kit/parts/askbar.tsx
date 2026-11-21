// TECHNICAL 18.6 Asks, 9.6: the question, Deny… (with a reason row, Cancel, Esc closes it), Allow <reach the kernel names> this session (only with a reach), Allow once.
import { useLayoutEffect, useRef, useState, type MutableRefObject, type RefObject } from "react";
import { MEASURE, type Actions, type ToolCall } from "../model";

const QUESTION: Record<string, string> = { edit: "Allow this edit?", execute: "Allow this command?" };

/** 18.6 Asks: once an answer is sent the bar gives way to one line until the kernel's decision entry arrives (9.4 rule 4: a pending answer counts as decided). */
export function AskBar({ call, askId, reach, actions, sent = false, judge = false, refused = false, lineRef, focusIn }:
  { call: ToolCall; askId: string; reach?: string; actions: Actions; sent?: boolean; judge?: boolean; refused?: boolean; lineRef?: MutableRefObject<HTMLElement | null>; focusIn?: RefObject<boolean> }) {
  const [denying, setDenying] = useState(false);
  const [reason, setReason] = useState("");
  const id = askId;
  const denyBtn = useRef<HTMLButtonElement>(null);
  const closeDeny = () => { setDenying(false); queueMicrotask(() => denyBtn.current?.focus()); };
  const reasonIn = useRef<HTMLInputElement>(null);
  const note = useRef<HTMLParagraphElement>(null);
  const ownLine = useRef<HTMLParagraphElement | null>(null);
  const line = ownLine;
  // whichever of the bar or the line is mounted marks the card for the refusal handler
  const mark = (n: HTMLElement | null) => { if (lineRef) lineRef.current = n; };
  // the button that was pressed is gone: focus stays in the card, on the line, never on another card's button
  useLayoutEffect(() => { if (sent) line.current?.focus({ preventScroll: true }); }, [sent]);
  // the line that held focus is gone: focus goes to the refused line (never a button), only when it was in this card when the refusal arrived (read then, not here);
  // focus anywhere else (the composer, another card) stays; the page reads what holds focus again
  useLayoutEffect(() => {
    const n = note.current;
    if (!refused || !n) return;
    if (focusIn?.current) n.focus({ preventScroll: true });
    n.dispatchEvent(new Event(MEASURE, { bubbles: true }));
  }, [refused]);
  if (sent) {
    return <p key="line" ref={(n) => { line.current = n; mark(n); }} className="wa-answered askbar" role="status" tabIndex={-1}>{judge ? "Checking with your judge" : "Answered, waiting"}</p>;
  }
  return (
    <>
      {refused ? <p key="refused" ref={note} className="wa-refused" role="alert" tabIndex={-1}>The answer was not taken. Try again.</p> : null}
      <div ref={mark} className="wa-askbar askbar" role="group" aria-label="Your answer">
        <span className="askq">{QUESTION[call.kind] ?? "Allow this call?"}</span>
        <button type="button" ref={denyBtn} className="btn quiet danger" onClick={() => { if (denying) closeDeny(); else { setDenying(true); queueMicrotask(() => reasonIn.current?.focus({ preventScroll: true })); } }}>Deny…</button>
        {reach ? <button type="button" className="btn" onClick={() => actions.answer(id, { outcome: "allow", for: "session" })}>Allow {reach} this session</button> : null}
        <button type="button" className="btn primary wa-primary" data-ask-first onClick={() => actions.answer(id, { outcome: "allow" })}>Allow once</button>
      </div>
      {denying ? (
        <form className="wa-deny denyrow" onSubmit={(e) => { e.preventDefault(); const r = reason.trim(); actions.answer(id, r ? { outcome: "deny", reason: r } : { outcome: "deny" }); }}>
          <label className="tag" htmlFor={`deny-${id}`}>Tell the agent why</label>
          <input id={`deny-${id}`} ref={reasonIn} type="text" value={reason}
            onChange={(e) => setReason(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Escape") { e.stopPropagation(); closeDeny(); } }} />
          <button type="button" className="btn quiet" onClick={closeDeny}>Cancel</button>
          <button type="submit" className="btn danger">Deny</button>
        </form>
      ) : null}
    </>
  );
}
