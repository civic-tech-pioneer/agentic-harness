// @vitest-environment node
// Validator S15-fix1: a second entry on one call id lends nothing of its call (status, output) to the card of the entry shown (TECHNICAL 9.8, 14).
import { describe, expect, it } from "vitest";
import { emptyView, type Row, type SessionView } from "./model";
import { reduce } from "./reduce";

const J = JSON.stringify;
const entry = (i: number, kind: string, body: object) => J({ _meta: { watane: { type: "entry", entry: { v: 1, i, kind, at: i, body } } } });
const feed = (msgs: string[], v: SessionView = emptyView) => msgs.reduce(reduce, v);
const card = (v: SessionView) => v.rows.find((r) => r.kind === "tool") as Extract<Row, { kind: "tool" }>;
const a1 = entry(1, "ask", { id: "a1", kind: "ask", toolCallId: "t", input: { a: 1 }, "expires-at": 9, toolCall: { toolCallId: "t", title: "bank", rawInput: { a: 1 } } });
const a2 = entry(2, "ask", { id: "a2", kind: "ask", toolCallId: "t", input: { n: 1 }, "expires-at": 9,
  toolCall: { toolCallId: "t", title: "notes", status: "completed", rawOutput: "harmless", content: [{ type: "content", content: { type: "text", text: "a note" } }] } });

describe("S15-fix1 validator", () => {
  it("regress_S15_fix1_second_entry_lends_no_status_or_output", () => {
    const c = card(feed([a1, a2]));
    expect(c.ask?.id).toBe("a1");
    expect(c.call.title).toBe("bank");
    expect(c.call.status).toBe("pending");
    expect(c.call.content).toEqual([]);
    expect("rawOutput" in c.call).toBe(false);
  });
  it("regress_S15_fix1_decided_entry_hands_the_card_to_the_open_ones_call", () => {
    const d1 = entry(3, "ask", { id: "a1", kind: "ask", toolCallId: "t", input: { a: 1 }, "expires-at": 9, decision: { outcome: "deny", by: "alice" } });
    const c = card(feed([a1, a2, d1]));
    expect(c.ask?.id).toBe("a2");
    expect(c.call.title).toBe("notes");
    expect(c.call.rawOutput).toBe("harmless");
  });
  it("regress_S15_fix1_none_open_keeps_the_call_decided_last", () => {
    const d = (i: number, id: string) => entry(i, "ask", { id, kind: "ask", toolCallId: "t", input: {}, "expires-at": 9, decision: { outcome: "deny", by: "alice" } });
    const c = card(feed([a1, a2, d(3, "a2"), d(4, "a1")]));
    expect(c.ask?.id).toBe("a1");
    expect(c.call.title).toBe("bank");
  });
});
