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

it("random message sequences give the same view as the reducer of 5882c51 after every step", () => {
  const SEQS = 2000, LEN = 40;
  let steps = 0;
  for (let s = 0; s < SEQS; s++) {
    const r = rng(s + 1);
    const g = messages(r);
    let a: SessionView = emptyView, b: SessionView = emptyView;
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
      if (JSON.stringify(a) !== JSON.stringify(b)) expect({ seed: s + 1, step: k, view: a }).toEqual({ seed: s + 1, step: k, view: b });
      seen.push([a, b]);
    }
  }
  expect(steps).toBe(SEQS * LEN);
});
