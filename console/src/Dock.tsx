// TECHNICAL 18.5, 18.6: the dock: Needs you, plan, queue, status line, input.
import { useEffect, useId, useRef, useState, type MutableRefObject, type ReactNode } from "react";
import { epochOf, errorWords, isActiveTurn, isFolderConsent, SESSION_IN_ANOTHER_TAB, type Actions, type SessionView } from "../kit";
import { drop, heldOf, hold, refused as refuseTaken, send as claim, take, undoShown, useTaken, type Taken } from "./taken";

export const QUEUE_MAX = 32;
const SHOWN = 3;

const svg = { viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeLinecap: "round", strokeLinejoin: "round", "aria-hidden": true } as const;

/** 18.6 "Send": the unsent text is kept per session in `sessionStorage` (a page without it works as before). */
const draftOf = (key?: string): string => { try { return key ? sessionStorage.getItem(key) ?? "" : ""; } catch { return ""; } };
const keepDraft = (key: string | undefined, text: string) => { try { if (key) { if (text) sessionStorage.setItem(key, text); else sessionStorage.removeItem(key); } } catch { /* no storage: the draft is not kept */ } };

/** The composer's text, held for the queue's actions: `put` copies `t` into an empty composer (with `error`'s line, as a refused
 * send does) and says whether it did; a composer with text keeps it. `from` is the taken item it copies: while the composer
 * holds that text it owns the item (taken.ts `hold`, kept beside the draft across a reload), its row shows no Undo or retry,
 * and sending the composer, edited or not, is the item's send, under the item's one in-flight guard; on acceptance the item
 * and the link leave together, and clearing the composer gives the item back to its row (S36 r5 F5.1, r6 F6.1). */
type Ctl = { text(): string; put(t: string, error?: unknown, from?: string): boolean };

