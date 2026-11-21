// @vitest-environment node
// S23-fix5 AC3: after every message of a random stream of asks and checks (ids from a small shared pool, so an id moves between calls,
// is decided naming either call, reopens, and the stream updates the calls), each tool row shows an open ask or check only if it is
// exactly the open entry for its own call (TECHNICAL 18.3, 9.8).
import { expect, it } from "vitest";
import { emptyView, type Row, type SessionView } from "./model";
import { openAsk, reduce, settleShort } from "./reduce";

function rng(seed: number) {
  let s = seed >>> 0;
  return () => { s = (s + 0x6d2b79f5) >>> 0; let t = s; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}
const J = JSON.stringify;

const decision0 = (m: string) => "decision" in JSON.parse(m)._meta.watane.entry.body;
function check(v: SessionView, log: string[]) {
  const fail = (what: string) => { throw new Error(`${what}\n${log.join("\n")}`); };
  const open = v.x?.open ?? [];
  const tools = v.rows.filter((r): r is Extract<Row, { kind: "tool" }> => r.kind === "tool");
  for (const r of tools) {
    if (!r.ask) continue;
    // validator: the card draws only its own call's ask (its input) and call (title), never another call's
    if ((r.ask.input as { n?: unknown }).n !== r.call.id || ![`t${r.call.id}`, "other", "read", r.call.id].includes(r.call.title)) fail(`(f) row ${r.key} of ${r.call.id} shows ${J(r.ask.input)} ${r.call.title}`);
    if (r.ask.decision) { if (r.call.id !== r.ask.toolCallId) fail(`(d) decided card of ${r.ask.toolCallId} shows call ${r.call.id}`); continue; }
    if (!open.some((o) => o.ask === r.ask && o.ask.toolCallId === r.call.id)) fail(`(a) row ${r.key} shows ask ${r.ask.id} that is not open on its call`);
    if (r.ask.kind === "ask" && openAsk(v, r.ask.id) !== r.ask) fail(`(e) openAsk(${r.ask.id}) is not the row's ask`);
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
    const sent: { i: number; id: string; kind: string; m: string }[] = []; // S30-1: the ask entries sent, which short decisions name
    const waiting: number[] = []; // the placeholders whose ask entry has not been read yet
    for (let k = 0; k < 10; k++) {
      const call = pick(["e1", "e2", "e3"]), id = pick(["a1", "a2", "a3"]), kind = pick(["ask", "ask", "check"]);
      const x = r(), callKind = r() < 0.2 ? { kind: "read" } : {}; // validator: a read call sits in an explored row until an ask splits it
      let m: string;
      if (waiting.length && r() < 0.3) {
        // S30-1: a read of the ask entry returns (the entry sent at `ask-i`, or nothing); it is applied to the view as it is now
        const i = waiting.splice(Math.floor(r() * waiting.length), 1)[0]!;
        const sh = v.x?.shorts?.find((q) => q.i === i);
        const e = sent.find((q) => q.i === sh?.askI);
        log.push(`settle ${i} ${e?.m}`);
        v = settleShort(v, i, r() < 0.8 ? e?.m ?? null : null);
        check(v, log);
        continue;
      }
      if (sent.length && r() < 0.2) {
        // S30-1: a short decision of a sent ask (held when that ask is open at that index, else not held), or of an id nobody sent
        const e = pick(sent);
        const i = n++;
        m = J({ _meta: { watane: { type: "entry", entry: { v: 1, i, kind: "ask", at: i, body: { id: r() < 0.9 ? e.id : "a9", kind: e.kind, "ask-i": e.i, decision: { outcome: pick(["allow", "deny", "none"]), by: "kernel" } } } } } });
        log.push(m);
        v = reduce(v, m);
        if (v.x?.shorts?.some((q) => q.i === i)) waiting.push(i);
        check(v, log);
        continue;
      }
      if (x < 0.2) m = J({ sessionUpdate: pick(["tool_call", "tool_call_update"]), toolCallId: call, title: `t${call}`, ...callKind, status: pick(["pending", "in_progress", "completed"]), ...(r() < 0.3 ? { content: [{ type: "content", content: { type: "text", text: "o" } }] } : {}) });
      else {
        const decision = x < 0.5 ? { decision: { outcome: pick(["allow", "deny"]), by: "you" } } : {};
        const withCall = r() < 0.6 ? { toolCall: { toolCallId: call, title: `t${call}`, ...callKind } } : {};
        const i = r() < 0.9 ? n++ : Math.max(1, n - 2); // validator: sometimes an index already seen (replay), which the reducer drops
        m = J({ _meta: { watane: { type: "entry", entry: { v: 1, i, kind: "ask", at: i, body: { id, kind, toolCallId: call, input: { n: call }, "expires-at": 9, ...withCall, ...decision } } } } });
      }
      log.push(m);
      v = reduce(v, m);
      if (!m.includes("sessionUpdate") && !decision0(m)) { const b = JSON.parse(m)._meta.watane.entry; sent.push({ i: b.i, id: b.body.id, kind: b.body.kind, m: JSON.stringify(b) }); }
      check(v, log);
    }
  }
  expect(Date.now() - t0).toBeLessThan(1000);
});
