// TECHNICAL 18.5, 18.6: the dock: Needs you, plan, queue, status line, input.
import { useState, type ReactNode } from "react";
import type { Actions, SessionView } from "../kit";

export const QUEUE_MAX = 32;
const SHOWN = 3;

const svg = { viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeLinecap: "round", strokeLinejoin: "round", "aria-hidden": true } as const;

export function Composer({ queued, actions, busy = false, stopping = false, session }: {
  queued: number; actions: Actions; busy?: boolean; stopping?: boolean; session?: { model?: string; mode?: string };
}) {
  const [text, setText] = useState("");
  const full = queued >= QUEUE_MAX;
  const send = () => {
    const t = text.trim();
    if (!t || full) return;
    actions.say(t);
    setText("");
  };
  const placeholder = full ? `The queue is full: ${QUEUE_MAX} messages wait. Remove one, or stop the turn.`
    : busy ? "Add a message: it is queued and sent when this turn ends"
    : "Message the agent  ·  Enter to send, Shift+Enter for a new line";
  const tags = [session?.model, session?.mode].filter((x): x is string => !!x);
  return (
    <form className="wa-composer" onSubmit={(e) => { e.preventDefault(); send(); }}>
      <textarea aria-label="Message" rows={2} value={text} placeholder={placeholder}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) { e.preventDefault(); send(); }
          else if (e.key === "Escape") actions.stop();
        }} />
      <div className="wa-irow">
        {tags.map((t, n) => <span key={n} className="wa-tag">{n > 0 ? <span aria-hidden="true">· </span> : null}{t}</span>)}
        {busy ? (
          <button type="button" className="wa-stop" onClick={() => actions.stop()}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><rect x="5" y="5" width="14" height="14" rx="2" /></svg>
            {stopping ? "Stopping…" : "Stop"}
          </button>
        ) : null}
        <button type="submit" className="wa-send" aria-label={busy ? "Queue message" : "Send message"} disabled={full || !text.trim()}>
          <svg width="16" height="16" strokeWidth="2.4" {...svg}><path d="M12 19V5M6 11l6-6 6 6" /></svg>
        </button>
      </div>
    </form>
  );
}

export function Dock(props: { view: SessionView; actions: Actions; onNeedsYou: () => void; status: ReactNode;
  session?: { model?: string; mode?: string }; stopping?: boolean }) {
  const { view } = props;
  const [opened, setOpened] = useState(false);
  const done = view.plan?.filter((p) => p.status === "completed").length ?? 0;
  const folded = (view.openAsks.length > 0 || view.queue.length > 0) && !opened;
  return (
    <div className="wa-dock">
      {view.openAsks.length ? (
        <button type="button" className="wa-needs" onClick={props.onNeedsYou}>
          <svg width="14" height="14" strokeWidth="2.2" {...svg}><path d="M12 5v14M6 13l6 6 6-6" /></svg>
          Needs you ({view.openAsks.length})
        </button>
      ) : null}
      {view.plan?.length ? (
        <section className="wa-plan" aria-label="Plan">
          {folded ? (
            <button type="button" className="wa-plan-head" aria-expanded={false} onClick={() => setOpened(true)}>
              <svg className="wa-chev" width="14" height="14" strokeWidth="2" {...svg}><path d="M9 6l6 6-6 6" /></svg>
              Plan · {done} of {view.plan.length} done
            </button>
          ) : (
            <p className="wa-plan-head">{done} of {view.plan.length} done</p>
          )}
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
      <Composer queued={view.queue.length} actions={props.actions} session={props.session} stopping={props.stopping}
        busy={view.turn?.state === "running" || view.turn?.state === "waiting"} />
    </div>
  );
}
