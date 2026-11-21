// TECHNICAL 18.3: turns one kernel stream message into the session view (rows and dock fields).
import { askArg, type Decision, type Internal, type Row, type SessionView, type ToolCall, type TurnState, type Ask } from "./model";
import { parse, type CallPatch, type TurnExtra } from "./wire";

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

/**
 * P14: where things are in a rows array, so one message costs the same at row 10 as at row 5 000. Kept per rows array (a WeakMap, so the
 * view stays plain data), patched as the reducer puts, pushes and splices rows, and rebuilt for a rows array it has not seen.
 * `keys` holds the first position of a row key; `calls` every position of a call id (a call inside an explored row maps to that row), since
 * two rows may hold one id (an ask's own call put on another call's row) and `find` must still reach the first that is left.
 */
interface Idx { keys: Map<string, number>; calls: Map<string, Set<number>>; writing: Set<number>; thinking: Set<number>; dup: boolean; changed: boolean }
const indexes = new WeakMap<Row[], Idx>();
let I: Idx;

function addCall(id: string, k: number) { const p = I.calls.get(id); if (p) p.add(k); else I.calls.set(id, new Set([k])); }
function dropCall(id: string, k: number) { const p = I.calls.get(id); if (p) { p.delete(k); if (!p.size) I.calls.delete(id); } }
function regRow(k: number, r: Row) {
  const have = I.keys.get(r.key);
  if (have === undefined) I.keys.set(r.key, k); else if (have !== k) I.dup = true;
  if (r.kind === "agent") { if (r.writing) I.writing.add(k); }
  else if (r.kind === "thought") { if (r.streaming) I.thinking.add(k); }
  else if (r.kind === "tool") addCall(r.call.id, k);
  else if (r.kind === "explored") for (const c of r.calls) addCall(c.id, k);
}
function unregRow(k: number, r: Row) {
  if (I.keys.get(r.key) === k) I.keys.delete(r.key);
  I.writing.delete(k); I.thinking.delete(k);
  if (r.kind === "tool") dropCall(r.call.id, k);
  else if (r.kind === "explored") for (const c of r.calls) dropCall(c.id, k);
}
function build(rows: Row[]): Idx {
  const prev = I;
  I = { keys: new Map(), calls: new Map(), writing: new Set(), thinking: new Set(), dup: false, changed: false };
  for (let k = 0; k < rows.length; k++) regRow(k, rows[k]!);
  const made = I; I = prev;
  return made;
}

/** AC3: the rows array is copied once, on the first change of a reduce; a message that changes no row keeps the parent's array. */
function own(v: SessionView) { if (!I.changed) { v.rows = v.rows.slice(); I.changed = true; } }
function put(v: SessionView, k: number, row: Row) { own(v); unregRow(k, v.rows[k]!); v.rows[k] = row; regRow(k, row); }
function push(v: SessionView, ...rows: Row[]) { own(v); for (const r of rows) { v.rows.push(r); regRow(v.rows.length - 1, r); } }
/** Replaces `del` rows at `at` with `ins`; the rows after it are registered again at their new positions. */
function splice(v: SessionView, at: number, del: number, ...ins: Row[]) {
  own(v);
  for (let k = at; k < v.rows.length; k++) unregRow(k, v.rows[k]!);
  v.rows.splice(at, del, ...ins);
  for (let k = at; k < v.rows.length; k++) regRow(k, v.rows[k]!);
}
const firstOf = (s: Set<number>) => { let m = -1; for (const k of s) if (m < 0 || k < m) m = k; return m; };
const lastOf = (s: Set<number>) => { let m = -1; for (const k of s) if (k > m) m = k; return m; };
const hasKey = (key: string) => I.keys.has(key);
const keyAt = (key: string) => I.keys.get(key) ?? -1;

function find(v: SessionView, id: string): { at: number; n?: number; call: ToolCall } | undefined {
  const p = I.calls.get(id);
  if (!p) return undefined;
  const at = firstOf(p);
  const r = v.rows[at]!;
  if (r.kind === "tool" && r.call.id === id) return { at, call: r.call };
  if (r.kind === "explored") { const n = r.calls.findIndex((c) => c.id === id); if (n >= 0) return { at, n, call: r.calls[n]! }; }
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
  splice(v, f.at, 1, ...rows);
  return f.at + (before.length ? 1 : 0);
}

