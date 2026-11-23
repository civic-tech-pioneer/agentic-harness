// TECHNICAL 18.5, 18.6: the dock: Needs you, plan, queue, status line, input.
import { useEffect, useRef, useState, type MutableRefObject, type ReactNode } from "react";
import { errorWords, type Actions, type SessionView } from "../kit";

export const QUEUE_MAX = 32;
const SHOWN = 3;
const UNDO_MS = 6000;

const svg = { viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeLinecap: "round", strokeLinejoin: "round", "aria-hidden": true } as const;

/** 18.6 "Send": the unsent text is kept per session in `sessionStorage` (a page without it works as before). */
const draftOf = (key?: string): string => { try { return key ? sessionStorage.getItem(key) ?? "" : ""; } catch { return ""; } };
const keepDraft = (key: string | undefined, text: string) => { try { if (key) { if (text) sessionStorage.setItem(key, text); else sessionStorage.removeItem(key); } } catch { /* no storage: the draft is not kept */ } };

/** The composer's text, held for the queue's actions: `take` puts `t` in the composer, and `then` runs once a send of it (as she edited it) is accepted. */
type Ctl = { text(): string; take(t: string, then: () => void): void };

export function Composer({ queued, actions, busy = false, stopping = false, session, draftKey, placeholder: given, sendLabel, ctl }: {
  queued: number; actions: Actions; busy?: boolean; stopping?: boolean; session?: { model?: string; mode?: string };
  /** Where the unsent text is kept (`sessionStorage`); none keeps nothing. */ draftKey?: string;
  /** Filled with a hold on the text (the queue's Edit reads and sets it). */ ctl?: MutableRefObject<Ctl | null>;
  /** Words for a composer that is not a session's (the new-session view). */ placeholder?: string; sendLabel?: string;
}) {
  const [text, setText] = useState(() => draftOf(draftKey));
  useEffect(() => keepDraft(draftKey, text), [draftKey, text]);
  // 18.6 "Queue": a `queue-full` refusal (the 2 MiB limit) keeps Send off until the queue's length changes or the turn ends
  const [fullAt, setFullAt] = useState<number | null>(null);
  const count = useRef(queued); count.current = queued;
  useEffect(() => { setFullAt((f) => (f === null || f === queued ? f : null)); }, [queued]);
  useEffect(() => { if (!busy) setFullAt(null); }, [busy]);
  const full = queued >= QUEUE_MAX || fullAt !== null;
  const [refused, setRefused] = useState<string | null>(null);
  const refuse = (e: unknown) => { setRefused(errorWords(e)); if ((e as { kind?: unknown } | null)?.kind === "queue-full") setFullAt(count.current); };
  const edited = useRef(0); // counts edits and sends: a refusal restores its text only if none came since
  const then = useRef<(() => void) | null>(null); // 18.6 "Queue": an Edit's queued input, taken out once the composer's send is accepted
  if (ctl) ctl.current = { text: () => text, take: (t, after) => { edited.current++; setRefused(null); setText(t); then.current = after; } };
  const off = !!actions.off; // 18.6 "Another tab": the stream is not this tab's
  const send = () => {
    const t = text.trim();
    if (!t || full || off) return;
    const mark = ++edited.current;
    const after = then.current;
    then.current = null;
    setRefused(null);
    setText("");
    keepDraft(draftKey, ""); // rule 2: out of the kept draft before the send is dispatched, not in the effect after
    const back = (e: unknown) => {
      // typing since wins the composer (18.6 "Send")
      if (edited.current === mark) { setText(t); then.current = after; refuse(e); }
      else if ((e as { kind?: unknown } | null)?.kind === "queue-full") setFullAt(count.current); // her newer draft stays; Send is off and says why
    };
    try { Promise.resolve(actions.say(t)).then(() => after?.(), back); } catch (e) { back(e); }
  };
  const why = queued >= QUEUE_MAX ? `The queue is full: ${QUEUE_MAX} messages wait. Remove one, or stop the turn.`
    : full ? "The queue is full: the messages that wait hold all the text it takes. Remove one, or stop the turn." : null;
  const placeholder = given ?? why ?? (busy ? "Add a message: it is queued and sent when this turn ends"
    : "Message the agent  ·  Enter to send, Shift+Enter for a new line");
  const alert = refused ?? (why && given === undefined && text.trim() ? why : null); // the placeholder is hidden under text: Send off says why here
  const tags = [session?.model, session?.mode].filter((x): x is string => !!x);
  return (
    <form className="wa-composer" onSubmit={(e) => { e.preventDefault(); send(); }}>
      {alert ? <p className="wa-send-err" role="alert">{alert}</p> : null}
      <textarea aria-label="Message" rows={2} value={text} placeholder={placeholder}
        onChange={(e) => { edited.current++; setRefused(null); setText(e.target.value); if (!e.target.value.trim()) then.current = null; }}
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
  const { actions } = props;
  const off = !!actions.off;
  const ctl = useRef<Ctl | null>(null);
  // the line under the queue; `undo` is the input its Undo puts back (Remove's, for 6 s)
  type Note = { text: string; undo?: string };
  const [note, setNote] = useState<Note | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>();
  useEffect(() => () => clearTimeout(timer.current), []);
  const say = (text: string, undo?: string) => {
    clearTimeout(timer.current);
    setNote({ text, undo });
    if (undo !== undefined) timer.current = setTimeout(() => setNote(null), UNDO_MS);
  };
  const clearNote = () => { clearTimeout(timer.current); setNote(null); };
  const fail = (e: unknown) => say((e as { kind?: unknown } | null)?.kind === "unknown-input" ? "That message was already sent" : errorWords(e));
  // 18.6 "Queue" (the deputy's rule, S36 r2): the queue keeps an item until its transfer is accepted, so a refusal changes
  // nothing and leaves nothing to restore. The dock is one session's (App keys it by the session); its calls act on that session alone.
  /** Takes input `q` out of the queue; a refusal shows as a line. */
  const unqueue = async (q: { n: number }) => {
    try { await actions.unqueue!(q.n); return true; } catch (e) { fail(e); return false; }
  };
  const edit = (q: { n: number; input: string }) => {
    if (ctl.current?.text().trim()) return say("Finish or clear your message first");
    clearNote();
    ctl.current?.take(q.input, () => void unqueue(q)); // the queued input goes once its edited send is accepted
  };
  const remove = async (q: { n: number; input: string }) => { if (await unqueue(q)) say("Removed.", q.input); };
  const undo = async (input: string) => {
    try { await actions.say(input); clearNote(); } catch (e) { say(errorWords(e), input); } // refused: the line and its Undo stay
  };
  const stopAndSend = async (q: { n: number; input: string }) => {
    try { await actions.interrupt!(q.input); } catch (e) { fail(e); return; } // refused: the input is still queued
    clearNote();
    await unqueue(q); // the replacement runs first (2.3 `interrupt`), so the queued copy is taken out before a turn could take it
  };
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
          <ul>{view.queue.slice(0, SHOWN).map((q, k) => (
            <li key={q.n}>
              <span className="wa-queue-text">{q.input}</span>
              <span className="wa-queue-ctl">
                <button type="button" className="wa-chip" aria-label={`Edit queued message ${k + 1}`} disabled={off} onClick={() => edit(q)}>Edit</button>
                <button type="button" className="wa-chip" aria-label={`Remove queued message ${k + 1}`} disabled={off} onClick={() => remove(q)}>Remove</button>
                <button type="button" className="wa-chip" aria-label={`Stop and send queued message ${k + 1}`} disabled={off} onClick={() => stopAndSend(q)}>Stop and send</button>
              </span>
            </li>
          ))}</ul>
          {view.queue.length > SHOWN ? <p className="wa-queue-more">+ {view.queue.length - SHOWN} more</p> : null}
        </section>
      ) : null}
      {note ? (
        <p className="wa-queue-note" role="status">{note.text}{note.undo !== undefined ? <> <button type="button" className="wa-chip" disabled={off} onClick={() => void undo(note.undo!)}>Undo</button></> : null}</p>
      ) : null}
      {props.status}
      {props.replaced ? (
        <p className="wa-replaced" role="status">This session is open in another tab. <button type="button" className="wa-chip" onClick={props.onUseHere}>Use here</button></p>
      ) : null}
      <Composer key={props.sessionId} ctl={ctl} draftKey={props.sessionId === undefined ? undefined : `wa-draft:${props.sessionId}`} queued={view.queue.length} actions={props.actions} session={props.session} stopping={props.stopping}
        busy={view.turn?.state === "running" || view.turn?.state === "waiting"} />
    </div>
  );
}