export function Composer({ queued, actions, busy = false, stopping = false, session, draftKey, placeholder: given, sendLabel, blocked, ctl, epoch, taken, offReasonId = actions.offReasonId }: {
  queued: number; /** The session's capacity epoch (taken.ts); none: the queue's length and the turn stand for it. */ epoch?: () => number; actions: Actions; busy?: boolean; stopping?: boolean; session?: { model?: string; mode?: string };
  offReasonId?: string;
  /** Where the unsent text is kept (`sessionStorage`); none keeps nothing. */ draftKey?: string;
  /** The session whose taken items (taken.ts) the composer can hold; none: it holds none. */ taken?: string;
  /** Filled with a hold on the text (the queue's Edit reads and sets it). */ ctl?: MutableRefObject<Ctl | null>;
  /** Words for a composer that is not a session's (the new-session view). */ placeholder?: string; sendLabel?: string;
  /** Why Start is off (the new-session view's refusals); none: not blocked. */ blocked?: string;
}) {
  const [text, setText_] = useState(() => draftOf(draftKey));
  const [emptyAttempt, setEmptyAttempt] = useState(false);
  const now = useRef(text); // the text as of the last change, for calls that land in one render batch (S36 r5 F5.1)
  const setText = (t: string) => { now.current = t; setText_(t); setEmptyAttempt(false); };
  /** The taken item the composer's text was copied from, while it holds it (S36 r6 F6.1). */
  const held = () => (taken === undefined ? undefined : heldOf(taken));
  const holdIt = (id?: string) => { if (taken !== undefined) hold(taken, id); };
  useEffect(() => keepDraft(draftKey, text), [draftKey, text]);
  useEffect(() => { if (!now.current.trim()) holdIt(undefined); }, []); // eslint-disable-line react-hooks/exhaustive-deps -- an empty kept draft holds nothing
  // 18.6 "Queue": a `queue-full` refusal (the 2 MiB limit) keeps Send off until the queue's length changes or the turn ends;
  // a refusal applies only in the capacity epoch its `say` was sent in: after any queue or turn update since, it describes a
  // queue that is gone, so it changes nothing, even if the same length and turn state come back (S36 r3, r4 F4.2). The epoch
  // is the kernel client's, bumped on receipt (kit/kernel.ts), and the lock holds only in the epoch it was set in (r5 F5.4).
  const [fullAt, setFullAt] = useState<{ queued: number; epoch?: number } | null>(null);
  const count = useRef(queued); count.current = queued;
  const running = useRef(busy); running.current = busy;
  useEffect(() => { setFullAt((f) => (f === null || f.queued === queued ? f : null)); }, [queued]);
  useEffect(() => { if (!busy) setFullAt(null); }, [busy]);
  const full = queued >= QUEUE_MAX || (fullAt !== null && fullAt.epoch === epoch?.());
  const [refused, setRefused] = useState<string | null>(null);
  /** The capacity a send saw: a `queue-full` refusal applies only while it still holds. */
  const epochNow = () => epoch?.();
  const capacity = () => ({ queued: count.current, busy: running.current, epoch: epochNow() });
  const full_ = (e: unknown, at: ReturnType<typeof capacity>) => {
    if ((e as { kind?: unknown } | null)?.kind === "queue-full" && epochNow() === at.epoch && running.current === at.busy && count.current === at.queued) setFullAt({ queued: at.queued, epoch: at.epoch });
  };
  const refuse = (e: unknown, at = capacity()) => { setRefused(errorWords(e)); full_(e, at); };
  const edited = useRef(0); // counts edits and sends: a refusal restores its text only if none came since
  if (ctl) ctl.current = {
    text: () => text,
    put: (t, e, id) => { if (now.current.trim()) return false; edited.current++; setText(t); holdIt(id); setRefused(null); if (e !== undefined) full_(e, capacity()); return true; },
  };
  const off = !!actions.off; // 18.6 "Another tab": the stream is not this tab's
  const send = () => {
    if (full || off || blocked) return;
    const t = now.current.trim();
    if (!t) { setEmptyAttempt(true); return; }
    const of = held();
    if (of !== undefined && !claim(taken!, of)) return; // the item's one guard: a call of it is in flight
    const mark = ++edited.current;
    const at = capacity();
    holdIt(undefined); // the composer is free for a new draft; the guard keeps the item's row from sending it meanwhile
    setRefused(null);
    setText("");
    keepDraft(draftKey, ""); // rule 2: out of the kept draft before the send is dispatched, not in the effect after
    const back = (e: unknown) => {
      // typing since wins the composer (18.6 "Send")
      const restore = edited.current === mark;
      if (restore) { setText(t); holdIt(of); }
      if (of === undefined && restore) refuse(e, at);
      else full_(e, at); // keep a newer draft and apply only a current capacity refusal
      if (of !== undefined) refuseTaken(taken!, of, errorWords(e));
    };
    try { Promise.resolve(actions.say(t)).then(() => { if (of !== undefined) drop(taken!, of); }, back); } catch (e) { back(e); }
  };
  const why = queued >= QUEUE_MAX ? `The queue is full: ${QUEUE_MAX} messages wait. Remove one, or stop the turn.`
    : full ? "The queue is full: the messages that wait hold all the text it takes. Remove one, or stop the turn." : null;
  const placeholder = given ?? why ?? (busy ? "Add a message: it is queued and sent when this turn ends"
    : "Message the agent  ·  Enter to send, Shift+Enter for a new line");
  const reasonId = useId();
  const fullReasonId = useId();
  const sendReason = off ? SESSION_IN_ANOTHER_TAB : blocked ?? why ?? (emptyAttempt ? "Enter a message first." : null);
  const fullReason = off || blocked ? null : why;
  const sendDescription = off && offReasonId ? offReasonId : fullReason ? fullReasonId : reasonId;
  const tags = [session?.model, session?.mode].filter((x): x is string => !!x);
  return (
    <form className="wa-composer" onSubmit={(e) => { e.preventDefault(); send(); }}>
      {refused ? <p className="wa-send-err" role="alert">{refused}</p> : null}
      <textarea aria-label="Message" rows={2} value={text} placeholder={placeholder}
        onChange={(e) => { edited.current++; setRefused(null); setText(e.target.value); if (!e.target.value.trim()) holdIt(undefined); }}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) { e.preventDefault(); send(); }
          else if (e.key === "Escape" && busy && !off) actions.stop();
        }} />
      <div className="wa-irow">
        {tags.map((t, n) => <span key={n} className="wa-tag">{n > 0 ? <span aria-hidden="true">· </span> : null}{t}</span>)}
        <span id={fullReasonId} role={fullReason ? "status" : undefined} aria-live="polite" aria-atomic="true" className={fullReason ? "wa-control-reason" : "wa-sr"}>{fullReason}</span>
        {sendReason && !fullReason && !(off && offReasonId) ? <span id={reasonId} role={emptyAttempt && !off && !blocked ? "status" : undefined} className="wa-control-reason">{sendReason}</span> : null}
        {busy ? (
          <button type="button" className="wa-stop" disabled={off} aria-describedby={off ? sendDescription : undefined} onClick={() => actions.stop()}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><rect x="5" y="5" width="14" height="14" rx="2" /></svg>
            {stopping ? "Stopping…" : "Stop"}
          </button>
        ) : null}
        <button type="submit" className="wa-send" aria-label={sendLabel ?? (busy ? "Queue message" : "Send message")} disabled={!!sendReason || !text.trim()} aria-describedby={sendReason ? sendDescription : undefined}>
          <svg width="16" height="16" strokeWidth="2.4" {...svg}><path d="M12 19V5M6 11l6-6 6 6" /></svg>
        </button>
      </div>
    </form>
  );
}