function upsertCall(v: SessionView, p: CallPatch, i?: number, fromAsk = false) {
  const f = find(v, p.id);
  if (!f) {
    const call = normalize(p);
    const last = v.rows[v.rows.length - 1];
    if (isExplored(call)) {
      if (last?.kind === "explored") put(v, v.rows.length - 1, { ...explored([...last.calls, call]), key: last.key });
      else push(v, explored([call]));
    } else push(v, { kind: "tool", key: `c${call.id}`, call, ...(i !== undefined ? { i } : {}) });
    return;
  }
  const r = v.rows[f.at]!;
  // 9.8: a card with an ask shows only what the ask carries: its call replaces the streamed one whole; the stream then changes only the status,
  // and after the decision also adds program output (never title, kind, locations, rawInput or a diff)
  const ask = r.kind === "tool" ? r.ask : undefined;
  const call: ToolCall = fromAsk ? { ...normalize(p), title: p.title ?? p.kind ?? "other" }
    : ask ? held(f.call, p, !!ask.decision) : { ...f.call, ...p } as ToolCall;
  if (r.kind === "explored" && f.n !== undefined) {
    const calls = r.calls.slice(); calls[f.n] = call;
    put(v, f.at, { ...explored(calls), key: r.key });
    if (!isExplored(call)) toToolRow(v, p.id);
  } else if (r.kind === "tool") {
    { const { ended: _e, ...rest } = r; put(v, f.at, { ...rest, call, ...(i !== undefined ? { i } : {}) }); }
  }
}

function held(c: ToolCall, p: CallPatch, decided: boolean): ToolCall {
  const next: ToolCall = { ...c };
  if (p.status !== undefined) next.status = p.status;
  if (!decided) return next;
  if ("rawOutput" in p) next.rawOutput = p.rawOutput;
  if (p.content) {
    const output = p.content.filter((b) => { const t = (b as { type?: unknown } | null)?.type; return t === "content" || t === "terminal"; });
    next.content = [...c.content.filter((b) => b.type === "diff"), ...output];
  }
  return next;
}

/** The tool row of `toolCallId` shows the first open ask, else the first open check, else `decided`; `openAsks` lists the open asks (not checks). */
function syncAsks(v: SessionView, x: Internal, toolCallId: string, decided?: { ask: Ask; call: ToolCall }) {
  const open = x.open ?? [];
  v.openAsks = open.filter((o) => o.ask.kind === "ask").map((o) => o.ask.id);
  const mine = open.filter((o) => o.ask.toolCallId === toolCallId);
  const pick = mine.find((o) => o.ask.kind === "ask") ?? mine[0] ?? decided;
  const f = find(v, toolCallId);
  const r = f && f.n === undefined ? v.rows[f.at] : undefined;
  // 9.8: the card shows the call of the very entry it answers; the stream's status and program output stay
  if (pick && r?.kind === "tool") {
    const own = pick.call, cur = r.call;
    // the stream's status and output belong to the entry the card already showed; a newly shown entry starts from its own call whole
    const call: ToolCall = r.ask?.id !== pick.ask.id ? own : { ...own, status: cur.status,
      content: [...own.content.filter((b) => b.type === "diff"), ...cur.content.filter((b) => b.type !== "diff")],
      ...("rawOutput" in cur ? { rawOutput: cur.rawOutput } : {}) };
    const { checking: _c, ...rest } = r;
    const alsoChecking = pick.ask.kind === "ask" && mine.some((o) => o.ask.kind === "check");
    put(v, f!.at, { ...rest, call, ask: pick.ask, ...(alsoChecking ? { checking: true as const } : {}) });
  } else if (!pick && r?.kind === "tool" && r.ask && !r.ask.decision) {
    // an open ask moved off this call (the same id opened on another): the card keeps its call and shows no open ask
    const { checking: _c, ask: _a, ...rest } = r;
    put(v, f!.at, rest);
  }
}

/** The index of the entry each open ask came from (9.4: a short decision names its ask entry by it); kept apart from the view, which stays as it was. */
const askAt = new WeakMap<Ask, number>();
const decisionWord = (d: Decision) => `${d.outcome === "allow" ? "Allowed" : d.outcome === "deny" ? "Denied" : "No objection"}${d.for ? " for this session" : ""}`;

