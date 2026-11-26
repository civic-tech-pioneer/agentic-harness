// TECHNICAL 18.4, 18.6: title, profile, provider, workspace, session allows, the context meter.
import type { Actions, SessionView } from "../kit";

export interface SessionInfo { id: string; title: string; profile: string; provider: string; workspace: string;
  place?: string; judge?: boolean; model?: string; mode?: string }
/** The title every place shows (18.4): the kernel's; else the first user message cut to 60 characters; else "New session". */
export const titleOf = (title: string, firstUser?: string): string => title || (firstUser ? (firstUser.length > 60 ? `${firstUser.slice(0, 60)}…` : firstUser) : "New session");
/** The sidebar's meta line: `Scratch folder · Claude Code (unchecked)` for a scratch session of claude, else `<backend> · <profile>`. */
export const metaOf = (i: Pick<SessionInfo, "workspace" | "provider" | "profile">): string => (i.workspace ? `${i.workspace} · ${i.provider}` : `${i.provider} · ${i.profile}`);
const OFF = "Not in this console yet.";
const svg = { width: 13, height: 13, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 2, strokeLinecap: "round", strokeLinejoin: "round", "aria-hidden": true } as const;

export const FOLDER = <svg {...svg}><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>;

const k = (n: number) => (n >= 1000 ? `${Math.round(n / 1000)}k` : String(n));

export function ContextMeter({ usage }: { usage: NonNullable<SessionView["usage"]> }) {
  const { used, window: win, compactAt } = usage;
  const limit = Math.min(win, compactAt ?? win);
  const warn = used >= 0.8 * limit;
  const pct = Math.round((used / win) * 100);
  return (
    <div className="wa-meter" data-warn={warn ? "true" : undefined}>
      <span>Context</span>
      <div className={`wa-bar${warn ? " hi" : ""}`} role="meter" aria-label="Context used" aria-valuemin={0} aria-valuemax={win} aria-valuenow={Math.min(used, win)}
        aria-valuetext={`${k(used)} / ${k(win)} tokens${warn ? ", filling up" : ""}`}>
        <i style={{ width: `${Math.min(100, (used / win) * 100)}%` }} />
      </div>
      <span>{pct}%</span>
    </div>
  );
}

export function Header({ info, view, actions }: { info: SessionInfo; view: SessionView; actions: Actions }) {
  const title = info.judge ? "Your judge reads each call before it runs and can deny it." : "No judge reads this provider's calls. Your rules and asks still apply.";
  return (
    <header className="wa-header">
      <h1 className="wa-title">{info.title}</h1>
      <div className="wa-chips">
        <span className="wa-chip" title="Profile">Profile <b>{info.profile}</b></span>
        <span className={`wa-chip${info.judge ? " checked" : ""}`} title={title}>
          <svg {...svg}><path d="M12 3l7 3v6c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9V6z" /></svg>
          {info.provider === "ChatGPT" ? `ChatGPT · ${info.profile}` : `${info.provider} · ${info.judge ? "your judge checks calls" : "no judge"}`}
        </span>
        {info.workspace ? <span className="wa-chip wa-workspace" title="Workspace">{FOLDER}<b title={info.workspace}>{info.workspace}</b></span> : null}
        {view.allows.map((a) => (
          <button key={a.id} type="button" className="wa-chip wa-chip-allow grant" onClick={() => actions.takeBack(a.id)}>{a.reach} this session ×</button>
        ))}
      </div>
      {view.usage ? <ContextMeter usage={view.usage} /> : null}
      <div className="wa-session-menu">
        <button type="button" className="wa-iconbtn" aria-label="Session menu: fork, rewind, rename, delete" aria-disabled="true" aria-describedby="wa-off-menu" onClick={(e) => e.preventDefault()}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><circle cx="5" cy="12" r="1.8" /><circle cx="12" cy="12" r="1.8" /><circle cx="19" cy="12" r="1.8" /></svg>
        </button>
        <span id="wa-off-menu" className="wa-control-reason">{OFF}</span>
      </div>
    </header>
  );
}