/** A queued message's row: its first 240 code points and "…" when longer. The scan stops at the cut, so a long message
 * costs no more than a short one. */
function preview(text: string, limit = 240): string {
  let end = 0;
  for (let n = 0; n < limit && end < text.length; n++) end += text.codePointAt(end)! > 0xffff ? 2 : 1;
  return end < text.length ? `${text.slice(0, end)}…` : text;
}

export function Dock(props: { view: SessionView; actions: Actions; onNeedsYou: () => void; needsHidden?: boolean; behind?: boolean; onBehind?: () => void; status: ReactNode;
  session?: { model?: string; mode?: string }; stopping?: boolean; sessionId?: string; replaced?: boolean; onUseHere?: () => void }) {
  const { view } = props;
  const active = isActiveTurn(view.turn);
  const resumeLost = view.turn?.state === "resume-lost";
  const showStopAndSend = active || resumeLost;
  const queueTiming = active ? "sent when this turn ends"
    : view.turn?.state === "paused" ? (view.turn.until === undefined ? "send a message to resume" : "sent after the session resumes")
    : view.turn?.state === "idle" && view.turn.stop.kind === "failed" ? "sent when you send another message"
    : "waiting for a new turn";
  const consentWait = isFolderConsent(view.turn);
  const interruptReason = consentWait ? "Stop and send is unavailable while watane waits for folder consent."
    : resumeLost ? "Claude no longer has this session." : null;
  const [opened, setOpened] = useState<boolean | null>(null);
  const done = view.plan?.filter((p) => p.status === "completed").length ?? 0;
  const { actions } = props;
  const off = !!actions.off;
  const localOffId = useId();
  const offId = actions.offReasonId ?? localOffId;
  const queueId = useId();
  const interruptId = useId();
  const pendingId = useId();
  const stopReasonId = off ? offId : interruptReason ? interruptId : undefined;
  const ctl = useRef<Ctl | null>(null);
  const sid = props.sessionId ?? "";
  // 18.6 "Queue" (S36 r4 F4.1, r5): what the actions took is the session's list (taken.ts), outside this component; `note` is
  // only a line with no text of hers in it (a refused take, "Finish or clear ...")
  const taken = useTaken(sid);
  // a Remove's Undo ends at its stored deadline: the page renders again then; the item stays (S36 r5 F5.3). One timer per
  // future deadline of an ordinary Undo; a refused item's retry is not an Undo and has none (S36 r6 F6.2)
  const [, tick] = useState(0);
  const ends = taken.filter((x) => x.tried === "undo" && x.error === undefined && x.until !== undefined && x.until > Date.now()).map((x) => x.until!).join(" ");
  useEffect(() => {
    const ts = ends ? ends.split(" ").map((at) => setTimeout(() => tick((n) => n + 1), Math.max(0, Number(at) - Date.now() + 1))) : [];
    return () => ts.forEach(clearTimeout);
  }, [ends]);
  const [note, setNote] = useState<string | null>(null);
  const live = useRef(true); // set again on setup: Strict Mode runs the cleanup and the setup once more
  useEffect(() => { live.current = true; return () => { live.current = false; }; }, []);
  const fail = (e: unknown) => setNote((e as { kind?: unknown } | null)?.kind === "unknown-input" ? "That message was already sent" : errorWords(e));
  const busyComposer = () => !!ctl.current?.text().trim();
  const draftKey = props.sessionId === undefined ? undefined : `wa-draft:${props.sessionId}`;
  // 18.6 "Queue" (the deputy's rule, S36 r3): take it back first. Edit, Stop and send and Remove call `unqueue(n)` first, the
  // kernel's one atomic take, so a turn can no longer take the item; `unknown-input` (a turn took it) leaves everything alone. The
  // dock is one session's (App keys it by the session); its calls act on that session alone.
  /** Takes input `q` out of the queue; a refusal shows as a line. */
  const unqueue = async (q: { n: number }) => {
    try { await actions.unqueue!(q.n); return true; } catch (e) { fail(e); return false; }
  };
  /** Copies taken item `id` into this session's open composer when that is empty. The item
   * stays listed, with the error, until a send of it is accepted or she discards it (S36 r5 F5.1). */
  const place = (id: string, t: string, e?: unknown) => {
    if (live.current) ctl.current?.put(t, e, id);
    refuseTaken(sid, id, e === undefined ? undefined : errorWords(e));
  };
  const interrupt = async (id: string, t: string) => {
    try { await actions.interrupt!(t); drop(sid, id); } catch (e) { place(id, t, e); } // refused: to the composer, as Send's, or it stays
  };
  const edit = async (q: { n: number; input: string }) => {
    if (busyComposer()) return setNote("Finish or clear your message first");
    setNote(null);
    if (await unqueue(q)) place(take(sid, q.input, "edit", true), q.input); // she sends it as an ordinary `say`
  };
  const remove = async (q: { n: number; input: string }) => { setNote(null); if (await unqueue(q)) take(sid, q.input, "undo"); };
  const stopAndSend = async (q: { n: number; input: string }) => {
    if (busyComposer()) return setNote("Finish or clear your message first"); // a refused interrupt's text needs the composer
    setNote(null);
    if (!(await unqueue(q))) return; // a turn took it: the current turn is left alone
    await interrupt(take(sid, q.input, "stop", true), q.input);
  };
  /** An item's Undo puts it back in the queue (a `say`); a Stop and send's can also be tried again. One call in flight at a time,
   * from its row or the composer, never while the composer holds it; an ordinary Undo past its deadline does nothing (S36 r6);
   * a refused one stays with its error. */
  const again = async (x: Taken, how: "say" | "interrupt") => {
    if (how === "say" && !undoShown(x)) return tick((n) => n + 1);
    if (heldOf(sid) === x.id || !claim(sid, x.id)) return;
    if (how === "interrupt") return interrupt(x.id, x.text);
    try { await actions.say(x.text); drop(sid, x.id); } catch (e) { refuseTaken(sid, x.id, errorWords(e)); }
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
          <p className="wa-queue-label">Queued · {view.queue.length} of {QUEUE_MAX} · {queueTiming}</p>
          <ul role="list">{view.queue.slice(0, SHOWN).map((q, k) => {
            const previewId = `${queueId}-${q.n}`;
            return <li key={q.n}>
              <bdi id={previewId} className="wa-queue-text">{preview(q.input)}</bdi>
              <span className="wa-queue-ctl">
                <button type="button" className="wa-queue-action" aria-label={`Edit queued message ${k + 1}`} disabled={off} aria-describedby={off ? offId : undefined} onClick={() => edit(q)}>Edit</button>
                {showStopAndSend && <button type="button" className="wa-queue-action" aria-label={`Stop and send queued message ${k + 1}`} disabled={!!stopReasonId} aria-describedby={stopReasonId} onClick={() => stopAndSend(q)}>Stop and send</button>}
                <button type="button" className="wa-queue-action wa-queue-remove" aria-label="Remove from queue" disabled={off} aria-describedby={off ? `${previewId} ${offId}` : previewId} onClick={() => remove(q)}>
                  <svg width="14" height="14" strokeWidth="2" {...svg}><path d="M6 6l12 12M18 6L6 18" /></svg>
                </button>
              </span>
            </li>;
          })}</ul>
          {view.queue.length > SHOWN ? <p className="wa-queue-more">+ {view.queue.length - SHOWN} more</p> : null}
        </section>
      ) : null}
      {note ? <p className="wa-queue-note" role="status">{note}</p> : null}
      {taken.length ? (
        <ul className="wa-taken" role="list" aria-label="Taken from the queue">{taken.map((x) => {
          const ordinaryUndo = x.tried === "undo" && x.error === undefined && x.until !== undefined && undoShown(x);
          return (
            <li key={x.id} className="wa-queue-note" role={x.error === undefined ? "status" : undefined}>
              {x.error !== undefined ? <p className="wa-taken-error" role="alert">{x.error}</p> : null}
              {x.error === undefined && x.tried === "undo"
                ? <span className="wa-taken-text">Removed “<bdi>{preview(x.text, 40)}</bdi>” from the queue</span>
                : <>{x.error === undefined && (x.tried === "edit" ? "Taken out to edit." : "Taken out to send.")} <q className="wa-taken-text"><bdi>{preview(x.text)}</bdi></q></>}
              {!x.held && undoShown(x) ? <button type="button" className="wa-queue-action" disabled={off || !!x.out} aria-describedby={x.out ? pendingId : off ? offId : undefined} onClick={() => void again(x, "say")}>Undo</button> : null}
              {!x.held && showStopAndSend && x.tried === "stop"
                ? <button type="button" className="wa-queue-action" disabled={!!stopReasonId || !!x.out} aria-describedby={x.out ? pendingId : stopReasonId} onClick={() => void again(x, "interrupt")}>Stop and send</button> : null}
              {!ordinaryUndo ? <button type="button" className="wa-queue-action" disabled={!!x.out} aria-describedby={x.out ? pendingId : undefined} onClick={() => drop(sid, x.id)}>Discard</button> : null}
            </li>
          );
        })}</ul>
      ) : null}
      {stopReasonId === interruptId && (view.queue.length > 0 || taken.some((x) => !x.held && x.tried === "stop"))
        ? <p id={interruptId} className="wa-control-reason">{interruptReason}</p> : null}
      {taken.some((x) => x.out) ? <p id={pendingId} className="wa-control-reason">Wait for the current send to finish.</p> : null}
      {props.status}
      {off ? (
        <p className="wa-replaced" role="status"><span id={offId}>{SESSION_IN_ANOTHER_TAB}</span>{props.replaced ? <button type="button" className="wa-chip" onClick={props.onUseHere}>Use here</button> : null}</p>
      ) : null}
      <Composer key={props.sessionId} ctl={ctl} epoch={() => epochOf(sid)} taken={props.sessionId} draftKey={draftKey} queued={view.queue.length} actions={props.actions} offReasonId={offId} session={props.session} stopping={props.stopping}
        busy={active} />
    </div>
  );
}
