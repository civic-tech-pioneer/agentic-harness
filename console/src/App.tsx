// TECHNICAL 18.4-18.6: the console's shell: sidebar, header, transcript column, dock.
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { decisionJson, emptyView, focusInOpenAsk, MEASURE, openSession, ThreadProvider, Transcript, StatusLine, type Actions, type Kernel, type SessionView } from "../kit";
import { Sidebar, type SidebarSession } from "./Sidebar";
import { Header, type SessionInfo } from "./Header";
import { Dock } from "./Dock";

let sent = 0;
export function App({ kernel, info, sessions }: { kernel: Kernel; info: SessionInfo; sessions?: SidebarSession[] }) {
  const [view, setView] = useState<SessionView>(emptyView);
  const [stopping, setStopping] = useState(false);
  const [now, setNow] = useState(() => Date.now());
  const scroller = useRef<HTMLDivElement>(null);
  const atEnd = useRef(true);
  const setAt = useRef(-1);
  const [behind, setBehind] = useState(false); // output arrived below while the reader is not following

  useEffect(() => {
    const s = openSession(kernel, info.id, setView);
    return () => s.close();
  }, [kernel, info.id]);
  // 18.6 Place: follow the end while the reader is within 40 px of it (and on open).
  const toEnd = useCallback(() => {
    const el = scroller.current;
    if (el && atEnd.current && !focusInOpenAsk(el)) { el.scrollTop = el.scrollHeight; setAt.current = el.scrollTop; }
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

  const actions: Actions = useMemo(() => ({
    say: (text) => void kernel.say(info.id, `c${Date.now()}-${sent++}`, text),
    answer: (id, d) => void kernel.answer(info.id, `c${Date.now()}-${sent++}`, id, decisionJson(d)),
    stop: () => { setStopping(true); void kernel.interrupt(info.id); },
    takeBack: (id) => void kernel.takeBack(info.id, id),
  }), [kernel, info.id]);

  const toTheEnd = useCallback(() => {
    const el = scroller.current;
    if (!el) return;
    (el.ownerDocument.activeElement as HTMLElement | null)?.blur?.(); // the pill takes focus (Safari does not give it on click)
    atEnd.current = true; // the transcript resumes too
    el.scrollTop = el.scrollHeight; setAt.current = el.scrollTop;
    el.dispatchEvent(new Event("scroll"));
    setBehind(false);
  }, []);

  const needsYou = useCallback(() => {
    const id = view.openAsks[0];
    const card = id === undefined ? null : scroller.current?.ownerDocument.querySelector<HTMLElement>(`[data-ask="${CSS.escape(id)}"]`);
    const btn = card?.querySelector<HTMLElement>("[data-ask-first]");
    if (!card || !btn) return;
    // The header says what asks, so a card that fits starts at the top; a taller one scrolls so the ask bar (with the
    // focused button) is fully in view. Focus must not scroll again.
    const fits = card.getBoundingClientRect().height <= (scroller.current?.clientHeight ?? 0);
    if (fits) card.scrollIntoView?.({ block: "start" });
    else (card.querySelector<HTMLElement>(".wa-askbar") ?? card).scrollIntoView?.({ block: "end" });
    scroller.current?.dispatchEvent(new Event("scroll")); // the position changed now: tell the transcript and the follow before anything else renders
    btn.focus({ preventScroll: true });
  }, [view.openAsks]);

  return (
    <div className="wa-app">
      <Sidebar info={info} turn={view.turn?.state} sessions={sessions} />
      <main className="wa-main">
        <Header info={info} view={view} actions={actions} />
        <ThreadProvider view={view} actions={actions}>
          <div className="wa-scroll" ref={scroller}
            // 18.6 Place: while focus is inside a card with an open ask, the transcript does not follow the end.
            onFocus={(e) => { if ((e.target as Element).closest("[data-asking]")) atEnd.current = false; }}
            onScroll={(e) => {
              // Our own scroll to the end may land after the column grew; that one does not stop the follow.
              const el = e.currentTarget, gap = el.scrollHeight - el.clientHeight - el.scrollTop;
              if (gap > 40 && el.scrollTop === setAt.current) return;
              atEnd.current = !focusInOpenAsk(el) && gap <= 40;
              if (atEnd.current) setBehind(false);
            }}>
            <div className="wa-column"><Transcript view={view} actions={actions} /></div>
          </div>
          <Dock view={view} actions={actions} onNeedsYou={needsYou} behind={behind && !atEnd.current} onBehind={toTheEnd} session={{ model: info.model, mode: info.mode }} stopping={stopping}
            status={<StatusLine view={view} now={now} stopping={stopping} actions={actions} onNeedsYou={needsYou} />} />
        </ThreadProvider>
      </main>
    </div>
  );
}
