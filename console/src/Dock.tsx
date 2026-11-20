// TECHNICAL 18.5, 18.6: the dock: Needs you, plan, queue, status line, input.
import { useState, type ReactNode } from "react";
import type { Actions, SessionView } from "../kit";

export const QUEUE_MAX = 32;
const SHOWN = 3;

export function Composer({ queued, actions }: { queued: number; actions: Actions }) {
  const [text, setText] = useState("");
  const full = queued >= QUEUE_MAX;
  const send = () => {
    const t = text.trim();
    if (!t || full) return;
    actions.say(t);
    setText("");
  };
  return (
    <form className="wa-composer" onSubmit={(e) => { e.preventDefault(); send(); }}>
      <textarea aria-label="Message" rows={2} value={text}
        placeholder={full ? `The queue is full (${QUEUE_MAX} inputs): wait for one to be sent` : "Say what to do next"}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) { e.preventDefault(); send(); }
          else if (e.key === "Escape") actions.stop();
        }} />
      <button type="submit" className="wa-primary" disabled={full || !text.trim()}>Send</button>
    </form>
  );
}

export function Dock(props: { view: SessionView; actions: Actions; onNeedsYou: () => void; status: ReactNode }) {
  const { view } = props;
  const [opened, setOpened] = useState(false);
  const done = view.plan?.filter((p) => p.status === "completed").length ?? 0;
  const folded = (view.openAsks.length > 0 || view.queue.length > 0) && !opened;
  return (
    <div className="wa-dock">
      {view.openAsks.length ? (
        <button type="button" className="wa-needs" onClick={props.onNeedsYou}>Needs you ({view.openAsks.length})</button>
      ) : null}
      {view.plan?.length ? (
        <section className="wa-plan" aria-label="Plan">
          {folded ? (
            <button type="button" className="wa-plan-head" aria-expanded={false} onClick={() => setOpened(true)}>Plan · {done} of {view.plan.length} done</button>
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
      <Composer queued={view.queue.length} actions={props.actions} />
    </div>
  );
}
