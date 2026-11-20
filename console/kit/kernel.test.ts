// TECHNICAL 9.4, 18.3: openSession against the fake kernel: reattach, reload, behind.
import { describe, expect, it, vi } from "vitest";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { openSession } from "./kernel";
import type { SessionView } from "./model";

const flush = () => new Promise((r) => setTimeout(r, 0));
function open(k: ReturnType<typeof fakeKernel>) {
  const views: SessionView[] = [];
  const s = openSession(k, "s1", (v) => views.push(v));
  return { s, views, last: () => views[views.length - 1]! };
}

describe("AC5 reattach and reload", () => {
  it("ac5_reattach_after_a_drop_uses_the_last_index_and_nothing_doubles", async () => {
    const k = fakeKernel(story);
    const a = open(k);
    await flush();
    expect(k.calls[0]).toEqual({ name: "attach", args: ["s1", 0] });
    k.play(0); k.play(1);
    const before = a.last();
    expect(before.lastIndex).toBe(6);
    k.drop();
    await flush();
    expect(k.calls.filter((c) => c.name === "attach")[1]!.args).toEqual(["s1", before.lastIndex]);
    expect(a.last().rows).toEqual(before.rows);
    k.play(2);
    const after = a.last();
    expect(after.rows.slice(0, before.rows.length)).toEqual(before.rows);
    expect(after.rows.length).toBeGreaterThan(before.rows.length);
    const keys = after.rows.map((r) => r.key);
    expect(new Set(keys).size).toBe(keys.length);
  });
  it("ac5_after_a_drop_the_live_steps_match_a_fresh_open_with_nothing_doubled", async () => {
    const k = fakeKernel(story);
    const a = open(k);
    await flush();
    k.play(0);
    k.drop();
    k.play(1); k.play(2);
    await flush();
    const b = open(k);
    await flush();
    expect(a.last().rows).toEqual(b.last().rows);
    const keys = a.last().rows.map((r) => r.key);
    expect(new Set(keys).size).toBe(keys.length);
  });
  it("ac5_a_fresh_open_after_a_reload_ends_with_the_same_rows", async () => {
    const k = fakeKernel(story);
    const a = open(k);
    await flush();
    k.play(0); k.play(1); k.play(2);
    const b = open(k);
    await flush();
    expect(b.last().rows).toEqual(a.last().rows);
    expect(b.last().lastIndex).toBe(a.last().lastIndex);
  });
});

describe("AC6 behind", () => {
  it("ac6_pages_with_read_newest_first_then_attaches_after_through", async () => {
    const pre = Array.from({ length: 300 }, (_, n) => JSON.stringify({ v: 1, i: n + 1, kind: "message", at: n, body: { role: "user", content: [{ type: "text", text: `m${n + 1}` }] } }));
    const k = fakeKernel([], { preload: pre });
    const a = open(k);
    await flush();
    const reads = k.calls.filter((c) => c.name === "read");
    expect(reads.map((c) => c.args)).toEqual([["s1", 44, 256], ["s1", 0, 44]]);
    expect(k.calls.map((c) => c.name)).toEqual(["attach", "read", "read", "attach"]);
    expect(k.calls[3]!.args).toEqual(["s1", 300]);
    expect(a.views.some((v) => v.rows.length === 256 && (v.rows[255] as { text: string }).text === "m300")).toBe(true);
    const texts = a.last().rows.map((r) => (r as { text: string }).text);
    expect(texts).toEqual(pre.map((_, n) => `m${n + 1}`));
    expect(a.last().synced).toBe(true);
  });
});

const J = JSON.stringify;
const chunk = (t: string, i?: number) => J({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: t }, ...(i ? { _meta: { watane: { i, at: 1 } } } : {}) });
const running = J({ _meta: { watane: { type: "turn", state: { state: "running", since: 1 } } } });

describe("lost message after attach", () => {
  it("lost_a_message_whose_first_chunk_comes_after_synced_ends_whole", async () => {
    const k = fakeKernel([[running, chunk("Hello "), chunk("world", 1), J({ sessionUpdate: "tool_call", toolCallId: "t", title: "t", kind: "edit", _meta: { watane: { i: 2 } } })]]);
    const a = open(k);
    await flush();
    k.play(0);
    await flush();
    const rows = a.last().rows;
    expect(rows.map((r) => r.kind)).toEqual(["agent", "tool"]);
    expect(rows[0]).toMatchObject({ text: "Hello world", writing: false, key: "e1" });
  });
  it("lost_a_real_mid_message_attach_gets_the_whole_entry_on_the_finishing_update", async () => {
    const k = fakeKernel([[running, chunk("Hello ")], [chunk("world", 1)]]);
    k.play(0);
    const a = open(k);
    await flush();
    expect(a.last().rows).toEqual([{ kind: "agent", key: "~w", text: "", writing: true }]);
    k.play(1);
    expect(a.views.every((v) => v.rows.every((r) => r.kind !== "agent" || r.text === "" || r.text === "Hello world"))).toBe(true);
    await flush();
    expect(k.calls.some((c) => c.name === "read" && JSON.stringify(c.args) === J(["s1", 0, 1]))).toBe(true);
    expect(a.last().rows).toEqual([{ kind: "agent", key: "e1", i: 1, text: "Hello world", writing: false }]);
  });
});

describe("full story reload", () => {
  it("story_a_fresh_open_after_all_four_steps_equals_the_live_view", async () => {
    const k = fakeKernel(story);
    const a = open(k);
    await flush();
    for (let n = 0; n < 4; n++) k.play();
    const b = open(k);
    await flush();
    const { x: _a, ...live } = a.last();
    const { x: _b, ...fresh } = b.last();
    expect(fresh).toEqual(live);
    expect(fresh.turn).toMatchObject({ state: "waiting" });
    expect(fresh.plan).toHaveLength(3);
  });
});

describe("S4-fix2 AC3 behind that cannot be paged", () => {
  it("ac3_stuck_behind_waits_1s_between_attaches_and_stops_after_3_with_an_unknown_row", async () => {
    vi.useFakeTimers();
    try {
      let attaches = 0;
      const k = { async attach() { attaches++; return { behind: 0 }; }, read: async () => [], say: async () => 0, answer: async () => {}, interrupt: async () => {}, takeBack: async () => {} };
      let last: SessionView | undefined;
      const s = openSession(k, "s1", (v) => { last = v; });
      await vi.advanceTimersByTimeAsync(500);
      expect(attaches).toBe(1);
      await vi.advanceTimersByTimeAsync(600);
      expect(attaches).toBe(2);
      await vi.advanceTimersByTimeAsync(10_000);
      expect(attaches).toBe(3);
      expect(last!.rows.filter((r) => r.kind === "unknown").length).toBeGreaterThan(0);
      s.close();
    } finally { vi.useRealTimers(); }
  });
});
