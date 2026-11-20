// TECHNICAL 18.3: turns one kernel stream message into the session view (rows and dock fields).
import type { Ask, Internal, Row, SessionView, ToolCall, TurnState } from "./model";
import { parse, type CallPatch, type TurnExtra } from "./wire";

type ToolRow = Extract<Row, { kind: "tool" }>;
type ExploredRow = Extract<Row, { kind: "explored" }>;
const isExplored = (c: ToolCall) => (c.kind === "read" || c.kind === "search") && c.status !== "failed";

function explored(calls: ToolCall[]): ExploredRow {
  return {
    kind: "explored", key: `x${calls[0]!.id}`, calls,
    reads: calls.filter((c) => c.kind === "read").length,
    searches: calls.filter((c) => c.kind === "search").length,
  };
}

function normalize(p: CallPatch): ToolCall {
  return { id: p.id, title: p.title ?? p.id, kind: p.kind ?? "other", status: p.status ?? "pending", content: p.content ?? [], locations: p.locations ?? [],
    ...("rawInput" in p ? { rawInput: p.rawInput } : {}), ...("rawOutput" in p ? { rawOutput: p.rawOutput } : {}) };
}

function find(v: SessionView, id: string): { at: number; n?: number; call: ToolCall } | undefined {
  for (let at = 0; at < v.rows.length; at++) {
    const r = v.rows[at]!;
    if (r.kind === "tool" && r.call.id === id) return { at, call: r.call };
    if (r.kind === "explored") { const n = r.calls.findIndex((c) => c.id === id); if (n >= 0) return { at, n, call: r.calls[n]! }; }
  }
  return undefined;
}

/** Makes the call a `tool` row (splitting an explored row around it) and returns its index. */
function toToolRow(v: SessionView, id: string): number {
  const f = find(v, id)!;
  if (f.n === undefined) return f.at;
  const r = v.rows[f.at] as ExploredRow;
  const before = r.calls.slice(0, f.n), after = r.calls.slice(f.n + 1);
  const rows: Row[] = [];
  if (before.length) rows.push({ ...explored(before), key: r.key });
  rows.push({ kind: "tool", key: `c${id}`, call: f.call });
  if (after.length) rows.push(explored(after));
  v.rows.splice(f.at, 1, ...rows);
  return f.at + (before.length ? 1 : 0);
}

function upsertCall(v: SessionView, p: CallPatch, i?: number) {
  const f = find(v, p.id);
  if (!f) {
    const call = normalize(p);
    const last = v.rows[v.rows.length - 1];
    if (isExplored(call)) {
      if (last?.kind === "explored") v.rows[v.rows.length - 1] = { ...explored([...last.calls, call]), key: last.key };
      else v.rows.push(explored([call]));
    } else v.rows.push({ kind: "tool", key: `c${call.id}`, call, ...(i !== undefined ? { i } : {}) });
    return;
  }
  const call: ToolCall = { ...f.call, ...p } as ToolCall;
  const r = v.rows[f.at]!;
  if (r.kind === "explored" && f.n !== undefined) {
    const calls = r.calls.slice(); calls[f.n] = call;
    v.rows[f.at] = { ...explored(calls), key: r.key };
    if (!isExplored(call)) toToolRow(v, p.id);
  } else if (r.kind === "tool") {
    v.rows[f.at] = { ...r, call, ...(i !== undefined ? { i } : {}) };
  }
}

function removeAsk(v: SessionView, id: string) {
  v.openAsks = v.openAsks.filter((a) => a !== id);
  v.rows = v.rows.map((r) => {
    if (r.kind === "tool" && r.ask?.toolCallId === id) { const { ask: _a, ...rest } = r; return rest; }
    return r;
  });
}

function toolCount(rows: Row[]): number {
  let n = 0;
  for (let k = rows.length - 1; k >= 0; k--) {
    const r = rows[k]!;
    if (r.kind === "user") break;
    if (r.kind === "explored") n += r.calls.length; else if (r.kind === "tool") n++;
  }
  return n;
}