/** A decision entry `i` of ask `a` (which carries its decision): closes the id, shows the card of the ask's call decided, and adds the decision line. */
function decide(v: SessionView, x: Internal, i: number, a: Ask, call: CallPatch | undefined) {
  const d = a.decision!;
  const open = x.open ?? [];
  const title = call?.title ?? open.find((o) => o.ask.id === a.id)?.title ?? askArg(a.input) ?? a.toolCallId;
  // 9.4: the decision repeats the ask, so a page that starts at it (no ask of this call seen open) still draws the ask's card
  if (!open.some((o) => o.ask.toolCallId === a.toolCallId)) { upsertCall(v, call ?? { id: a.toolCallId }, undefined, true); toToolRow(v, a.toolCallId); }
  // the entry removed may be open on another call than the decision names: that card is synced too, and the decided card keeps its own call
  const gone = open.find((o) => o.ask.id === a.id);
  const own = ownCall(call, a, gone?.ask.toolCallId === a.toolCallId ? gone.call : undefined);
  x.open = open.filter((o) => o.ask.id !== a.id);
  for (const o of open) if (o.ask.id === a.id && o.ask.toolCallId !== a.toolCallId) syncAsks(v, x, o.ask.toolCallId);
  syncAsks(v, x, a.toolCallId, { ask: a, call: own });
  push(v, { kind: "decision", key: `e${i}`, i, who: d.by, what: `${decisionWord(d)}: ${title}`, ...(d.reason !== undefined ? { reason: d.reason } : {}) });
}

