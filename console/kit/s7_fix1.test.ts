// @vitest-environment node
// S7-fix1 (P14): one reduce costs the same at row 10 as at row 5 000, and the view is what the reducer of 5882c51 gave.
import { describe, expect, it } from "vitest";
import { longStory } from "../fake/long";
import { story } from "../fake/story";
import { emptyView, type SessionView } from "./model";
import { reduce } from "./reduce";
import { reduce as oldReduce } from "./s7_fix1_old_reduce";

const mean = (a: number[]) => a.reduce((s, x) => s + x, 0) / a.length;

describe("reducer cost and output", () => {
  it("ac1_cost_per_message_does_not_grow_with_the_transcript", () => {
    const msgs = longStory(5000, 0)[0]!;
    const run = () => {
      let v: SessionView = emptyView;
      const t: number[] = [];
      const t0 = performance.now();
      for (const m of msgs) { const a = performance.now(); v = reduce(v, m); t.push(performance.now() - a); }
      return { total: performance.now() - t0, first: mean(t.slice(0, 500)), last: mean(t.slice(-500)) };
    };
    const r = run();
    console.log(`reduce ${msgs.length} messages: total ${r.total.toFixed(1)} ms, first 500 ${(r.first * 1000).toFixed(1)} us, last 500 ${(r.last * 1000).toFixed(1)} us`);
    expect(r.total).toBeLessThanOrEqual(400);
    expect(r.last).toBeLessThanOrEqual(2 * r.first);
  });

  const same = (msgs: string[]) => {
    let a: SessionView = emptyView, b: SessionView = emptyView;
    for (const m of msgs) {
      a = reduce(a, m); b = oldReduce(b, m);
      expect(a).toEqual(b);
    }
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
