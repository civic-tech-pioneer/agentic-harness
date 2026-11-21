// @vitest-environment node
// S23-fix5 AC3: after every message of a random stream of asks and checks (ids from a small shared pool, so an id moves between calls,
// is decided naming either call, reopens, and the stream updates the calls), each tool row shows an open ask or check only if it is
// exactly the open entry for its own call (TECHNICAL 18.3, 9.8).
import { expect, it } from "vitest";
import { emptyView, type Row, type SessionView } from "./model";
import { reduce } from "./reduce";

function rng(seed: number) {
  let s = seed >>> 0;
  return () => { s = (s + 0x6d2b79f5) >>> 0; let t = s; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}
const J = JSON.stringify;

function check(v: SessionView, log: string[]) {
  const fail = (what: string) => { throw new Error(`${what}\n${log.join("\n")}`); };
  const open = v.x?.open ?? [];
  const tools = v.rows.filter((r): r is Extract<Row, { kind: "tool" }> => r.kind === "tool");
  for (const r of tools) {
    if (!r.ask) continue;
    if (r.ask.decision) { if (r.call.id !== r.ask.toolCallId) fail(`(d) decided card of ${r.ask.toolCallId} shows call ${r.call.id}`); continue; }
    if (!open.some((o) => o.ask === r.ask && o.ask.toolCallId === r.call.id)) fail(`(a) row ${r.key} shows ask ${r.ask.id} that is not open on its call`);
  }
  for (const o of open) {
    if (o.ask.kind !== "ask") continue;
    const shown = tools.filter((r) => r.ask === o.ask).length;
    if (shown > 1) fail(`(b) ask ${o.ask.id} shown on ${shown} rows`);
    if (shown === 0) {
      const row = tools.find((r) => r.call.id === o.ask.toolCallId);
      const first = row?.ask && !row.ask.decision && row.ask.kind === "ask" && open.some((p) => p.ask === row.ask);
      if (!first) fail(`(b) open ask ${o.ask.id} on ${o.ask.toolCallId} has no row`);
    }
  }
  const ids = open.filter((o) => o.ask.kind === "ask").map((o) => o.ask.id);
  if (J(v.openAsks) !== J(ids)) fail(`(c) openAsks ${J(v.openAsks)} != ${J(ids)}`);
}

it("ac3_open_ask_invariant_holds_after_every_message", () => {
  const r = rng(23005);
  const pick = <T,>(a: readonly T[]): T => a[Math.floor(r() * a.length)]!;
  const t0 = Date.now();
  for (let s = 0; s < 1000; s++) {
    let v = emptyView, n = 1;
    const log: string[] = [];
    for (let k = 0; k < 10; k++) {
      const call = pick(["e1", "e2", "e3"]), id = pick(["a1", "a2", "a3"]), kind = pick(["ask", "ask", "check"]);
      const x = r();
      let m: string;
      if (x < 0.2) m = J({ sessionUpdate: pick(["tool_call", "tool_call_update"]), toolCallId: call, title: `t${call}`, status: pick(["pending", "in_progress", "completed"]), ...(r() < 0.3 ? { content: [{ type: "content", content: { type: "text", text: "o" } }] } : {}) });
      else {
        const decision = x < 0.5 ? { decision: { outcome: pick(["allow", "deny"]), by: "you" } } : {};
        const withCall = r() < 0.7 && !decision.decision ? { toolCall: { toolCallId: call, title: `t${call}` } } : {};
        m = J({ _meta: { watane: { type: "entry", entry: { v: 1, i: n++, kind: "ask", at: n, body: { id, kind, toolCallId: call, input: { n: call }, "expires-at": 9, ...withCall, ...decision } } } } });
      }
      log.push(m);
      v = reduce(v, m);
      check(v, log);
    }
  }
  expect(Date.now() - t0).toBeLessThan(1000);
});
