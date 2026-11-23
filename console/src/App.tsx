// TECHNICAL 18.4-18.6: the console's shell: sidebar, header, transcript column, dock.
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { decisionJson, emptyView, FOCUS_END, focusInOpenAsk, isAskRefusal, MEASURE, openAsk, releaseOpenAsk, openSession, ThreadProvider, Transcript, StatusLine, type Actions, type Ask, type Kernel, type SessionView } from "../kit";
import { Sidebar, type SidebarSession } from "./Sidebar";
import { Header, titleOf, type SessionInfo } from "./Header";
import { Dock } from "./Dock";

let sent = 0;
const LOADING: SessionView = { ...emptyView, loading: true };

export function App({ kernel, info, sessions, onNew }: { kernel: Kernel; info: SessionInfo; sessions?: SidebarSession[]; onNew?: () => void }) {
  // the view is tagged with its subscription (a token per run of the openSession effect), and with the session and kernel it was opened on
  const [held, setHeld] = useState<{ tok: object | null; id: string; kernel: Kernel; view: SessionView }>({ tok: null, id: info.id, kernel, view: LOADING });
  const [sub, setSub] = useState<object | null>(null); // the current subscription's token
  const mine = held.tok === sub && held.id === info.id && held.kernel === kernel;
  const view = mine ? held.view : LOADING; // a new subscription shows Loading until its first view
  const firstUser = view.rows.find((r) => r.kind === "user");
  const named = { ...info, title: titleOf(info.title, firstUser?.kind === "user" ? firstUser.text : undefined) };
  const [stopping, setStopping] = useState(false);
  const [now, setNow] = useState(() => Date.now());
  const scroller = useRef<HTMLDivElement>(null);
  const atEnd = useRef(true);
  const setAt = useRef(-1);
  const [behind, setBehind] = useState(false); // output arrived below while the reader is not following
  const [answered, setAnswered] = useState<ReadonlySet<string>>(new Set()); // asks this console answered that have no decision entry yet (9.4 rule 4)
  const answeredAsk = useRef(new Map<string, Ask | undefined>()); // the ask object each answered id stood for when it was answered
  const viewRef = useRef(view); viewRef.current = view;
  const heldRef = useRef(held); heldRef.current = held;
  const subRef = useRef(sub); subRef.current = sub;
  const [replaced, setReplaced] = useState(false); // 18.6 "Another tab": the stream went to another tab
  const reattach = useRef<(() => void) | null>(null);
  const [needsHidden, setNeedsHidden] = useState(false);
  const lastAsk = useRef<string | null>(null); // the ask whose card last took focus
  // ask ids repeat across sessions: what this console answered is per session; a new session opens at its end, not stopping
  useEffect(() => { setAnswered(new Set()); answeredAsk.current.clear(); setStopping(false); atEnd.current = true; setBehind(false); lastAsk.current = null; }, [info.id]);

  useEffect(() => {
    const id = info.id;
    let open = true; // a late view of a closed session is never kept
    const tok = {};
    setSub(tok);
    setReplaced(false);
    const s = openSession(kernel, id, (v) => { if (open) setHeld({ tok, id, kernel, view: v }); }, (r) => { if (open) setReplaced(r); });
    reattach.current = s.reattach;
    return () => { open = false; reattach.current = null; s.close(); };
  }, [kernel, info.id]);
  // 18.6 Place: follow the end while the reader is within 40 px of it (and on open).
  const toEnd = useCallback(() => {
    const el = scroller.current;
    if (el && focusInOpenAsk(el)) atEnd.current = false; // also when the ask opened on the card that had focus: the pill shows
    if (el && atEnd.current) { el.scrollTop = el.scrollHeight; setAt.current = el.scrollTop; }
  }, []);
  useLayoutEffect(toEnd, [view, toEnd]);
  const first = useRef(true);
  useEffect(() => { if (first.current) { first.current = false; return; } if (!atEnd.current) setBehind(true); }, [view]);
  useEffect(() => {
    const col = scroller.current?.firstElementChild;
    if (!col || typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(() => { scroller.current?.dispatchEvent(new Event(MEASURE)); toEnd(); });
    ro.observe(col);
    return () => ro.disconnect();
  }, [toEnd]);
  useEffect(() => { const t = setInterval(() => setNow(Date.now()), 1000); return () => clearInterval(t); }, []);
  useEffect(() => { if (view.turn?.state === "idle") setStopping(false); }, [view.turn]);

  // 9.4 rule 4: an ask with an answer sent counts as decided: Needs you, its target and the status subject skip it
  // it also ends when the row's ask gets its decision entry or is replaced by another ask under the same id (a later stream reuses ids);
  // the set is filtered while rendering, so no frame shows "Answered, waiting" on an ask that left it
  const live = useMemo<ReadonlySet<string>>(() => {
    if (!answered.size) return answered;
    const keep = (id: string) => { const a = openAsk(view, id), was = answeredAsk.current.get(id); return !!a && !a.decision && (was === undefined || a === was); };
    return [...answered].every(keep) ? answered : new Set([...answered].filter(keep));
  }, [answered, view]);
  useEffect(() => { if (live !== answered) setAnswered((s) => (s === answered ? live : s)); }, [live, answered]);
  const answeredAsks = useMemo(() => new Map([...live].flatMap((id) => { const a = openAsk(view, id); return a ? [[id, a] as const] : []; })), [live, view]);
  const send = (id: string, d: Parameters<Actions["answer"]>[1], mine: Ask): Promise<void> => {
    answeredAsk.current.set(id, mine);
    setAnswered((a) => new Set(a).add(id));
    let p: Promise<void>;
    try { p = kernel.answer(info.id, `c${Date.now()}-${sent++}`, id, decisionJson(d)); } catch (e) { p = Promise.reject(e); } // a throw before the promise is a refusal too: the set and the card agree
    return p.catch((e) => {
      if (!isAskRefusal(e) && answeredAsk.current.get(id) === mine) { answeredAsk.current.delete(id); setAnswered((a) => { const n = new Set(a); n.delete(id); return n; }); } // "already decided" keeps it: the decision entry is on its way
      throw e;
    });
  };
  const off = replaced;
  const actions: Actions = useMemo(() => ({
    off,
    answeredAsks,
    say: (text) => kernel.say(info.id, `c${Date.now()}-${sent++}`, text),
    answer: () => Promise.reject(new Error("an answer goes out only with the ask its card drew")), // fail closed: no drawn ask, no identity check
    answerDrawn: (drawn, d) => {
      const h = heldRef.current;
      if (!(h.tok === subRef.current && h.id === info.id && h.kernel === kernel)) return Promise.reject(new Error("the view is not this session's")); // fail closed
      const mine = openAsk(viewRef.current, drawn.id);
      // fail closed: an answer for an ask that is not the open one of its id sends and records nothing
      return mine === drawn ? send(drawn.id, d, mine) : Promise.reject(new Error("the ask is no longer the one the card drew"));
    },
    stop: () => { setStopping(true); void kernel.interrupt(info.id); },
    unqueue: (n) => kernel.unqueue(info.id, n),
    interrupt: (replacement) => {
      setStopping(true);
      return kernel.interrupt(info.id, replacement).catch((e) => { setStopping(false); throw e; });
    },
    takeBack: (id) => void kernel.takeBack(info.id, id),
  }), [kernel, info.id, answeredAsks, off]);

  const shown = useMemo(() => live.size && view.openAsks.some((id) => live.has(id)) ? { ...view, openAsks: view.openAsks.filter((id) => !live.has(id)) } : view, [view, live]);

  const toTheEnd = useCallback(() => {
    const el = scroller.current;
    if (!el) return;
    releaseOpenAsk(el);
    if (focusInOpenAsk(el)) (el.ownerDocument.activeElement as HTMLElement).blur(); // the pill takes focus (Safari does not give it on click)
    atEnd.current = true; // the transcript resumes too
    el.scrollTop = el.scrollHeight; setAt.current = el.scrollTop;
    el.dispatchEvent(new Event("scroll"));
    el.dispatchEvent(new Event(FOCUS_END)); // focus goes to the newest row, not the page body
    setBehind(false);
  }, []);

  const askCard = (id: string) => scroller.current?.ownerDocument.querySelector<HTMLElement>(`[data-ask="${CSS.escape(id)}"][data-asking]`) ?? null;
  // 18.6: Needs you hides while the only open ask's card is fully in view and holds focus
  const measureNeeds = useCallback(() => {
    const el = scroller.current, id = shown.openAsks.length === 1 ? shown.openAsks[0]! : undefined;
    const card = id === undefined ? null : askCard(id);
    let hide = false;
    if (el && card && card.contains(el.ownerDocument.activeElement)) {
      const c = card.getBoundingClientRect(), s = el.getBoundingClientRect();
      hide = c.top >= s.top - 0.5 && c.bottom <= s.bottom + 0.5;
    }
    setNeedsHidden(hide);
  }, [shown.openAsks]);
  useLayoutEffect(measureNeeds, [measureNeeds, view]);
  useEffect(() => { const e = scroller.current; e?.addEventListener(MEASURE, measureNeeds); return () => e?.removeEventListener(MEASURE, measureNeeds); }, [measureNeeds]);

  const needsYou = useCallback(() => {
    // the next open ask whose card does not hold focus (the first, when every card does)
    // (a click on the pill takes focus itself, so the card that held it is the one last focused)
    const act = scroller.current?.ownerDocument.activeElement ?? null;
    const holds = (a: string) => { const c = askCard(a); return !!c && (c.contains(act) || (!!act?.closest(".wa-needs") && lastAsk.current === a)); };
    const id = shown.openAsks.find((a) => askCard(a) && !holds(a)) ?? shown.openAsks[0];
    const card = id === undefined ? null : askCard(id);
    const btn = card?.querySelector<HTMLElement>("[data-ask-first]");
    if (!card || !btn) return;
    // The header says what asks, so a card that fits starts at the top; a taller one scrolls so the ask bar (with the
    // focused button) is fully in view. Focus must not scroll again.
    const fits = card.getBoundingClientRect().height <= (scroller.current?.clientHeight ?? 0);
    if (fits) card.scrollIntoView?.({ block: "start" });
    else (card.querySelector<HTMLElement>(".wa-askbar") ?? card).scrollIntoView?.({ block: "end" });
    scroller.current?.dispatchEvent(new Event("scroll")); // the position changed now: tell the transcript and the follow before anything else renders
    btn.focus({ preventScroll: true });
  }, [shown.openAsks]);

  return (
    <div className="wa-app">
      <Sidebar info={named} turn={view.turn?.state} needs={shown.openAsks.length > 0} sessions={sessions} onNew={onNew} />
      <main className="wa-main">
        <Header info={named} view={view} actions={actions} />
        <ThreadProvider view={view} actions={actions}>
          <div className={`wa-scroll${behind && !atEnd.current ? " wa-room" : ""}`} ref={scroller}
            // 18.6 Place: while focus is inside a card with an open ask, the transcript does not follow the end.
            onFocus={(e) => { const t = e.target as Element; if (t.closest("[data-asking]")) atEnd.current = false; lastAsk.current = t.closest("[data-ask]")?.getAttribute("data-ask") ?? null; measureNeeds(); }}
            onBlur={() => queueMicrotask(measureNeeds)}
            onScroll={(e) => {
              measureNeeds();
              // Our own scroll to the end may land after the column grew; that one does not stop the follow.
              const el = e.currentTarget, gap = el.scrollHeight - el.clientHeight - el.scrollTop;
              if (gap > 40 && el.scrollTop === setAt.current) return;
              atEnd.current = !focusInOpenAsk(el) && gap <= 40;
              if (atEnd.current) setBehind(false);
            }}>
            <div className="wa-column"><Transcript view={view} actions={actions} /></div>
          </div>
          <Dock view={shown} actions={actions} onNeedsYou={needsYou} needsHidden={needsHidden} behind={behind && !atEnd.current} onBehind={toTheEnd} session={{ model: info.model, mode: info.mode }} sessionId={info.id} stopping={stopping} replaced={replaced} onUseHere={() => reattach.current?.()}
            status={<StatusLine view={shown} answered={view.openAsks.length - shown.openAsks.length} now={now} stopping={stopping} actions={actions} onNeedsYou={needsYou} />} />
        </ThreadProvider>
      </main>
    </div>
  );
}
