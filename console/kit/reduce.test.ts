// TECHNICAL 18.3: the reducer, one test per row of the table, replay and mid-message attach.
import { describe, expect, it } from "vitest";
import { emptyView, type SessionView } from "./model";
import { reduce, settle } from "./reduce";
import { fakeKernel } from "../fake/kernel";
import { story, expiresAt } from "../fake/story";

const J = JSON.stringify;
const meta = (m: object) => J({ _meta: { watane: m } });
const entry = (i: number, kind: string, body: object, v = 1) => J({ v, i, kind, at: i, body });
const upd = (u: object, m?: object) => J({ ...u, ...(m ? { _meta: { watane: m } } : {}) });
const txt = (t: string) => ({ type: "text", text: t });
const feed = (msgs: string[], v: SessionView = emptyView) => msgs.reduce(reduce, v);
const synced = (through: number) => meta({ type: "synced", through });
const call = (id: string, kind: string, extra: object = {}) => upd({ sessionUpdate: "tool_call", toolCallId: id, title: id, kind, status: "pending", ...extra });

describe("AC2 rows", () => {
  it("ac2_user_message_entry_is_a_user_row", () => {
    const v = feed([entry(1, "message", { role: "user", content: [txt("hi")] })]);
    expect(v.rows).toEqual([{ kind: "user", key: "e1", i: 1, text: "hi" }]);
  });
  it("ac2_thought_chunks_stream_then_fold_with_seconds", () => {
    let v = feed([upd({ sessionUpdate: "agent_thought_chunk", content: txt("a") }, { at: 1000 }), upd({ sessionUpdate: "agent_thought_chunk", content: txt("b") }, { at: 2000 })]);
    expect(v.rows).toHaveLength(1);
    expect(v.rows[0]).toMatchObject({ kind: "thought", text: "ab", streaming: true });
    v = feed([upd({ sessionUpdate: "agent_message_chunk", content: txt("x") }, { at: 4000 })], v);
    expect(v.rows[0]).toMatchObject({ kind: "thought", text: "ab", streaming: false, seconds: 3 });
  });
  it("ac2_reads_and_searches_fold_into_one_explored_row_and_a_break_starts_a_new_one", () => {
    let v = feed([call("a", "read"), call("b", "search"), call("c", "read")]);
    expect(v.rows).toHaveLength(1);
    expect(v.rows[0]).toMatchObject({ kind: "explored", reads: 2, searches: 1 });
    v = feed([call("d", "edit"), call("e", "read")], v);
    expect(v.rows.map((r) => r.kind)).toEqual(["explored", "tool", "explored"]);
  });
  it("ac2_edit_is_a_tool_row_updated_in_place_with_its_diff", () => {
    let v = feed([call("e", "edit")]);
    const diff = { type: "diff", path: "a.ts", oldText: "x", newText: "y" };
    v = feed([upd({ sessionUpdate: "tool_call_update", toolCallId: "e", status: "completed", content: [diff] }, { i: 4 })], v);
    expect(v.rows).toHaveLength(1);
    expect(v.rows[0]).toMatchObject({ kind: "tool", i: 4, call: { status: "completed", content: [diff] } });
  });
  it("ac2_a_failed_read_is_a_tool_row", () => {
    let v = feed([call("a", "read"), call("b", "read"), call("c", "read")]);
    v = feed([upd({ sessionUpdate: "tool_call_update", toolCallId: "b", status: "failed" })], v);
    expect(v.rows.map((r) => r.kind)).toEqual(["explored", "tool", "explored"]);
    expect(feed([entry(1, "tool", { toolCallId: "z", kind: "read", status: "failed" })]).rows[0]!.kind).toBe("tool");
  });
  it("ac2_open_ask_marks_its_tool_row_and_a_decided_ask_is_a_decision_row", () => {
    let v = feed([call("e", "edit"), entry(5, "ask", { toolCallId: "e", input: { x: 1 }, expiresAt: 9 })]);
    expect(v.openAsks).toEqual(["e"]);
    expect(v.rows[0]).toMatchObject({ kind: "tool", ask: { toolCallId: "e", expiresAt: 9 } });
    v = feed([entry(6, "ask", { toolCallId: "e", input: {}, expiresAt: 9, decision: { outcome: "deny", reason: "no", by: "alice" } })], v);
    expect(v.openAsks).toEqual([]);
    expect((v.rows[0] as { ask?: unknown }).ask).toBeUndefined();
    expect(v.rows[1]).toMatchObject({ kind: "decision", who: "alice", reason: "no" });
    expect((v.rows[1] as { what: string }).what).toContain("Denied");
  });
  it("ac3_ask_toolcall_replaces_the_streamed_call_in_its_row", () => {
    let v = feed([call("e", "edit"), entry(5, "ask", { toolCallId: "e", input: {}, expiresAt: 9,
      toolCall: { toolCallId: "e", title: "Edit /real", kind: "edit", status: "pending", locations: [{ path: "/real" }],
        content: [{ type: "diff", path: "/real", oldText: "", newText: "x" }] } })]);
    expect(v.rows[0]).toMatchObject({ kind: "tool", call: { title: "Edit /real", locations: [{ path: "/real" }], content: [{ path: "/real" }] } });
  });
  it("ac3_stream_update_cannot_change_the_open_asks_call_until_it_is_answered", () => {
    const real = { toolCallId: "e", title: "Edit /real", kind: "edit", status: "pending", locations: [{ path: "/real" }],
      content: [{ type: "diff", path: "/real", oldText: "", newText: "x" }] };
    const fake = { sessionUpdate: "tool_call_update", toolCallId: "e", title: "Edit /fake", kind: "read", locations: [{ path: "/fake" }],
      content: [{ type: "diff", path: "/fake", oldText: "", newText: "y" }] };
    let v = feed([call("e", "edit"), entry(5, "ask", { toolCallId: "e", input: {}, expiresAt: 9, toolCall: real }), upd(fake),
      upd({ sessionUpdate: "tool_call_update", toolCallId: "e", status: "in_progress" })]);
    expect(v.rows[0]).toMatchObject({ kind: "tool", call: { title: "Edit /real", kind: "edit", status: "in_progress", locations: [{ path: "/real" }], content: [{ path: "/real", newText: "x" }] } });
    v = feed([entry(6, "ask", { toolCallId: "e", input: {}, expiresAt: 9, decision: { outcome: "allow", by: "alice" } }), upd(fake)], v);
    expect(v.rows[0]).toMatchObject({ kind: "tool", call: { title: "Edit /fake", locations: [{ path: "/fake" }] } });
  });
  it("ac2_allow_and_verdict_are_decision_rows_and_allow_updates_allows", () => {
    let v = feed([entry(1, "allow", { id: "a1", reach: "edit src/**", given: true, by: "alice" })]);
    expect(v.allows).toEqual([{ id: "a1", reach: "edit src/**" }]);
    expect(v.rows[0]).toMatchObject({ kind: "decision", who: "alice" });
    v = feed([entry(2, "allow", { id: "a1", reach: "edit src/**", takenBack: true, by: "alice" }),
      entry(3, "verdict", { toolCallId: "t", outcome: "deny", reasons: ["r1", "r2"], by: "judge" })], v);
    expect(v.allows).toEqual([]);
    expect(v.rows[2]).toMatchObject({ kind: "decision", who: "judge", reason: "r1; r2" });
  });
  it("ac2_plan_turn_queue_usage_set_view_fields_and_idle_adds_a_footer", () => {
    let v = feed([
      upd({ sessionUpdate: "plan", entries: [{ content: "a", priority: "low", status: "completed" }] }),
      meta({ type: "queue", inputs: [{ n: 1, input: "more" }] }),
      meta({ type: "usage", "context-used": 1, "context-window": 2 }),
      call("e", "edit"),
      meta({ type: "turn", state: { state: "running", since: 5 } }),
    ]);
    expect(v.plan).toEqual([{ content: "a", status: "completed" }]);
    expect(v.queue).toEqual([{ n: 1, input: "more" }]);
    expect(v.usage).toEqual({ used: 1, window: 2 });
    expect(v.turn).toEqual({ state: "running", since: 5 });
    v = feed([meta({ type: "turn", state: { state: "idle", stop: { kind: "completed" }, seconds: 9, tokens: 100 } })], v);
    expect(v.turn).toEqual({ state: "idle", stop: { kind: "completed" } });
    expect(v.rows[v.rows.length - 1]).toMatchObject({ kind: "footer", seconds: 9, tools: 1, tokens: 100 });
  });
  it("ac2_unknown_things_become_unknown_rows", () => {
    const v = feed([
      entry(1, "unknown", { raw: { a: 1 } }),
      entry(2, "message", { role: "user", content: [] }),
      entry(3, "message", { role: "user", content: [] }, 9),
      "not json",
      J({ hello: 1 }),
    ]);
    expect(v.rows.map((r) => r.kind)).toEqual(["unknown", "user", "unknown", "unknown", "unknown"]);
    expect(v.rows[0]).toMatchObject({ raw: { a: 1 } });
    expect(v.rows[3]).toMatchObject({ raw: "not json" });
  });
  it("ac2_agent_chunks_make_one_writing_row_until_the_entry_index_arrives", () => {
    let v = feed([upd({ sessionUpdate: "agent_message_chunk", content: txt("ab") }), upd({ sessionUpdate: "agent_message_chunk", content: txt("c") })]);
    expect(v.rows).toHaveLength(1);
    expect(v.rows[0]).toMatchObject({ kind: "agent", text: "abc", writing: true });
    v = feed([upd({ sessionUpdate: "agent_message_chunk", content: txt("d") }, { i: 3 })], v);
    expect(v.rows).toEqual([{ kind: "agent", key: "e3", i: 3, text: "abcd", writing: false }]);
  });
});

