// TECHNICAL 18.5: the status line, one shape per state; only the word and its subject are live.
import type { Actions, Row, SessionView } from "./model";

type Shape = "dot" | "square" | "diamond" | "bars" | "ring" | "dashed-ring" | "triangle";
const NAMES: Record<Shape, string> = { dot: "Running", square: "Checking", diamond: "Waiting for you", bars: "Paused", ring: "Idle", "dashed-ring": "Not running", triangle: "Failed" };
interface Spec { word: string; shape: Shape; clock?: string; act?: { label: string; key?: string; run: () => void } }

const pad = (n: number) => String(n).padStart(2, "0");
const clock = (ms: number) => { const s = Math.max(0, Math.floor(ms / 1000)); return `${Math.floor(s / 60)}:${pad(s % 60)}`; };
const hhmm = (ms: number) => { const d = new Date(ms); return `${pad(d.getHours())}:${pad(d.getMinutes())}`; };
const file = (p?: string) => p ?? "";

/** The word of a plain running turn: the call or message in flight, else Thinking. */
function runningWord(rows: Row[]): string {
  const last = rows[rows.length - 1];
  if (last?.kind === "agent" && last.writing) return "Writing";
  const call = last?.kind === "tool" ? last.call : last?.kind === "explored" ? last.calls[last.calls.length - 1] : undefined;
  if (call && (call.status === "pending" || call.status === "in_progress")) {
    const f = file(call.locations[0]?.path);
    if (call.kind === "read") return f ? `Reading ${f}` : "Reading";
    if (call.kind === "edit") return f ? `Editing ${f}` : "Editing";
  }
  return "Thinking";
}

function spec(view: SessionView, now: number, actions: Actions, onNeedsYou: () => void): Spec {
  const t = view.turn;
  if (!t) return { word: "Idle", shape: "ring" };
  switch (t.state) {
    case "running": {
      const clk = clock(now - t.since);
      const stop = { label: "Stop", key: "Esc", run: () => actions.stop() };
      const a = t.activity;
      if (a?.kind === "checking") return { word: "Checking with your judge", shape: "square", clock: clk };
      if (a?.kind === "compacting") return { word: "Making room", shape: "dot", clock: clk };
      if (a?.kind === "consent") return { word: "Waiting for your answer in watane", shape: "diamond", act: { label: "Open the question", run: onNeedsYou } };
      return { word: runningWord(view.rows), shape: "dot", clock: clk, act: stop };
    }
    case "waiting": return { word: "Waiting for you", shape: "diamond", clock: clock(t.expiresAt - now), act: { label: "Needs you", run: onNeedsYou } };
    case "paused": return { word: `Paused until ${hhmm(t.until)}`, shape: "bars", act: { label: "Resume now", run: () => actions.say("Resume now") } };
    case "idle":
      if (t.stop.kind === "stopped") return { word: `Stopped at the ${t.stop.limit}`, shape: "ring" };
      if (t.stop.kind === "failed") return { word: `Failed: ${t.stop.error}`, shape: "triangle", act: { label: "Try again", run: () => actions.say("Try again") } };
      return { word: "Idle", shape: "ring" };
    case "unknown": return { word: "Not running: watane restarted", shape: "dashed-ring", act: { label: "Continue", run: () => actions.say("Continue") } };
    case "resume-lost": return { word: "Claude no longer has this session", shape: "dashed-ring", act: { label: "Continue from a summary", run: () => actions.say("Continue from a summary") } };
  }
}

export function StatusLine(props: { view: SessionView; now: number; stopping: boolean; actions: Actions; onNeedsYou: () => void }) {
  const { view, now, stopping } = props;
  const s: Spec = stopping && view.turn?.state !== "idle"
    ? { word: "Stopping…", shape: "dot" }
    : spec(view, now, props.actions, props.onNeedsYou);
  return (
    <div className="wa-status" data-state={view.turn?.state ?? "none"}>
      <span className="wa-shape" data-shape={s.shape} role="img" aria-label={NAMES[s.shape]} />
      <span role="status" className="wa-status-word">{s.word}</span>
      {s.clock ? <span className="wa-status-clock" aria-hidden="true">{s.clock}</span> : null}
      {s.act ? <button type="button" onClick={s.act.run}>{s.act.label}{s.act.key ? <> <kbd>{s.act.key}</kbd></> : null}</button> : null}
    </div>
  );
}
