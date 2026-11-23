// TECHNICAL 18.5, 18.6: the dock: Needs you, plan, queue, status line, input.
import { useEffect, useRef, useState, type ReactNode } from "react";
import { errorWords, type Actions, type SessionView } from "../kit";

export const QUEUE_MAX = 32;
const SHOWN = 3;

const svg = { viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeLinecap: "round", strokeLinejoin: "round", "aria-hidden": true } as const;

/** 18.6 "Send": the unsent text is kept per session in `sessionStorage` (a page without it works as before). */
const draftOf = (key?: string): string => { try { return key ? sessionStorage.getItem(key) ?? "" : ""; } catch { return ""; } };
const keepDraft = (key: string | undefined, text: string) => { try { if (key) { if (text) sessionStorage.setItem(key, text); else sessionStorage.removeItem(key); } } catch { /* no storage: the draft is not kept */ } };

export function Composer({ queued, actions, busy = false, stopping = false, session, draftKey, placeholder: given, sendLabel }: {
  queued: number; actions: Actions; busy?: boolean; stopping?: boolean; session?: { model?: string; mode?: string };
  /** Where the unsent text is kept (`sessionStorage`); none keeps nothing. */ draftKey?: string;
  /** Words for a composer that is not a session's (the new-session view). */ placeholder?: string; sendLabel?: string;
}) {
  const [text, setText] = useState(() => draftOf(draftKey));
  useEffect(() => keepDraft(draftKey, text), [draftKey, text]);
  const full = queued >= QUEUE_MAX;
  const [refused, setRefused] = useState<string | null>(null);
  const edited = useRef(0); // counts edits and sends: a refusal restores its text only if none came since
  const off = !!actions.off; // 18.6 "Another tab": the stream is not this tab's
  const send = () => {
    const t = text.trim();
    if (!t || full || off) return;
    const mark = ++edited.current;
    setRefused(null);
    setText("");
    keepDraft(draftKey, ""); // rule 2: out of the kept draft before the send is dispatched, not in the effect after
    const back = (e: unknown) => {
      if (edited.current !== mark) return;
      setText(t);
      setRefused(errorWords(e));
    };
    try { Promise.resolve(actions.say(t)).catch(back); } catch (e) { back(e); }
  };
  const placeholder = given ?? (full ? `The queue is full: ${QUEUE_MAX} messages wait. Remove one, or stop the turn.`
    : busy ? "Add a message: it is queued and sent when this turn ends"
    : "Message the agent  ·  Enter to send, Shift+Enter for a new line");
  const tags = [session?.model, session?.mode].filter((x): x is string => !!x);
  return (
    <form className="wa-composer" onSubmit={(e) => { e.preventDefault(); send(); }}>
      {refused ? <p className="wa-send-err" role="alert">{refused}</p> : null}
      <textarea aria-label="Message" rows={2} value={text} placeholder={placeholder}
        onChange={(e) => { edited.current++; setRefused(null); setText(e.target.value); }}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) { e.preventDefault(); send(); }
          else if (e.key === "Escape" && !off) actions.stop();
        }} />
      <div className="wa-irow">
        {tags.map((t, n) => <span key={n} className="wa-tag">{n > 0 ? <span aria-hidden="true">· </span> : null}{t}</span>)}
        {busy ? (
          <button type="button" className="wa-stop" disabled={off} onClick={() => actions.stop()}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><rect x="5" y="5" width="14" height="14" rx="2" /></svg>
            {stopping ? "Stopping…" : "Stop"}
          </button>
        ) : null}
        <button type="submit" className="wa-send" aria-label={sendLabel ?? (busy ? "Queue message" : "Send message")} disabled={full || off || !text.trim()}>
          <svg width="16" height="16" strokeWidth="2.4" {...svg}><path d="M12 19V5M6 11l6-6 6 6" /></svg>
        </button>
      </div>
    </form>
  );
}

export function Dock(props: { view: SessionView; actions: Actions; onNeedsYou: () => void; needsHidden?: boolean; behind?: boolean; onBehind?: () => void; status: ReactNode;
  session?: { model?: string; mode?: string }; stopping?: boolean; sessionId?: string; replaced?: boolean; onUseHere?: () => void }) {
  const { view } = props;
  const [opened, setOpened] = useState<boolean | null>(null);
  const done = view.plan?.filter((p) => p.status === "completed").length ?? 0;
  const folded = !(opened ?? !(view.openAsks.length > 0 || view.queue.length > 0));
  return (
    <div className="wa-dock">
      {props.behind ? (
        <button type="button" className="wa-chip wa-pill" onClick={props.onBehind}>
          <svg width="14" height="14" strokeWidth="2.2" {...svg}><path d="M12 5v14M6 13l6 6 6-6" /></svg>
          New activity below
        </button>
      ) : null}
      {view.openAsks.length ? (
        // hidden, not removed, while its card is in view and holds focus: the dock keeps its height, so the transcript does not resize under the reader
        <button type="button" className="wa-needs" style={props.needsHidden ? { visibility: "hidden" } : undefined} onClick={props.onNeedsYou}>
          <svg width="14" height="14" strokeWidth="2.2" {...svg}><path d="M12 5v14M6 13l6 6 6-6" /></svg>
          Needs you ({view.openAsks.length})
        </button>
      ) : null}
      {view.plan?.length ? (
        <section className="wa-plan" aria-label="Plan">
          <button type="button" className="wa-plan-head" aria-expanded={!folded} onClick={() => setOpened(folded)}>
            <svg className="wa-chev" width="14" height="14" strokeWidth="2" {...svg}><path d="M9 6l6 6-6 6" /></svg>
            Plan · {done} of {view.plan.length} done
          </button>
          {folded ? null : <ul>{view.plan.map((p, n) => <li key={n} data-status={p.status}>{p.content}</li>)}</ul>}
        </section>
      ) : null}
      {view.queue.length ? (
        <section className="wa-queue" aria-label="Queue">
          <ul>{view.queue.slice(0, SHOWN).map((q) => <li key={q.n}>{q.input}</li>)}</ul>
          {view.queue.length > SHOWN ? <p className="wa-queue-more">+ {view.queue.length - SHOWN} more</p> : null}
        </section>
      ) : null}
      {props.status}
      {props.replaced ? (
        <p className="wa-replaced" role="status">This session is open in another tab. <button type="button" className="wa-chip" onClick={props.onUseHere}>Use here</button></p>
      ) : null}
      <Composer key={props.sessionId} draftKey={props.sessionId === undefined ? undefined : `wa-draft:${props.sessionId}`} queued={view.queue.length} actions={props.actions} session={props.session} stopping={props.stopping}
        busy={view.turn?.state === "running" || view.turn?.state === "waiting"} />
    </div>
  );
}
