// TECHNICAL 18.4: the sidebar, 260 px, hidden under 760 px. One session until the list grows past it (out of scope).
import type { SessionInfo } from "./Header";

export function Sidebar({ info }: { info: SessionInfo }) {
  return (
    <nav className="wa-sidebar" aria-label="Sessions">
      <h2 className="wa-sidebar-title">Sessions</h2>
      <ul>
        <li>
          <button type="button" className="wa-session-row" aria-current="true">
            <span className="wa-session-title">{info.title}</span>
            <span className="wa-session-meta">{info.provider} · {info.workspace}</span>
          </button>
        </li>
      </ul>
    </nav>
  );
}
