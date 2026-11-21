// TECHNICAL 18.4-18.6: the sidebar, 260 px, hidden under 760 px: Agents, the off "+ New session", rows grouped by state.
import { useEffect, useState } from "react";
import type { TurnState } from "../kit";
import type { SessionInfo } from "./Header";

export type SessionShape = "needs" | "running" | "paused" | "idle" | "unknown";
export interface SidebarSession { id: string; title: string; place: string; meta: string; state: SessionShape }

const WORD: Record<SessionShape, string> = { needs: "Needs you", running: "Running", paused: "Paused", idle: "Idle", unknown: "Unknown" };
const GLYPH: Record<SessionShape, string> = { needs: "g-ask", running: "g-run", paused: "g-pause", idle: "g-idle", unknown: "g-unknown" };
const COLORS = ["#1B7346", "#3F4452", "#7B5CD6", "#3355D6", "#B4541F", "#0F6E7A"];
const colorOf = (place: string) => {
  let h = 0;
  for (const c of place) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return COLORS[h % COLORS.length];
};

export function shapeOf(turn: TurnState["state"] | undefined): SessionShape {
  switch (turn) {
    case "waiting": return "needs";
    case "running": return "running";
    case "paused": return "paused";
    case "unknown": case "resume-lost": return "unknown";
    default: return "idle";
  }
}
type Group = "needs" | "running" | "recent";
const groupOf = (s: SessionShape): Group => (s === "needs" ? "needs" : s === "running" ? "running" : "recent");
const GROUPS: [Group, string][] = [["needs", "Needs you"], ["running", "Running"], ["recent", "Recent"]];
const OFF = "Not in this console yet.";

function Row({ s, on }: { s: SidebarSession; on?: boolean }) {
  return (
    <button type="button" className="wa-srow" aria-current={on ? "true" : undefined}>
      <span className="wa-mono" aria-hidden="true" style={{ background: colorOf(s.place) }}>{s.place.charAt(0).toUpperCase()}</span>
      <span className="wa-stext"><span className="wa-stitle">{s.title}</span><span className="wa-smeta">{s.meta}</span></span>
      <span className={`glyph ${GLYPH[s.state]}`} role="img" aria-label={WORD[s.state]} />
    </button>
  );
}

export function Sidebar({ info, turn, sessions = [] }: { info: SessionInfo; turn?: TurnState["state"]; sessions?: SidebarSession[] }) {
  const [held, setHeld] = useState<Group | null>(null);
  const [over, setOver] = useState(false);
  const live = shapeOf(turn);
  // The hold begins once the state is known: a pointer already over the list before the first turn holds nothing.
  useEffect(() => { if (over && turn !== undefined) setHeld((h) => h ?? groupOf(live)); }, [over, turn]);
  const group = held ?? groupOf(live);
  const cur: SidebarSession = { id: info.id, title: info.title, place: info.place ?? info.workspace, meta: `${info.place ?? info.workspace} · ${info.provider}`, state: live };
  const others = sessions.filter((s) => s.id !== info.id);
  return (
    <nav className="wa-sidebar" aria-label="Sessions" onMouseEnter={() => setOver(true)} onMouseLeave={() => { setOver(false); setHeld(null); }}>
      <div className="wa-brand">
        <span className="wa-markshape" aria-hidden="true">
          <i style={{ background: "#3355D6" }} /><i style={{ background: "#D9534F", borderRadius: "50%" }} /><i style={{ background: "#7B5CD6", transform: "rotate(45deg)" }} />
        </span>
        <h2 className="wa-sidebar-title">Agents</h2>
      </div>
      <button type="button" className="wa-newbtn" aria-disabled="true" aria-describedby="wa-off-new" onClick={(e) => e.preventDefault()}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
        New session
      </button>
      <span id="wa-off-new" className="wa-sr">{OFF} (18.8)</span>
      {GROUPS.map(([g, label]) => {
        const rows = others.filter((s) => groupOf(s.state) === g);
        const here = group === g;
        if (!rows.length && !here) return null;
        return (
          <div className="wa-group" key={g}>
            <div className="wa-glabel">{label}</div>
            {here ? <Row s={cur} on /> : null}
            {rows.map((s) => <Row key={s.id} s={s} />)}
          </div>
        );
      })}
    </nav>
  );
}
