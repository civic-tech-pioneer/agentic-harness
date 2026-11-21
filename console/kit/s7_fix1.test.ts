// @vitest-environment node
// S7-fix1 (P14): one reduce costs the same at row 10 as at row 5 000, and the view is what the reducer of 5882c51 gave.
import { describe, expect, it } from "vitest";
import { longStory } from "../fake/long";
import { story } from "../fake/story";
import { emptyView, type SessionView } from "./model";
import { reduce } from "./reduce";
import { reduce as oldReduce } from "./fixtures/reduce_5882c51";

describe("reducer cost and output", () => {
  // Warmed up (a first pass compiles the reducer), then timed in blocks of 500 messages, the best of 3 passes so a GC pause does not count:
  // the 500 messages after the 500th (a view of some 300 rows) against the last 500 (some 5 000 rows). Copying the rows array on a change still grows with it (about 4x here); the reducer of 5882c51 gives about 19x and 0.5 ms.
  it("ac1_cost_per_message_does_not_grow_with_the_transcript", () => {
    const msgs = longStory(5000, 0)[0]!;
    const pass = () => {
      let v: SessionView = emptyView;
      const t0 = performance.now();
      for (const m of msgs.slice(0, 500)) v = reduce(v, m);
      const a = performance.now();
      for (const m of msgs.slice(500, 1000)) v = reduce(v, m);
      const b = performance.now();
      for (const m of msgs.slice(1000, -500)) v = reduce(v, m);
      const c = performance.now();
      for (const m of msgs.slice(-500)) v = reduce(v, m);
      const d = performance.now();
      expect(v.rows.length).toBeGreaterThan(5000);
      return { total: d - t0, early: (b - a) / 500, late: (d - c) / 500 };
    };
    const cold = pass();
    const warm = [pass(), pass(), pass()];
    const early = Math.min(...warm.map((r) => r.early)), late = Math.min(...warm.map((r) => r.late));
    console.log(`reduce ${msgs.length} messages: cold total ${cold.total.toFixed(1)} ms; warm mean per message ${(early * 1000).toFixed(1)} us at ~300 rows, ${(late * 1000).toFixed(1)} us at ~5 000 rows`);
    expect(cold.total).toBeLessThanOrEqual(400);
    expect(late).toBeLessThanOrEqual(8 * early);
    expect(late).toBeLessThanOrEqual(0.05);
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