describe("AC3 replay", () => {
  it("ac3_an_index_already_shown_is_dropped_and_synced_sets_the_fields", () => {
    const m = (i: number) => entry(i, "message", { role: "user", content: [txt(`m${i}`)] });
    let v = feed([m(1), m(2), m(3), synced(3)]);
    expect(v).toMatchObject({ synced: true, lastIndex: 3 });
    v = feed([m(2), m(3), m(4), synced(4)], v);
    expect(v.rows.map((r) => (r as { text: string }).text)).toEqual(["m1", "m2", "m3", "m4"]);
    expect(v.lastIndex).toBe(4);
    expect(feed([synced(7)]).lastIndex).toBe(7);
  });
});

describe("AC4 mid-message attach", () => {
  const chunk = (t: string, m?: object) => upd({ sessionUpdate: "agent_message_chunk", content: txt(t) }, m);
  it("ac4_synced_writing_drops_the_deltas_shows_writing_and_the_entry_replaces_it_once_whole", () => {
    let v = feed([entry(4, "message", { role: "user", content: [txt("q")] }), meta({ type: "synced", through: 4, writing: true })]);
    expect(v.rows[1]).toEqual({ kind: "agent", key: "~w", text: "", writing: true });
    v = feed([chunk("tail "), upd({ sessionUpdate: "agent_thought_chunk", content: txt("hm") }), chunk("more")], v);
    expect(v.rows).toHaveLength(2);
    expect(v.rows[1]).toEqual({ kind: "agent", key: "~w", text: "", writing: true });
    v = feed([chunk("end", { i: 5 })], v);
    expect(v.rows[1]).toEqual({ kind: "agent", key: "~p5", text: "", writing: true });
    // the entry replaces it even though lastIndex is already at 5, and moves nothing else
    v = feed([entry(5, "message", { role: "agent", content: [txt("whole message")], thought: { text: "hm", seconds: 2 } })], v);
    expect(v.rows.slice(1)).toEqual([
      { kind: "thought", key: "e5-t", i: 5, text: "hm", streaming: false, seconds: 2 },
      { kind: "agent", key: "e5", i: 5, text: "whole message", writing: false },
    ]);
    v = feed([chunk("next")], v);
    expect(v.rows[3]).toMatchObject({ text: "next", writing: true });
  });
  it("ac4_without_writing_a_first_chunk_after_synced_streams", () => {
    let v = feed([synced(0), chunk("tail "), chunk("more")]);
    expect(v.rows).toEqual([{ kind: "agent", key: "~a0", text: "tail more", writing: true }]);
    v = feed([chunk("!", { i: 5 })], v);
    expect(v.rows).toEqual([{ kind: "agent", key: "e5", i: 5, text: "tail more!", writing: false }]);
  });
});

