// TECHNICAL 18.4-18.6: the console's shell: sidebar, header, transcript column, dock.
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { emptyView, openSession, ThreadProvider, Transcript, StatusLine, type Actions, type Kernel, type SessionView } from "../kit";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { Sidebar } from "./Sidebar";
import { Header, type SessionInfo } from "./Header";
import { Dock } from "./Dock";

// The shell's session facts (title, profile, provider, workspace) are not in the kernel stream yet: fixed for the fake.
export const INFO: SessionInfo = { id: "s1", title: "Fix the retry bug in sync.ts", profile: "Careful", provider: "Claude Code", workspace: "~/work/sync" };

/** The scripted kernel with the whole story played: what a page opens until a real kernel exists. */
export function scriptedKernel(): Kernel {
  const fake = fakeKernel(story);
  for (let n = 0; n < story.length; n++) fake.play(n);
  return fake;
}

let sent = 0;
export function App(props: { kernel?: Kernel; info?: SessionInfo }) {
  const info = props.info ?? INFO;
  const kernel = useMemo(() => props.kernel ?? scriptedKernel(), [props.kernel]);
  const [view, setView] = useState<SessionView>(emptyView);
  const [stopping, setStopping] = useState(false);
  const [now, setNow] = useState(() => Date.now());
  const scroller = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const s = openSession(kernel, info.id, setView);
    return () => s.close();
  }, [kernel, info.id]);
  useEffect(() => { const t = setInterval(() => setNow(Date.now()), 1000); return () => clearInterval(t); }, []);
  useEffect(() => { if (view.turn?.state === "idle") setStopping(false); }, [view.turn]);

  const actions: Actions = useMemo(() => ({
    say: (text) => void kernel.say(info.id, `c${Date.now()}-${sent++}`, text),
    answer: (id, d) => void kernel.answer(info.id, `c${Date.now()}-${sent++}`, id, JSON.stringify(d)),
    stop: () => { setStopping(true); void kernel.interrupt(info.id); },
    takeBack: (id) => void kernel.takeBack(info.id, id),
  }), [kernel, info.id]);

  const needsYou = useCallback(() => {
    const id = view.openAsks[0];
    const btn = id === undefined ? null : scroller.current?.ownerDocument.querySelector<HTMLElement>(`[data-call="${CSS.escape(id)}"] [data-ask-first]`);
    if (!btn) return;
    btn.scrollIntoView?.({ block: "center" });
    btn.focus();
  }, [view.openAsks]);

  return (
    <div className="wa-app">
      <Sidebar info={info} />
      <main className="wa-main">
        <Header info={info} view={view} actions={actions} />
        <ThreadProvider view={view} actions={actions}>
          <div className="wa-scroll" ref={scroller}>
            <div className="wa-column"><Transcript view={view} actions={actions} /></div>
          </div>
          <Dock view={view} actions={actions} onNeedsYou={needsYou}
            status={<StatusLine view={view} now={now} stopping={stopping} actions={actions} onNeedsYou={needsYou} />} />
        </ThreadProvider>
      </main>
    </div>
  );
}
