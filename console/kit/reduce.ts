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

/** A key no other row has: a counter kept in the view's internal `x`, not the row count. */
function nextKey(x: Internal, prefix: string): string {
  const n = x.seq ?? 0;
  x.seq = n + 1;
  return `${prefix}${n}`;
}

/** The view with one more `unknown` row showing `raw`. */
export function withUnknown(view: SessionView, raw: unknown): SessionView {
  const x: Internal = { ...view.x };
  return { ...view, rows: [...view.rows, { kind: "unknown", key: nextKey(x, "u"), raw }], x };
}

/** `reduce` that cannot throw: a message it chokes on becomes an `unknown` row. */
export function safeReduce(view: SessionView, message: string): SessionView {
  try { return reduce(view, message); } catch { return withUnknown(view, message); }
}

/** 9.4: the entry `n` read back for a `~p<n>` row replaces the row with what the entry is (a message, an `unknown` row, or the "could not be read" row for `null`); no `~p` row is left. */
export function settle(view: SessionView, n: number, entry: string | null): SessionView {
  const key = `~p${n}`;
  if (!view.rows.some((r) => r.key === key)) return view;
  const base = entry === null ? view : safeReduce(view, entry);
  const k = base.rows.findIndex((r) => r.key === key);
  if (k < 0) return base;
  const appended = base.rows.slice(view.rows.length);
  if (appended.length) {
    const rows = base.rows.slice(0, view.rows.length);
    rows.splice(k, 1, ...appended);
    return { ...base, rows: uniqueKeys(rows) };
  }
  let raw: unknown = "could not be read";
  if (entry !== null) { try { raw = JSON.parse(entry); } catch { raw = entry; } }
  const rows = view.rows.slice();
  rows.splice(view.rows.findIndex((r) => r.key === key), 1, { kind: "unknown", key: `e${n}`, i: n, raw });
  return { ...view, rows: uniqueKeys(rows) };
}

export function reduce(view: SessionView, message: string): SessionView {
  const m = parse(message);
  const i = "i" in m ? m.i : undefined;
  const refetch = m.t === "message" && view.rows.some((r) => r.key === `~p${m.i}`);
  if (m.t !== "synced" && i !== undefined && i <= view.lastIndex && !refetch) return view;
  const x: Internal = { ...view.x };
  const v: SessionView = { ...view, rows: view.rows.slice(), x };

  if (m.t === "chunk" || m.t === "call" || m.t === "plan") {
    if (!(m.t === "chunk" && m.part === "thought")) fold(v, x, m.at);
  } else if (m.t !== "synced" && m.t !== "queue" && m.t !== "usage" && m.t !== "turn") fold(v, x);

  switch (m.t) {
    case "synced":
      v.synced = true; v.lastIndex = Math.max(v.lastIndex, m.through);
      // 9.4 no partial replay: a message being written at attach shows as "Writing…" until its entry
      x.skip = m.writing;
      if (m.writing) {
        const k = v.rows.findIndex((r) => r.kind === "agent" && r.writing);
        const row: Row = { kind: "agent", key: "~w", text: "", writing: true };
        if (k >= 0) v.rows[k] = row; else v.rows.push(row);
      }
      break;
    case "queue": v.queue = m.inputs; break;
    case "usage": v.usage = { used: m.used, window: m.window, ...(m.compactAt !== undefined ? { compactAt: m.compactAt } : {}) }; break;
    case "turn": setTurn(v, x, m.state, m.extra); break;
    case "compaction": break;
    case "unreadable":
      v.rows.push({ kind: "unknown", key: i !== undefined ? `e${i}` : nextKey(x, "u"), ...(i !== undefined ? { i } : {}), raw: m.raw });
      break;
    case "message": {
      if (m.role === "user") {
        const row: Row = { kind: "user", key: `e${m.i}`, i: m.i, text: m.text };
        const last = v.rows[v.rows.length - 1];
        const p = v.rows.findIndex((r) => r.key === `~p${m.i}`);
        if (p >= 0) v.rows[p] = row;
        else if (last?.kind === "user" && last.key.startsWith("~")) v.rows[v.rows.length - 1] = row; else v.rows.push(row);
        break;
      }
      const rows: Row[] = [];
      if (m.thought) rows.push({ kind: "thought", key: `e${m.i}-t`, i: m.i, text: m.thought.text, streaming: false, seconds: m.thought.seconds });
      rows.push({ kind: "agent", key: `e${m.i}`, i: m.i, text: m.text, writing: false });
      let w = v.rows.findIndex((r) => r.key === `~p${m.i}`);
      if (w < 0) w = v.rows.findIndex((r) => r.kind === "agent" && r.writing);
      if (w > 0 && v.rows[w - 1]!.kind === "thought" && v.rows[w - 1]!.key.startsWith("~")) { w--; v.rows.splice(w, 2, ...rows); }
      else if (w >= 0) v.rows.splice(w, 1, ...rows); else v.rows.push(...rows);
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
      // the ask's own call is the one that will run: it wins over what the stream left in the row
      if (!find(v, m.ask.toolCallId) || m.call) upsertCall(v, m.call ?? { id: m.ask.toolCallId });
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
    case "call": upsertCall(v, m.call, m.i); break;
    case "chunk": {
      const last = v.rows[v.rows.length - 1];
      if (x.skip && m.part !== "user") {
        // a delta of the message being written at attach: dropped; its finishing update fetches the entry
        if (m.part === "agent" && m.i !== undefined) {
          const k = v.rows.findIndex((r) => r.key === "~w");
          if (k >= 0) v.rows[k] = { kind: "agent", key: `~p${m.i}`, text: "", writing: true };
          x.skip = false;
        }
        break;
      }
      if (m.part === "thought") {
        if (last?.kind === "thought" && last.streaming) v.rows[v.rows.length - 1] = { ...last, text: last.text + m.text };
        else { v.rows.push({ kind: "thought", key: nextKey(x, "~t"), text: m.text, streaming: true }); x.thoughtAt = m.at; }
        break;
      }
      if (m.part === "user") {
        if (last?.kind === "user" && last.key.startsWith("~")) v.rows[v.rows.length - 1] = { ...last, text: last.text + m.text };
        else v.rows.push({ kind: "user", key: nextKey(x, "~u"), text: m.text });
        if (m.i !== undefined) finish(v, m.i);
        break;
      }
      if (last?.kind === "agent" && last.writing) v.rows[v.rows.length - 1] = { ...last, text: last.text + m.text };
      else v.rows.push({ kind: "agent", key: nextKey(x, "~a"), text: m.text, writing: true });
      if (m.i !== undefined) finish(v, m.i);
      break;
    }
  }
  if (i !== undefined) v.lastIndex = Math.max(v.lastIndex, i);
  v.rows = uniqueKeys(v.rows);
  return v;
}

/** 18.3 invariant: no key appears twice; the first row of a key stays. */
function uniqueKeys(rows: Row[]): Row[] {
  const seen = new Set<string>();
  const out = rows.filter((r) => !seen.has(r.key) && seen.add(r.key));
  return out.length === rows.length ? rows : out;
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