describe("AC7 story", () => {
  it("ac7_the_story_ends_with_the_view_of_the_frame", () => {
    const v = feed(story.flat());
    expect(v.rows.filter((r) => r.kind === "user")).toHaveLength(1);
    expect(v.rows.find((r) => r.kind === "thought")).toMatchObject({ streaming: false, seconds: 3 });
    expect(v.rows.find((r) => r.kind === "explored")).toMatchObject({ reads: 3, searches: 1 });
    const edits = v.rows.filter((r) => r.kind === "tool");
    expect(edits).toHaveLength(2);
    expect(edits[0]).toMatchObject({ call: { status: "completed", content: [{ type: "diff", path: "src/retry.ts" }] } });
    expect(edits[1]).toMatchObject({ ask: { toolCallId: "e2" } });
    expect(v.openAsks).toEqual(["e2"]);
    expect(v.turn).toMatchObject({ state: "waiting", expiresAt: expiresAt });
    expect(v.queue).toHaveLength(1);
    expect(v.usage).toEqual({ used: 42000, window: 200000, compactAt: 160000 });
    expect(v.plan).toHaveLength(3);
    expect(v.plan!.filter((p) => p.status === "completed")).toHaveLength(1);
  });
  it("ac7_the_fake_plays_the_story_through_play", () => {
    const k = fakeKernel(story);
    const seen: string[] = [];
    void k.attach("s", 0, (j) => seen.push(j), () => {});
    for (let n = 0; n < story.length; n++) k.play();
    expect(seen).toEqual([JSON.stringify({ _meta: { watane: { type: "synced", through: 0 } } }), ...story.flat()]);
  });
});

