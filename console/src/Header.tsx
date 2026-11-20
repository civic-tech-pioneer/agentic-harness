// TECHNICAL 18.4, 18.6: title, profile, provider, workspace, session allows, the context meter.
import type { Actions, SessionView } from "../kit";

export interface SessionInfo { id: string; title: string; profile: string; provider: string; workspace: string }

const k = (n: number) => (n >= 1000 ? `${Math.round(n / 1000)}k` : String(n));

export function ContextMeter({ usage }: { usage: NonNullable<SessionView["usage"]> }) {
  const { used, window: win, compactAt } = usage;
  const limit = Math.min(win, compactAt ?? win);
  const warn = used >= 0.8 * limit;
  const text = `${k(used)} / ${k(win)}`;
  return (
    <div className="wa-meter" data-warn={warn ? "true" : undefined}>
      <div className="wa-meter-bar" role="meter" aria-label="Context used" aria-valuemin={0} aria-valuemax={win} aria-valuenow={Math.min(used, win)}
        aria-valuetext={`${text} tokens${warn ? ", filling up" : ""}`}>
        <div className="wa-meter-fill" style={{ width: `${Math.min(100, (used / win) * 100)}%` }} />
      </div>
      <span className="wa-meter-text">{text}</span>
      {warn ? <span className="wa-meter-word">Filling up</span> : null}
    </div>
  );
}

export function Header({ info, view, actions }: { info: SessionInfo; view: SessionView; actions: Actions }) {
  return (
    <header className="wa-header">
      <h1 className="wa-title">{info.title}</h1>
      <div className="wa-chips">
        <span className="wa-chip">{info.profile}</span>
        <span className="wa-chip">{info.provider}</span>
        <span className="wa-chip wa-mono">{info.workspace}</span>
        {view.allows.map((a) => (
          <button key={a.id} type="button" className="wa-chip wa-chip-allow" onClick={() => actions.takeBack(a.id)}>{a.reach} this session ×</button>
        ))}
      </div>
      {view.usage ? <ContextMeter usage={view.usage} /> : null}
    </header>
  );
}
