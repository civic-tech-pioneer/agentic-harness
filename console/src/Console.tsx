// TECHNICAL 18.2: the page's top: an open session, or the new-session view; the frame shows before the kernel has answered.
import { useState } from "react";
import { App } from "./App";
import { Composer } from "./Dock";
import { FOLDER, type SessionInfo } from "./Header";
import { Sidebar, type SidebarSession } from "./Sidebar";
import type { Served } from "./served";
import { startKey } from "./served";

const NEW_KEY = "wa-draft:new";

/** The shell before there is a session: the same sidebar, header and dock as the open session, so nothing blanks (18.2, S35 AC4c). */
export function Frame() {
  return (
    <div className="wa-app" data-frame="loading">
      <nav className="wa-sidebar" aria-label="Sessions"><div className="wa-brand"><h2 className="wa-sidebar-title">Agents</h2></div></nav>
      <main className="wa-main">
        <header className="wa-header"><h1 className="wa-title">&nbsp;</h1></header>
        <div className="wa-scroll"><div className="wa-column" /></div>
        <div className="wa-dock"><div className="wa-status"><span role="status" className="wa-status-word">Loading</span></div></div>
      </main>
    </div>
  );
}

/** 18.2: stage B's new-session view: the profile `scratch`, "A new scratch folder", and the composer whose Send starts the session. */
export function NewSession({ sessions, onNew, start }: { sessions: SidebarSession[]; onNew: () => void; start: (input: string) => Promise<void> }) {
  return (
    <div className="wa-app" data-view="new-session">
      <Sidebar sessions={sessions} onNew={onNew} />
      <main className="wa-main">
        <header className="wa-header">
          <h1 className="wa-title">New session</h1>
          <div className="wa-chips">
            <span className="wa-chip" title="Profile">Profile <b>Scratch</b></span>
            <span className="wa-chip" title="Place">{FOLDER}<b>A new scratch folder</b></span>
          </div>
        </header>
        <div className="wa-scroll" />
        <div className="wa-dock">
          <Composer queued={0} draftKey={NEW_KEY} placeholder="Message Claude…" sendLabel="Start session"
            actions={{ say: start, answer: () => {}, stop: () => {}, takeBack: () => {} }} />
        </div>
      </main>
    </div>
  );
}

/** The newest session open, or the new-session view; Start opens the new session in place, and New session goes back to the view. */
export function Console({ served }: { served: Served }) {
  const { kernel } = served;
  const [info, setInfo] = useState<SessionInfo | null>(served.info);
  const [sessions, setSessions] = useState(served.sessions);
  const start = async (input: string) => {
    const id = await kernel.start(startKey(), "scratch", "{}", input);
    // the new session's own title and the sidebar come from `list`; a failed read leaves the session open under its first words
    let title = input.length > 60 ? `${input.slice(0, 60)}…` : input;
    try {
      const l = await served.list();
      setSessions(l.sessions);
      title = l.info(id)?.title ?? title;
    } catch { /* the session is started: the sidebar catches up at the next list */ }
    setInfo({ id, title, profile: "scratch", provider: "Claude Code (unchecked)", workspace: "Scratch folder" });
  };
  return info
    ? <App kernel={kernel} info={info} sessions={sessions} onNew={() => setInfo(null)} />
    : <NewSession sessions={sessions} onNew={() => setInfo(null)} start={start} />;
}
