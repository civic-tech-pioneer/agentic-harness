// TECHNICAL 18.5: the status line, one shape per state; only the word and its subject are live.
import { useId } from "react";
import { cardName, SESSION_IN_ANOTHER_TAB, type Actions, Row, SessionView } from "./model";
import { openAsk } from "./reduce";
import { answeredWords } from "./parts/askbar";

type Shape = "dot" | "square" | "diamond" | "bars" | "ring" | "dashed-ring" | "triangle" | "loading";
const NAMES: Record<Shape, string> = { dot: "Running", square: "Checking", diamond: "Waiting for you", bars: "Paused", ring: "Idle", "dashed-ring": "Not running", triangle: "Failed", loading: "Loading" };
/** An act with no `run` has no kernel call yet: it shows disabled, with a description saying so. */
interface Spec { word: string; shape: Shape; clock?: string; esc?: boolean; act?: { label: string; key?: string; run?: () => void; off?: boolean } }

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

/** Host 19.7 includes the provider identity. Older records carry just the absolute folder. A control character in it shows
 * as its picture (U+2400 to U+2421), so a newline or a tab reads as one and is never collapsed to a space (S140). */
function workspaceFolder(what: string): string | undefined {
  return what.match(/^workspace:(?:[^:/][^:]*:)?(\/.*)$/s)?.[1]
    ?.replace(/[\u0000-\u001f\u007f]/g, (c) => String.fromCharCode(c === "\u007f" ? 0x2421 : 0x2400 + c.charCodeAt(0)));
}

/** 18.5: put known provider outcomes in participant-facing words. */
function failedWords(stop: { error: string; cause?: string }): string {
  const reason = stop.cause ?? stop.error;
  if (reason === "no-binary") return "Choose a Claude Code binary in Settings → Claude Code";
  if (stop.error === "judge" && stop.cause === undefined) return "your judge denied it";
  const refused = reason.match(/^consent-refused\((.*)\)$/s);
  const folder = refused ? workspaceFolder(refused[1]!) : undefined;
  if (folder) return `Workspace access was refused for ${folder}`;
  if (refused?.[1] === "account:openai") return "Consent was refused for the OpenAI account";
  if (refused) return `Consent was refused for ${refused[1]}`;
  const version = reason.match(/^version (\S+) < (\S+)$/);
  if (version) return `Claude Code ${version[1]} is too old. Version ${version[2]} or later is required.`;
  if (stop.error !== "provider-ended") return reason;
  if (stop.cause === undefined || reason === "gone" || reason.startsWith("failed: ")) return "Connection to Claude Code ended";
  const mismatch = reason.match(/^mismatch\((.*)\)$/s);
  if (mismatch) return `Claude Code ${mismatch[1]} did not match the launch`;
  if (reason === "no-app") return "The app is unavailable";
  if (reason === "no-account") return "The required account is unavailable";
  const outcomes: Record<string, string> = {
    "idle-timeout": "The connection timed out while idle",
    lagging: "The connection fell behind",
    "consumer-failed": "The session reader failed",
    cancelled: "The session was cancelled",
    replaced: "The session connection was replaced",
  };
  return outcomes[reason] ?? reason;
}

function spec(view: SessionView, now: number, answered: number, actions: Actions): Spec {
  const t = view.turn;
  if (!t) return view.loading ? { word: "Loading", shape: "loading" } : { word: "Idle", shape: "ring" };
  switch (t.state) {
    case "running": {
      const clk = elapsed(now - t.since);
      const a = t.activity;
      if (a?.kind === "checking") return { word: "Checking with your judge", shape: "square", clock: clk, esc: true };
      if (a?.kind === "compacting") return { word: "Making room", shape: "dot", clock: clk, esc: true };
      if (a?.kind === "consent") {
        const folder = workspaceFolder(a.what);
        const word = folder ? `Waiting for your answer in watane for ${folder}` : "Waiting for your answer in watane";
        return { word, shape: "diamond", esc: true, act: { label: "Open the question" } };
      }
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
    // 18.5, 9.3 R5: Continue is `say` with the text `Continue`; a refusal leaves the status as it is, so she can try again
    case "unknown": return { word: "Not running: watane restarted", shape: "dashed-ring", act: { label: "Continue", off: !!actions.off, run: () => { try { void Promise.resolve(actions.say("Continue")).catch(() => {}); } catch { /* refused: the status stays */ } } } };
    case "resume-lost": return { word: "Claude no longer has this session", shape: "dashed-ring", act: { label: "Continue from a summary" } };
  }
}

export function StatusLine(props: { view: SessionView; now: number; stopping: boolean; actions: Actions; onNeedsYou: () => void;
  /** How many open asks this console answered that `view` already leaves out of `openAsks` (App's `shown`). */ answered?: number }) {
  const { view, now, stopping } = props;
  const reasonId = useId();
  const s: Spec = stopping && view.turn?.state !== "idle"
    ? { word: "Stopping…", shape: "dot" }
    : spec(view, now, props.answered ?? 0, props.actions);
  const disabledReason = !s.act ? undefined : !s.act.run ? "This console cannot do this yet."
    : s.act.off ? SESSION_IN_ANOTHER_TAB : undefined;
  const sharedReasonId = s.act?.run && s.act.off ? props.actions.offReasonId : undefined;
  return (
    <div className="wa-status" data-state={view.turn?.state ?? "none"}>
      <span className="wa-status-label">
        <span className="wa-shape" data-shape={s.shape} role="img" aria-label={NAMES[s.shape]} />
        <span role="status" className="wa-status-word">{s.word}</span>
      </span>
      {s.clock ? <span className="wa-status-clock" aria-hidden="true">· {s.clock}</span> : null}
      {s.act ? (
        <button type="button" onClick={s.act.run} disabled={!!disabledReason} aria-describedby={disabledReason ? sharedReasonId ?? reasonId : undefined}>
          {s.act.label}{s.act.key ? <> <kbd>{s.act.key}</kbd></> : null}
        </button>
      ) : null}
      {disabledReason && !sharedReasonId ? <span id={reasonId}>{disabledReason}</span> : null}
      {s.esc && !props.actions.off ? <span className="wa-status-esc"><kbd>Esc</kbd> to stop</span> : null}
    </div>
  );
}
