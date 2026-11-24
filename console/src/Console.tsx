// TECHNICAL 18.2: the page's top: an open session, or the new-session view; index.html's static frame shows until the kernel has answered (S35 AC4c).
import { useState } from "react";
import { App } from "./App";
import { Composer } from "./Dock";
import { FOLDER, type SessionInfo } from "./Header";
import { Sidebar, type SidebarSession } from "./Sidebar";
import type { Served } from "./served";
import { startKey } from "./served";

const NEW_KEY = "wa-draft:new";

/** 18.2: The workspace choice and composer for a new session. */
export function NewSession({ sessions, onNew, start }: { sessions: SidebarSession[]; onNew: () => void; start: (input: string, folder?: string) => Promise<void> }) {
  const [folder, setFolder] = useState("");
  return (
    <div className="wa-app" data-view="new-session">
      <Sidebar sessions={sessions} onNew={onNew} />
      <main className="wa-main">
        <header className="wa-header">
          <h1 className="wa-title">New session</h1>
          <div className="wa-chips">
            <span className="wa-chip" title="Profile">Profile <b>scratch</b></span>
            <label className="wa-chip wa-folder-path">
              {FOLDER} Folder path
              <input value={folder} onChange={(e) => setFolder(e.target.value)}
                aria-describedby="wa-folder-default" />
            </label>
            <span id="wa-folder-default" className="wa-folder-default">A new scratch folder if left blank</span>
          </div>
        </header>
        <div className="wa-scroll" />
        <div className="wa-dock">
          <Composer queued={0} draftKey={NEW_KEY} placeholder="Message Claude…" sendLabel="Start session"
            actions={{ say: (input) => start(input, folder.trim() ? folder : undefined), answer: () => {}, stop: () => {}, takeBack: () => {} }} />
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
  const start = async (input: string, folder?: string) => {
    const id = folder === undefined
      ? await kernel.start(startKey(), "scratch", "{}", input)
      : await kernel.start(startKey(), "scratch", "{}", input, folder);
    // the new session's own title and the sidebar come from `list`; a failed read leaves the session open under its first words
    let title = input.length > 60 ? `${input.slice(0, 60)}…` : input;
    let workspace = folder ?? "Scratch folder";
    try {
      const l = await served.list();
      setSessions(l.sessions);
      const session = l.info(id);
      title = session?.title ?? title;
      workspace = session?.workspace || workspace;
    } catch { /* the session is started: the sidebar catches up at the next list */ }
    setInfo({ id, title, profile: "scratch", provider: "Claude Code (unchecked)", workspace });
  };
  return info
    ? <App kernel={kernel} info={info} sessions={sessions} onNew={() => setInfo(null)} />
    : <NewSession sessions={sessions} onNew={() => setInfo(null)} start={start} />;
}