/** The entry's own call, as the card draws it (9.8). */
function ownCall(p: CallPatch | undefined, a: Ask, kept?: ToolCall): ToolCall {
  if (!p && kept) return kept;
  const q = p ?? { id: a.toolCallId };
  return { ...normalize(q), title: q.title ?? q.kind ?? "other" };
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

/** 9.3 R4: the turn is over, so a streamed call nothing drives (no open ask or check) is marked ended; its own data stays. */
function markEnded(v: SessionView, x: Internal) {
  const open = new Set((x.open ?? []).map((o) => o.ask.toolCallId));
  for (let k = 0; k < v.rows.length; k++) {
    const r = v.rows[k]!;
    if (r.kind === "tool" && !r.ended && (r.call.status === "pending" || r.call.status === "in_progress") && !open.has(r.call.id) && !(r.ask && !r.ask.decision))
      put(v, k, { ...r, ended: true as const });
  }
}

function setTurn(v: SessionView, x: Internal, state: TurnState, extra: TurnExtra) {
  const prev = v.turn;
  v.turn = state;
  if (state.state === "unknown" || state.state === "resume-lost" || state.state === "idle") markEnded(v, x);
  if (state.state !== "idle" || prev?.state === "idle") return;
  const key = `f:${v.rows[v.rows.length - 1]?.key ?? ""}`;
  if (hasKey(key)) return;
  push(v, { kind: "footer", key, seconds: extra.seconds ?? 0, tools: extra.tools ?? toolCount(v.rows), ...(extra.tokens !== undefined ? { tokens: extra.tokens } : {}) });
}

/** Folds the thought that is streaming, if any, at time `at` (ms). */
function fold(v: SessionView, x: Internal, at?: number) {
  const k = lastOf(I.thinking);
  if (k >= 0) {
    const r = v.rows[k] as Extract<Row, { kind: "thought" }>;
    const seconds = at !== undefined && x.thoughtAt !== undefined ? Math.round((at - x.thoughtAt) / 1000) : 0;
    put(v, k, { ...r, streaming: false, seconds });
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

/** The ask open for `id` now (not a check): the very object the card's row holds, so a caller can tell a replaced ask from the one it saw. */
export function openAsk(view: SessionView, id: string): Ask | undefined {
  return view.x?.open?.find((o) => o.ask.kind === "ask" && o.ask.id === id)?.ask;
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

/**
 * 9.4: the ask entry a short decision named, read back (`entry`, or null when it could not be read), draws the card and the decision line at the
 * placeholder's place. An entry that is not an undecided ask of the same `id` and `kind` at `askI` draws nothing: the placeholder stays as it is.
 * It touches the placeholder and the card it draws only; no ask opens or closes, so `openAsks`, the turn and a newer ask's card stay.
 */
export function settleShort(view: SessionView, i: number, entry: string | null): SessionView {
  const sh = view.x?.shorts?.find((s) => s.i === i);
  if (!sh) return view;
  const x: Internal = { ...view.x, shorts: view.x!.shorts!.filter((s) => s.i !== i) };
  let m: ReturnType<typeof parse> | undefined;
  try { if (entry !== null) m = parse(entry); } catch { m = undefined; }
  if (m?.t !== "ask" || m.i !== sh.askI || m.ask.id !== sh.id || m.ask.kind !== sh.kind || m.ask.decision || !view.rows.some((r) => r.key === `e${i}` && r.kind === "decision")) return { ...view, x };
  let idx = indexes.get(view.rows);
  if (!idx) { idx = build(view.rows); indexes.set(view.rows, idx); }
  I = idx;
  indexes.delete(view.rows);
  I.changed = false; I.dup = false;
  const v: SessionView = { ...view, x };
  const a = m.ask;
  const decided: Ask = { ...a, decision: sh.decision };
  if (!(x.open ?? []).some((o) => o.ask.toolCallId === a.toolCallId)) {
    const had = find(v, a.toolCallId) !== undefined;
    upsertCall(v, m.call ?? { id: a.toolCallId }, undefined, true);
    toToolRow(v, a.toolCallId);
    syncAsks(v, x, a.toolCallId, { ask: decided, call: ownCall(m.call, a) });
    const at = find(v, a.toolCallId)?.at ?? -1, k = keyAt(`e${i}`);
    // a card the page did not hold stands at the placeholder's place
    if (!had && at > k && k >= 0) { const row = v.rows[at]!; splice(v, at, 1); splice(v, k, 0, row); }
  }
  const k = keyAt(`e${i}`), row = v.rows[k];
  if (row?.kind === "decision") put(v, k, { ...row, what: `${decisionWord(sh.decision)}: ${m.call?.title ?? askArg(a.input) ?? a.toolCallId}` });
  if (I.dup) { v.rows = uniqueKeys(v.rows); I = build(v.rows); }
  indexes.set(v.rows, I);
  return v;
}

export function reduce(view: SessionView, message: string): SessionView {
  const m = parse(message);
  const i = "i" in m ? m.i : undefined;
  let idx = indexes.get(view.rows);
  if (!idx) { idx = build(view.rows); indexes.set(view.rows, idx); }
  I = idx;
  const refetch = m.t === "message" && hasKey(`~p${m.i}`);
  if (m.t !== "synced" && i !== undefined && i <= view.lastIndex && !refetch) return view;
  indexes.delete(view.rows); // the index now follows the new rows; a reduce that throws leaves none, and the next one rebuilds
  I.changed = false; I.dup = false;
  const x: Internal = { ...view.x };
  const v: SessionView = { ...view, x }; // rows: copied by `own` on the first change

  if (m.t === "chunk" || m.t === "call" || m.t === "plan") {
    if (!(m.t === "chunk" && m.part === "thought")) fold(v, x, m.at);
  } else if (m.t !== "synced" && m.t !== "queue" && m.t !== "usage" && m.t !== "turn") fold(v, x);

  switch (m.t) {
    case "synced":
      v.synced = true; v.lastIndex = Math.max(v.lastIndex, m.through);
      // 9.4 no partial replay: a message being written at attach shows as "Writing…" until its entry
      x.skip = m.writing;
      if (m.writing) {
        const k = firstOf(I.writing);
        const row: Row = { kind: "agent", key: "~w", text: "", writing: true };
        if (k >= 0) put(v, k, row); else push(v, row);
      }
      break;
    case "queue": v.queue = m.inputs; break;
    case "usage": v.usage = { used: m.used, window: m.window, ...(m.compactAt !== undefined ? { compactAt: m.compactAt } : {}) }; break;
    case "turn": setTurn(v, x, m.state, m.extra); break;
    case "compaction": break;
    case "unreadable":
      push(v, { kind: "unknown", key: i !== undefined ? `e${i}` : nextKey(x, "u"), ...(i !== undefined ? { i } : {}), raw: m.raw });
      break;
    case "message": {
      if (m.role === "user") {
        const row: Row = { kind: "user", key: `e${m.i}`, i: m.i, text: m.text };
        const last = v.rows[v.rows.length - 1];
        const p = keyAt(`~p${m.i}`);
        if (p >= 0) put(v, p, row);
        else if (last?.kind === "user" && last.key.startsWith("~")) put(v, v.rows.length - 1, row); else push(v, row);
        break;
      }
      const rows: Row[] = [];
      if (m.thought) rows.push({ kind: "thought", key: `e${m.i}-t`, i: m.i, text: m.thought.text, streaming: false, seconds: m.thought.seconds });
      rows.push({ kind: "agent", key: `e${m.i}`, i: m.i, text: m.text, writing: false });
      let w = keyAt(`~p${m.i}`);
      if (w < 0) w = firstOf(I.writing);
      if (w > 0 && v.rows[w - 1]!.kind === "thought" && v.rows[w - 1]!.key.startsWith("~")) { w--; splice(v, w, 2, ...rows); }
      else if (w >= 0) splice(v, w, 1, ...rows); else push(v, ...rows);
      break;
    }
    case "tool": upsertCall(v, m.call, m.i); break;
    case "short": {
      // 9.4: the decision closes the id at once; with the ask entry at `askI` open in the view it is exactly the full decision for that ask
      const held = (x.open ?? []).find((o) => o.ask.id === m.id && o.ask.kind === m.kind && askAt.get(o.ask) === m.askI);
      if (held) { decide(v, x, m.i, { ...held.ask, decision: m.decision }, undefined); break; }
      const open = x.open ?? [];
      x.open = open.filter((o) => o.ask.id !== m.id);
      for (const o of open) if (o.ask.id === m.id) syncAsks(v, x, o.ask.toolCallId);
      x.shorts = [...(x.shorts ?? []), { i: m.i, askI: m.askI, id: m.id, kind: m.kind, decision: m.decision }];
      push(v, { kind: "decision", key: `e${m.i}`, i: m.i, who: m.decision.by, what: `${decisionWord(m.decision)}: ${m.id}`, ...(m.decision.reason !== undefined ? { reason: m.decision.reason } : {}) });
      break;
    }
    case "ask": {
      const a = m.ask, d = a.decision;
      const open = x.open ?? [];
      if (d) { decide(v, x, m.i, a, m.call); break; }
      // 9.8: the card is the ask's own call, whatever the stream said of the id before (an entry already open on the call keeps its card)
      if (!open.some((o) => o.ask.toolCallId === a.toolCallId && o.ask.id !== a.id)) upsertCall(v, m.call ?? { id: a.toolCallId }, undefined, true);
      toToolRow(v, a.toolCallId);
      const moved = open.find((o) => o.ask.id === a.id && o.ask.toolCallId !== a.toolCallId)?.ask.toolCallId;
      askAt.set(a, m.i);
      x.open = [...open.filter((o) => o.ask.id !== a.id), { ask: a, title: m.call?.title ?? askArg(a.input) ?? a.toolCallId, call: ownCall(m.call, a) }];
      if (moved !== undefined) syncAsks(v, x, moved);
      syncAsks(v, x, a.toolCallId);
      break;
    }
    case "allow":
      v.allows = v.allows.filter((a) => a.id !== m.id);
      if (m.given) v.allows.push({ id: m.id, reach: m.reach });
      push(v, { kind: "decision", key: `e${m.i}`, i: m.i, who: m.by, what: m.given ? `Allowed ${m.reach} for this session` : `Took back ${m.reach}` });
      break;
    case "verdict":
      push(v, { kind: "decision", key: `e${m.i}`, i: m.i, who: m.by, what: `${m.outcome}: ${m.toolCallId === undefined ? "the input" : toolTitle(v, m.toolCallId)}`, ...(m.reasons.length ? { reason: m.reasons.join("; ") } : {}) });
      break;
    case "plan": v.plan = m.items; break;
    case "call": upsertCall(v, m.call, m.i); break;
    case "chunk": {
      const last = v.rows[v.rows.length - 1];
      if (x.skip && m.part !== "user") {
        // a delta of the message being written at attach: dropped; its finishing update fetches the entry
        if (m.part === "agent" && m.i !== undefined) {
          const k = keyAt("~w");
          if (k >= 0) put(v, k, { kind: "agent", key: `~p${m.i}`, text: "", writing: true });
          x.skip = false;
        }
        break;
      }
      if (m.part === "thought") {
        if (last?.kind === "thought" && last.streaming) put(v, v.rows.length - 1, { ...last, text: last.text + m.text });
        else { push(v, { kind: "thought", key: nextKey(x, "~t"), text: m.text, streaming: true }); x.thoughtAt = m.at; }
        break;
      }
      if (m.part === "user") {
        if (last?.kind === "user" && last.key.startsWith("~")) put(v, v.rows.length - 1, { ...last, text: last.text + m.text });
        else push(v, { kind: "user", key: nextKey(x, "~u"), text: m.text });
        if (m.i !== undefined) finish(v, m.i);
        break;
      }
      if (last?.kind === "agent" && last.writing) put(v, v.rows.length - 1, { ...last, text: last.text + m.text });
      else push(v, { kind: "agent", key: nextKey(x, "~a"), text: m.text, writing: true });
      if (m.i !== undefined) finish(v, m.i);
      break;
    }
  }
  if (i !== undefined) v.lastIndex = Math.max(v.lastIndex, i);
  if (I.dup) { v.rows = uniqueKeys(v.rows); I = build(v.rows); }
  indexes.set(v.rows, I);
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
    put(v, k, { ...r, key: `e${i}`, i, writing: false });
    const t = v.rows[k - 1];
    if (t?.kind === "thought" && t.key.startsWith("~")) put(v, k - 1, { ...t, key: `e${i}-t`, i });
  } else if (r.kind === "user") put(v, k, { ...r, key: `e${i}`, i });
}
