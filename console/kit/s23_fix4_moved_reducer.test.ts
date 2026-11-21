// @vitest-environment node
// S23-fix4 AC2: an open ask's id moved to another call leaves the old call's row with no open ask.
import { describe, expect, it } from "vitest";
import { emptyView, type Row, type SessionView } from "./model";
import { reduce } from "./reduce";

const J = JSON.stringify;
const entry = (i: number, body: object) => J({ _meta: { watane: { type: "entry", entry: { v: 1, i, kind: "ask", at: i, body } } } });
const feed = (msgs: string[], v: SessionView = emptyView) => msgs.reduce(reduce, v);
const tool = (v: SessionView, id: string) => v.rows.find((r) => r.kind === "tool" && r.call.id === id) as Extract<Row, { kind: "tool" }>;
const on = (i: number, call: string) => entry(i, { id: "a8", kind: "ask", toolCallId: call, input: { n: call }, "expires-at": 9, toolCall: { toolCallId: call, title: call } });

describe("S23-fix4 reducer", () => {
  it("ac2_moved_ask_old_row_shows_no_open_ask_and_keeps_its_call", () => {
    const v = feed([on(1, "e2"), on(2, "e9")]);
    expect(tool(v, "e2").ask).toBeUndefined();
    expect(tool(v, "e2").call.title).toBe("e2");
    expect(tool(v, "e9").ask?.id).toBe("a8");
    expect(v.openAsks).toEqual(["a8"]);
  });
  it("ac2_moved_ask_old_row_shows_its_next_open_ask", () => {
    const b = entry(2, { id: "b1", kind: "ask", toolCallId: "e2", input: { z: 1 }, "expires-at": 9, toolCall: { toolCallId: "e2", title: "e2b" } });
    const v = feed([on(1, "e2"), b, on(3, "e9")]);
    expect(tool(v, "e2").ask?.id).toBe("b1");
  });
});
