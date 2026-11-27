// @vitest-environment node
// S7-fix1, TECHNICAL 14 "An ask shows the tool call that will run": the reducer's row index never points a message at another call's
// row. Random message sequences (a few call ids and entry indexes, so calls share ids, explored rows split, keys collide and `~p` rows
// settle) reduced by the new reducer and by the frozen one of 5882c51 give the same view after every step; sometimes a sequence goes
// on from an earlier view (a parent reduced twice, as React may do), which the new reducer must handle as a rows array it has not seen.
import { expect, it } from "vitest";
import { emptyView, type SessionView } from "./model";
import { safeReduce, settle } from "./reduce";
import { safeReduce as oldSafeReduce, settle as oldSettle } from "./fixtures/reduce_5882c51";

function rng(seed: number) {
  let s = seed >>> 0;
  return () => { s = (s + 0x6d2b79f5) >>> 0; let t = s; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}

function messages(r: () => number) {
  const pick = <T,>(a: readonly T[]): T => a[Math.floor(r() * a.length)]!;
  const j = JSON.stringify;
  const id = () => pick(["c0", "c1", "c2", "c3"]);
  const kind = () => pick(["read", "search", "edit", "execute"]);
  const status = () => pick(["pending", "in_progress", "completed", "failed"]);
  const content = () => (r() < 0.3 ? [{ type: "diff", path: "a.ts", oldText: "a", newText: `b${Math.floor(r() * 9)}` }] : r() < 0.5 ? [{ type: "content", content: { type: "text", text: "out" } }] : undefined);
  const callBody = (cid: string) => ({ toolCallId: cid, ...(r() < 0.7 ? { title: `T ${cid} ${Math.floor(r() * 5)}` } : {}), ...(r() < 0.8 ? { kind: kind() } : {}), ...(r() < 0.8 ? { status: status() } : {}), ...(content() ? { content: content() } : {}) });
  let n = 1;
  const i = () => (r() < 0.75 ? n++ : Math.max(1, n - 1 - Math.floor(r() * 4)));
  const meta = () => { const m: Record<string, number> = {}; if (r() < 0.4) m.i = i(); if (r() < 0.5) m.at = Math.floor(r() * 1e6); return Object.keys(m).length ? { _meta: { watane: m } } : {}; };
  const entry = (k: number, kindName: string, body: object) => j({ _meta: { watane: { type: "entry", entry: { v: 1, i: k, kind: kindName, at: 0, body } } } });
  const text = () => ({ type: "text", text: pick(["a ", "b ", "c "]) });
  const turn = () => pick([{ state: "running", since: 0 }, { state: "idle", stop: { kind: "completed" }, ...(r() < 0.5 ? { tools: 2 } : {}) }, { state: "unknown" }, { state: "resume-lost" }, { state: "waiting", asks: ["a0"], "expires-at": 9 }]);
  const one = (): string => {
    const x = r();
    if (x < 0.12) return j({ sessionUpdate: pick(["agent_message_chunk", "agent_thought_chunk", "user_message_chunk"]), content: text(), ...meta() });
    if (x < 0.30) return j({ sessionUpdate: pick(["tool_call", "tool_call_update"]), ...callBody(id()), ...meta() });
    if (x < 0.40) return entry(i(), "message", { role: pick(["user", "agent"]), content: text(), ...(r() < 0.3 ? { thought: { text: "t", seconds: 2 } } : {}) });
    if (x < 0.48) return entry(i(), "tool", callBody(id()));
    if (x < 0.65) {
      const cid = id(), k = pick(["ask", "check"]);
      const decided = r() < 0.5 ? { decision: { outcome: pick(["allow", "deny", "none"]), by: "you", ...(r() < 0.3 ? { for: "session" } : {}) } } : {};
      return entry(i(), "ask", { id: pick(["a0", "a1", "a2"]), kind: k, toolCallId: cid, input: {}, "expires-at": 9, ...(k === "ask" ? { reach: "r" } : {}), ...(r() < 0.8 ? { toolCall: callBody(pick([cid, "zz"])) } : {}), ...decided });
    }
    if (x < 0.69) return entry(i(), "verdict", { ...(r() < 0.7 ? { toolCallId: id() } : { on: "input" }), outcome: "deny", by: "rule", reasons: ["x"] });
    if (x < 0.71) return entry(i(), "allow", { id: pick(["p0", "p1"]), reach: "r", by: "you", given: r() < 0.5 });
    if (x < 0.78) return r() < 0.5 ? entry(i(), "turn", turn()) : j({ _meta: { watane: { type: "turn", state: turn() } } });
    if (x < 0.83) return j({ _meta: { watane: { type: "synced", through: Math.max(0, n - 1 - Math.floor(r() * 3)), writing: r() < 0.5 } } });
    if (x < 0.86) return j({ sessionUpdate: "plan", entries: [{ content: "p", status: "pending" }], ...meta() });
    if (x < 0.88) return j({ _meta: { watane: { type: "queue", inputs: [] } } });
    if (x < 0.90) return entry(i(), "unknown", { raw: 1 });
    if (x < 0.92) return entry(i(), "compaction", {});
    return r() < 0.5 ? "not json" : j({ what: 1 });
  };
  return { one, pick, entryAt: (k: number) => (r() < 0.2 ? null : r() < 0.5 ? entry(k, "message", { role: "agent", content: text() }) : one()) };
}

type Tool = Extract<SessionView["rows"][number], { kind: "tool" }>;
const isOpen = (v: SessionView, ask: unknown) => !!v.x?.open?.some((o) => o.ask === ask);
const checks = (v: SessionView, call: string) => !!v.x?.open?.some((o) => o.ask.kind === "check" && o.ask.toolCallId === call);
/** The row's open ask and its checking flag agree with the reducer's open asks. */
const fits = (v: SessionView, r: Tool) => (!r.ask || !!r.ask.decision || (isOpen(v, r.ask) && r.ask.toolCallId === r.call.id)) && (!r.checking || checks(v, r.call.id));
/** S23-fix4: when an open ask's id moves to another call, the frozen reducer leaves the old call's row holding the stale ask (or a stale
 * checking flag); the new one shows that call's next open ask (and its call) or none. Only such a row takes its ask from the new view, and only when the
 * new row fits; the rest of that sequence is reduced (no throw) but not compared, as the frozen row stays stale and takes no updates. */
let decidedFixed = 0;
function unstale(a: SessionView, b: SessionView): SessionView {
  const rows = b.rows.map((r, k) => {
    const n = a.rows[k] as Tool | undefined;
    // S23-fix5: the frozen reducer shows a decided card the call of the moved entry it removed (another call than the ask's); the new one
    // keeps the ask's own call. Only the call is taken from the new row, and only when that is the ask's own call.
    if (r.kind === "tool" && r.ask?.decision && r.call.id !== r.ask.toolCallId && n?.kind === "tool" && n.key === r.key && n.call.id === n.ask?.toolCallId) {
      decidedFixed++;
      return { ...r, call: n.call } as Tool;
    }
    if (r.kind !== "tool" || fits(b, r) || n?.kind !== "tool" || n.key !== r.key || !fits(a, n)) return r;
    const { ask: _a, checking: _c, ...rest } = r;
    return { ...rest, call: n.call, ...(n.ask ? { ask: n.ask } : {}), ...(n.checking ? { checking: n.checking } : {}) } as Tool;
  });
  return { ...b, rows };
}

/** S156 (S154 r1 P1.2): the footer no longer counts a call that only asked and never ran; the frozen reducer's footer takes the new count
 * when its row is the same footer at the same place. Nothing else is taken. */
let toolsFixed = 0;
function sameTools(a: SessionView, b: SessionView): SessionView {
  if (!b.rows.some((r, k) => r.kind === "footer" && a.rows[k]?.kind === "footer" && a.rows[k]!.key === r.key && (a.rows[k] as typeof r).tools !== r.tools)) return b;
  toolsFixed++;
  return { ...b, rows: b.rows.map((r, k) => { const n = a.rows[k]; return r.kind === "footer" && n?.kind === "footer" && n.key === r.key ? { ...r, tools: n.tools } : r; }) };
}

it("random message sequences give the same view as the reducer of 5882c51 after every step", () => {
  const SEQS = 1000, LEN = 40;
  let steps = 0, moved = 0;
  for (let s = 0; s < SEQS; s++) {
    const r = rng(s + 1);
    const g = messages(r);
    let a: SessionView = emptyView, b: SessionView = emptyView, apart = false;
    const seen: [SessionView, SessionView][] = [];
    for (let k = 0; k < LEN; k++) {
      if (seen.length && r() < 0.05) [a, b] = g.pick(seen); // a parent reduced again
      if (r() < 0.08) {
        const n = Math.max(1, Math.floor(r() * (a.lastIndex + 2))), e = g.entryAt(n);
        a = settle(a, n, e); b = oldSettle(b, n, e);
      } else {
        const m = g.one();
        a = safeReduce(a, m); b = oldSafeReduce(b, m);
      }
      steps++;
      if (!apart && JSON.stringify(a) !== JSON.stringify(b)) {
        const t = sameTools(a, b), c = unstale(a, t);
        if (c.rows.some((r, k) => r !== t.rows[k])) { moved++; apart = true; }
        expect({ seed: s + 1, step: k, view: a }).toEqual({ seed: s + 1, step: k, view: c });
      }
      seen.push([a, b]);
    }
  }
  expect(steps).toBe(SEQS * LEN);
  expect(moved).toBeGreaterThan(0); // moved-ask sequences stay in the run
  expect(decidedFixed).toBeGreaterThan(0); // the decided-card excuse fires
  expect(toolsFixed).toBeGreaterThan(0); // the S156 footer count differs somewhere, and only there
});
