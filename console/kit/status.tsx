// TECHNICAL 18.5: the status line, one shape per state; only the word and its subject are live.
import { useId } from "react";
import { cardName, type Actions, Row, SessionView } from "./model";
import { openAsk } from "./reduce";
import { answeredWords } from "./parts/askbar";

type Shape = "dot" | "square" | "diamond" | "bars" | "ring" | "dashed-ring" | "triangle" | "loading";
const NAMES: Record<Shape, string> = { dot: "Running", square: "Checking", diamond: "Waiting for you", bars: "Paused", ring: "Idle", "dashed-ring": "Not running", triangle: "Failed", loading: "Loading" };
/** An act with no `run` has no kernel call yet: it shows disabled, with a description saying so. */
interface Spec { word: string; shape: Shape; clock?: string; esc?: boolean; act?: { label: string; key?: string; run?: () => void } }

const pad = (n: number) => String(n).padStart(2, "0");
const clock = (ms: number) => { const s = Math.max(0, Math.floor(ms / 1000)); return `${Math.floor(s / 60)}:${pad(s % 60)}`; };
const elapsed = (ms: number) => { const s = Math.max(0, Math.floor(ms / 1000)); return s < 60 ? `${s} s` : clock(ms); };
const hhmm = (ms: number) => { const d = new Date(ms); return `${pad(d.getHours())}:${pad(d.getMinutes())}`; };
const file = (p?: string) => p ?? "";

/** The word of a plain running turn: the call or message in flight, Thinking while a thought streams, else Working (no activity is named). */
function runningWord(rows: Row[]): string {
  const last = rows[rows.length - 1];
  if (last?.kind === "agent" && last.writing) return "Writing";
  const call = last?.kind === "tool" && !last.ended ? last.call : last?.kind === "explored" ? last.calls[last.calls.length - 1] : undefined;
  if (call && (call.status === "pending" || call.status === "in_progress")) {
    const f = file(call.locations[0]?.path);
    if (call.kind === "read") return f ? `Reading ${f}` : "Reading";
    if (call.kind === "edit") return f ? `Editing ${f}` : "Editing";
  }
  return last?.kind === "thought" && last.streaming ? "Thinking" : "Working";
}

/** " · Edit src/sync.ts": the first open ask's card name (the card header's own, `cardName`). */
function askSubject(view: SessionView): string {
  const id = view.openAsks[0];
  const open = id === undefined ? undefined : openAsk(view, id);
  if (!open) return "";
  for (const r of view.rows) {
    if (r.kind === "tool" && r.ask === open) return ` · ${cardName(r.call, r.ask)}`;
  }
  return "";
}

/** 18.5: the cause of a failed turn in words: `no-binary` is Claude Code missing; another cause is its own text; none, the error's name. */
function failedWords(stop: { error: string; cause?: string }): string {
  if (stop.cause === "no-binary") return "Claude Code is not available here";
  return stop.cause ?? stop.error;
}

function spec(view: SessionView, now: number, onNeedsYou: () => void, answered: number): Spec {
  const t = view.turn;
  if (!t) return view.loading ? { word: "Loading", shape: "loading" } : { word: "Idle", shape: "ring" };
  switch (t.state) {
    case "running": {
      const clk = elapsed(now - t.since);
      const a = t.activity;
      if (a?.kind === "checking") return { word: "Checking with your judge", shape: "square", clock: clk, esc: true };
      if (a?.kind === "compacting") return { word: "Making room", shape: "dot", clock: clk, esc: true };
      if (a?.kind === "consent") return { word: "Waiting for your answer in watane", shape: "diamond", act: { label: "Open the question", run: onNeedsYou } };
      return { word: runningWord(view.rows), shape: "dot", clock: clk, esc: true };
    }
    case "waiting": {
      // every open ask answered by this console: the cards' own words, no countdown; else the next ask's own `expires-at`
      if (answered > 0 && view.openAsks.length === 0) return { word: answeredWords(false), shape: "diamond" };
      const next = view.openAsks[0] === undefined ? undefined : openAsk(view, view.openAsks[0]);
      return { word: `Waiting for you${askSubject(view)}`, shape: "diamond", clock: `denied in ${clock((next?.expiresAt ?? t.expiresAt) - now)} if unanswered` };
    }
    case "paused": return { word: t.until === undefined ? "Paused" : `Paused until ${hhmm(t.until)}`, shape: "bars", act: { label: "Resume now" } };
    case "idle":
      if (t.stop.kind === "stopped") return { word: `Stopped at the ${t.stop.limit}`, shape: "ring" };
      if (t.stop.kind === "failed") return { word: `Failed: ${failedWords(t.stop)}`, shape: "triangle", act: { label: "Try again" } };
      return { word: "Idle", shape: "ring" };
    case "unknown": return { word: "Not running: watane restarted", shape: "dashed-ring", act: { label: "Continue" } };
    case "resume-lost": return { word: "Claude no longer has this session", shape: "dashed-ring", act: { label: "Continue from a summary" } };
  }
}

export function StatusLine(props: { view: SessionView; now: number; stopping: boolean; actions: Actions; onNeedsYou: () => void;
  /** How many open asks this console answered that `view` already leaves out of `openAsks` (App's `shown`). */ answered?: number }) {
  const { view, now, stopping } = props;
  const noKernel = useId();
  const s: Spec = stopping && view.turn?.state !== "idle"
    ? { word: "Stopping…", shape: "dot" }
    : spec(view, now, props.onNeedsYou, props.answered ?? 0);
  return (
    <div className="wa-status" data-state={view.turn?.state ?? "none"}>
      <span className="wa-shape" data-shape={s.shape} role="img" aria-label={NAMES[s.shape]} />
      <span role="status" className="wa-status-word">{s.word}</span>
      {s.clock ? <span className="wa-status-clock" aria-hidden="true">· {s.clock}</span> : null}
      {s.act ? (
        <button type="button" onClick={s.act.run} disabled={!s.act.run} aria-describedby={s.act.run ? undefined : noKernel}>
          {s.act.label}{s.act.key ? <> <kbd>{s.act.key}</kbd></> : null}
        </button>
      ) : null}
      {s.esc ? <span className="wa-status-esc"><kbd>Esc</kbd> to stop</span> : null}
      {s.act && !s.act.run ? <span id={noKernel} className="wa-sr">This console cannot do this yet.</span> : null}
    </div>
  );
}