function setTurn(v: SessionView, x: Internal, state: TurnState, extra: TurnExtra) {
  const prev = v.turn;
  v.turn = state;
  if (state.state === "running") x.lost = false;
  if (state.state !== "idle" || prev?.state === "idle") return;
  const key = `f:${v.rows[v.rows.length - 1]?.key ?? ""}`;
  if (v.rows.some((r) => r.key === key)) return;
  v.rows.push({ kind: "footer", key, seconds: extra.seconds ?? 0, tools: extra.tools ?? toolCount(v.rows), ...(extra.tokens !== undefined ? { tokens: extra.tokens } : {}) });
}

/** Folds the thought that is streaming, if any, at time `at` (ms). */
function fold(v: SessionView, x: Internal, at?: number) {
  for (let k = v.rows.length - 1; k >= 0; k--) {
    const r = v.rows[k]!;
    if (r.kind === "thought" && r.streaming) {
      const seconds = at !== undefined && x.thoughtAt !== undefined ? Math.round((at - x.thoughtAt) / 1000) : 0;
      v.rows[k] = { ...r, streaming: false, seconds };
      break;
    }
  }
  x.thoughtAt = undefined;
}

function toolTitle(v: SessionView, id: string): string {
  return find(v, id)?.call.title ?? id;
}

