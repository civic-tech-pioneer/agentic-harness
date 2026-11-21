// @vitest-environment node
// Adversary S23 pass 4: a moved ask whose decision entry names its old call leaves the new card holding an open ask
// that is no longer open (row ask !== openAsk, not in openAsks): the card keeps a bar no answer can pass, Needs you drops it.
import { describe, expect, it } from "vitest";
import { emptyView, type Row, type SessionView } from "./model";
import { openAsk, reduce } from "./reduce";

const J = JSON.stringify;
const entry = (i: number, body: object) => J({ _meta: { watane: { type: "entry", entry: { v: 1, i, kind: "ask", at: i, body } } } });
const feed = (msgs: string[], v: SessionView = emptyView) => msgs.reduce(reduce, v);
const on = (i: number, call: string) => entry(i, { id: "a8", kind: "ask", toolCallId: call, input: { n: call }, "expires-at": 9, toolCall: { toolCallId: call, title: call } });
const decided = (i: number, call: string) => entry(i, { id: "a8", kind: "ask", toolCallId: call, input: { n: call }, "expires-at": 9, decision: { outcome: "allow", by: "you" } });
const tool = (v: SessionView, key: string) => v.rows.find((r) => r.kind === "tool" && r.key === key) as Extract<Row, { kind: "tool" }>;

describe("adv S23 fix4 decided", () => {
  it("adv_S23_fix4_decision_on_old_call_leaves_no_card_asking", () => {
    const v = feed([on(1, "e2"), on(2, "e9"), decided(3, "e2")]);
    // every card showing an undecided ask shows the open ask of its id (else its bar is stuck: shown, never sent, not counted)
    for (const r of v.rows) if (r.kind === "tool" && r.ask && !r.ask.decision && r.ask.kind === "ask") expect(openAsk(v, r.ask.id)).toBe(r.ask);
    // and a card keeps its own call
    expect(tool(v, "ce2").call.id).toBe("e2");
  });
});