describe("S4-fix2 AC2 unique keys", () => {
  const user = entry(5, "message", { role: "user", content: [txt("hi")] });
  const start = () => feed([meta({ type: "synced", through: 4, writing: true }), upd({ sessionUpdate: "agent_message_chunk", content: txt("x") }, { i: 5 })]);
  const unique = (v: SessionView) => expect(new Set(v.rows.map((r) => r.key)).size).toBe(v.rows.length);
  it("ac2_stream_then_read_keeps_unique_keys", () => {
    const v = settle(reduce(start(), meta({ type: "entry", entry: JSON.parse(user) })), 5, user);
    unique(v);
    expect(v.rows.some((r) => r.key.startsWith("~p"))).toBe(false);
  });
  it("ac2_read_then_stream_keeps_unique_keys", () => {
    const v = reduce(settle(start(), 5, user), meta({ type: "entry", entry: JSON.parse(user) }));
    unique(v);
    expect(v.rows.filter((r) => r.key === "e5")).toHaveLength(1);
  });
  it("ac2_read_returning_null_keeps_unique_keys", () => {
    const v = settle(reduce(start(), meta({ type: "entry", entry: JSON.parse(user) })), 5, null);
    unique(v);
    const w = settle(start(), 5, null);
    unique(w);
    expect(w.rows.some((r) => r.key.startsWith("~p"))).toBe(false);
  });
});