export function reduce(view: SessionView, message: string): SessionView {
  const m = parse(message);
  const i = "i" in m ? m.i : undefined;
  if (m.t !== "synced" && i !== undefined && i <= view.lastIndex) return view;
  const x: Internal = { lost: false, ...view.x };
  const v: SessionView = { ...view, rows: view.rows.slice(), x };
  let advance = true;

  if (m.t === "chunk" || m.t === "call" || m.t === "plan") {
    if (!(m.t === "chunk" && m.part === "thought")) fold(v, x, m.at);
  } else if (m.t !== "synced" && m.t !== "queue" && m.t !== "usage" && m.t !== "turn") fold(v, x);

  switch (m.t) {
    case "synced":
      v.synced = true; v.lastIndex = Math.max(v.lastIndex, m.through); x.lost = true;
      break;
    case "queue": v.queue = m.inputs; break;
    case "usage": v.usage = { used: m.used, window: m.window, ...(m.compactAt !== undefined ? { compactAt: m.compactAt } : {}) }; break;
    case "turn": setTurn(v, x, m.state, m.extra); break;
    case "compaction": break;
    case "unreadable":
      v.rows.push({ kind: "unknown", key: i !== undefined ? `e${i}` : `u${v.rows.length}`, ...(i !== undefined ? { i } : {}), raw: m.raw });
      break;
    case "message": {
      x.lost = false;
      if (m.role === "user") {
        const row: Row = { kind: "user", key: `e${m.i}`, i: m.i, text: m.text };
        const last = v.rows[v.rows.length - 1];
        if (last?.kind === "user" && last.key.startsWith("~")) v.rows[v.rows.length - 1] = row; else v.rows.push(row);
        break;
      }
      const rows: Row[] = [];
      if (m.thought) rows.push({ kind: "thought", key: `e${m.i}-t`, i: m.i, text: m.thought.text, streaming: false, seconds: m.thought.seconds });
      rows.push({ kind: "agent", key: `e${m.i}`, i: m.i, text: m.text, writing: false });
      const w = v.rows.findIndex((r) => r.kind === "agent" && r.writing);
      if (w >= 0) v.rows.splice(w, 1, ...rows); else v.rows.push(...rows);
      break;
    }
    case "tool": upsertCall(v, m.call, m.i); break;
    case "ask": {
      const d = m.ask.decision;
      if (d) {
        removeAsk(v, m.ask.toolCallId);
        v.rows.push({ kind: "decision", key: `e${m.i}`, i: m.i, who: d.by,
          what: `${d.outcome === "allow" ? "Allowed" : "Denied"}${d.for ? " for this session" : ""}: ${toolTitle(v, m.ask.toolCallId)}`,
          ...(d.reason !== undefined ? { reason: d.reason } : {}) });
        break;
      }
      if (!find(v, m.ask.toolCallId)) {
        const inp = (m.ask.input ?? {}) as Record<string, unknown>;
        const title = [inp.title, inp.command, inp.file_path, inp.path].find((s) => typeof s === "string") as string | undefined;
        v.rows.push({ kind: "tool", key: `c${m.ask.toolCallId}`, call: normalize({ id: m.ask.toolCallId, ...(title ? { title } : {}) }) });
      }
      const at = toToolRow(v, m.ask.toolCallId);
      const row = v.rows[at] as ToolRow;
      const ask: Ask = m.ask;
      v.rows[at] = { ...row, ask };
      if (!v.openAsks.includes(ask.toolCallId)) v.openAsks = [...v.openAsks, ask.toolCallId];
      break;
    }
    case "allow":
      v.allows = v.allows.filter((a) => a.id !== m.id);
      if (m.given) v.allows.push({ id: m.id, reach: m.reach });
      v.rows.push({ kind: "decision", key: `e${m.i}`, i: m.i, who: m.by, what: m.given ? `Allowed ${m.reach} for this session` : `Took back ${m.reach}` });
      break;
    case "verdict":
      v.rows.push({ kind: "decision", key: `e${m.i}`, i: m.i, who: m.by, what: `${m.outcome}: ${toolTitle(v, m.toolCallId)}`, ...(m.reasons.length ? { reason: m.reasons.join("; ") } : {}) });
      break;
    case "plan": v.plan = m.items; break;
    case "call": upsertCall(v, m.call, m.i); x.lost = false; break;
    case "chunk": {
      const last = v.rows[v.rows.length - 1];
      if (m.part === "thought") {
        if (x.lost) break;
        if (last?.kind === "thought" && last.streaming) v.rows[v.rows.length - 1] = { ...last, text: last.text + m.text };
        else { v.rows.push({ kind: "thought", key: `~t${v.rows.length}`, text: m.text, streaming: true }); x.thoughtAt = m.at; }
        break;
      }
      if (m.part === "user") {
        x.lost = false;
        if (last?.kind === "user" && last.key.startsWith("~")) v.rows[v.rows.length - 1] = { ...last, text: last.text + m.text };
        else v.rows.push({ kind: "user", key: `~u${v.rows.length}`, text: m.text });
        if (m.i !== undefined) finish(v, m.i);
        break;
      }
      if (x.lost) {
        if (!v.rows.some((r) => r.key === "~w")) v.rows.push({ kind: "agent", key: "~w", text: "", writing: true });
        if (m.i !== undefined) { x.lost = false; advance = false; }
        break;
      }
      if (last?.kind === "agent" && last.writing && last.key !== "~w") v.rows[v.rows.length - 1] = { ...last, text: last.text + m.text };
      else v.rows.push({ kind: "agent", key: `~a${v.rows.length}`, text: m.text, writing: true });
      if (m.i !== undefined) finish(v, m.i);
      break;
    }
  }
  if (advance && i !== undefined) v.lastIndex = Math.max(v.lastIndex, i);
  return v;
}

/** The streaming row at the end of the rows becomes the entry `i`; a folded thought before it too. */
function finish(v: SessionView, i: number) {
  const k = v.rows.length - 1;
  const r = v.rows[k]!;
  if (r.kind === "agent") {
    v.rows[k] = { ...r, key: `e${i}`, i, writing: false };
    const t = v.rows[k - 1];
    if (t?.kind === "thought" && t.key.startsWith("~")) v.rows[k - 1] = { ...t, key: `e${i}-t`, i };
  } else if (r.kind === "user") v.rows[k] = { ...r, key: `e${i}`, i };
}
