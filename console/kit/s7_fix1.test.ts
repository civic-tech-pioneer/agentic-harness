// @vitest-environment node
// S7-fix1 (P14): indexed lookup and view parity with the reducer of 5882c51.
import { describe, expect, it } from "vitest";
import { longStory } from "../fake/long";
import { story } from "../fake/story";
import { emptyView, type SessionView } from "./model";
import { reduce } from "./reduce";
import { reduce as oldReduce } from "./fixtures/reduce_5882c51";

describe("reducer indexing and output", () => {
  it("ac1_updates_a_tool_without_scanning_unrelated_rows", () => {
    let reads = 0;
    // Count visits to old rows, even if the reducer shallow-copies the array.
    const rows: SessionView["rows"] = Array.from({ length: 5_000 }, (_, i) => ({
      get kind() { reads++; return "user" as const; },
      get key() { reads++; return `u${i}`; },
      text: "earlier message",
    }));
    rows[2_500] = {
      kind: "tool", key: "ctarget",
      call: { id: "target", title: "Check", kind: "other", status: "pending", content: [], locations: [] },
    };
    const synced = JSON.stringify({ _meta: { watane: { type: "synced", through: 0, writing: false } } });
    const indexed = reduce({ ...emptyView, rows }, synced);
    reads = 0;
    const updated = reduce(indexed, JSON.stringify({
      sessionUpdate: "tool_call_update", toolCallId: "target", status: "completed",
    }));
    expect(updated.rows[2_500]).toMatchObject({ kind: "tool", call: { id: "target", status: "completed" } });
    expect(indexed.rows[2_500]).toMatchObject({ call: { status: "pending" } });
    expect(reads).toBeLessThan(10);
  });

  const same = (msgs: string[]) => {
    let a: SessionView = emptyView, b: SessionView = emptyView;
    msgs.forEach((m, k) => {
      a = reduce(a, m); b = oldReduce(b, m);
      if (k % 25 === 0 || k === msgs.length - 1) expect(a).toEqual(b); // every step of short random runs: s7_fix1_random.test.ts
    });
    expect(a.rows.length).toBeGreaterThan(0);
  };
  it("ac2_same_view_as_the_reducer_of_5882c51_for_longStory_500", () => same(longStory(500, 20).flat()));
  it("ac2_same_view_as_the_reducer_of_5882c51_for_the_scripted_story", () => same(story.flat()));

  it("ac3_a_view_reduced_twice_from_one_parent_gives_the_same_result_and_unchanged_rows_keep_identity", () => {
    const msgs = longStory(60, 2).flat();
    let v: SessionView = emptyView;
    for (const m of msgs.slice(0, 200)) v = reduce(v, m);
    const a = reduce(v, msgs[200]!), b = reduce(v, msgs[200]!);
    expect(a).toEqual(b);
    const c = reduce(v, msgs[201]!);
    expect(c.rows.slice(0, v.rows.length - 1).every((r, k) => r === v.rows[k])).toBe(true);
  });
});
