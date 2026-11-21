// S15-2 AC1, AC2: the joiner of `part` entries (TECHNICAL 9.4).
import { describe, expect, it } from "vitest";
import { partsOf } from "../fake/kernel";
import { joiner } from "./join";

const msg = (i: number, text = "hi") => ({ v: 1, i, kind: "message", at: 1, body: { role: "agent", content: [{ type: "text", text }] } });
const part = (i: number, n: number, m: number, text: string) => JSON.stringify({ v: 1, i, kind: "part", at: 1, body: { n, m, text } });
const asItem = (p: string) => JSON.stringify({ _meta: { watane: { type: "entry", entry: JSON.parse(p) } } });

describe("S15-2 AC1 the joiner", () => {
  it("ac1_a_message_that_is_not_a_part_passes_through_unchanged", () => {
    const j = joiner();
    const s = JSON.stringify(msg(1));
    expect(j.push(s)).toEqual([s]);
    expect(j.push("not json")).toEqual(["not json"]);
  });
  it("ac1_parts_pass_on_nothing_until_the_last_which_passes_the_whole_entry", () => {
    const j = joiner();
    const ps = partsOf(msg(5, "x".repeat(100)), 40);
    expect(ps.length).toBeGreaterThan(2);
    const out = ps.flatMap((p) => j.push(p));
    expect(out).toHaveLength(1);
    expect(JSON.parse(out[0])).toEqual(msg(5, "x".repeat(100)));
  });
  it("ac1_parts_inside_entry_items_join_too", () => {
    const j = joiner();
    const out = partsOf(msg(3, "y".repeat(50)), 30).flatMap((p) => j.push(asItem(p)));
    expect(out).toHaveLength(1);
    expect(JSON.parse(out[0]).i).toBe(3);
  });
});

describe("S15-2 AC2 failures", () => {
  const unknownAt = (s: string, i: number) => { const o = JSON.parse(s); expect(o).toMatchObject({ v: 1, i, kind: "unknown" }); expect(o.body).toHaveProperty("raw"); };
  it("ac2_a_gap_in_n_fails_once", () => {
    const j = joiner();
    expect(j.push(part(1, 1, 3, "{"))).toEqual([]);
    const out = j.push(part(2, 3, 3, "x"));
    expect(out).toHaveLength(1); unknownAt(out[0], 2);
  });
  it("ac2_a_different_m_fails_and_the_rest_of_the_group_is_absorbed", () => {
    const j = joiner();
    j.push(part(1, 1, 3, "{"));
    const out = j.push(part(2, 2, 4, "x"));
    expect(out).toHaveLength(1); unknownAt(out[0], 2);
    expect(j.push(part(3, 3, 4, "y"))).toEqual([]);
    expect(j.push(part(4, 4, 4, "z"))).toEqual([]);
    const s = JSON.stringify(msg(5));
    expect(j.push(s)).toEqual([s]);
  });
  it("ac2_i_not_consecutive_fails", () => {
    const j = joiner();
    j.push(part(1, 1, 2, "{"));
    unknownAt(j.push(part(3, 2, 2, "}"))[0], 3);
  });
  it("ac2_bad_n_or_m_fails", () => {
    unknownAt(joiner().push(part(1, 0, 2, "{"))[0], 1);
    unknownAt(joiner().push(part(1, 3, 2, "{"))[0], 1);
    unknownAt(joiner().push(part(1, 1.5, 2, "{"))[0], 1);
    unknownAt(joiner().push(part(1, 1, 5000, "{"))[0], 1);
  });
  it("ac2_a_group_that_does_not_start_at_1_fails", () => {
    unknownAt(joiner().push(part(7, 2, 3, "{"))[0], 7);
  });
  it("ac2_text_that_is_not_one_entry_fails", () => {
    const j = joiner();
    j.push(part(1, 1, 2, "{\"v\":"));
    unknownAt(j.push(part(2, 2, 2, "nope"))[0], 2);
    const k = joiner();
    k.push(part(1, 1, 2, "[1,"));
    unknownAt(k.push(part(2, 2, 2, "2]"))[0], 2);
  });
  it("ac2_a_joined_entry_whose_i_is_not_the_last_parts_fails", () => {
    const j = joiner();
    const shifted = partsOf(msg(8, "z".repeat(60)), 40).map((p) => JSON.stringify({ ...JSON.parse(p), i: JSON.parse(p).i + 1 }));
    const out = shifted.flatMap((p) => j.push(p));
    expect(out).toHaveLength(1);
    expect(JSON.parse(out[0]).kind).toBe("unknown");
  });
  it("ac2_the_joined_text_over_64_MiB_fails", () => {
    const j = joiner();
    const big = "a".repeat(1024 * 1024);
    let out: string[] = [];
    for (let k = 1; k <= 70 && !out.length; k++) out = j.push(part(k, k, 70, big));
    expect(out).toHaveLength(1); expect(JSON.parse(out[0]).kind).toBe("unknown");
  });
  it("ac2_a_non_part_entry_fails_the_open_group_at_its_last_i_and_passes_on", () => {
    const j = joiner();
    j.push(part(1, 1, 3, "{"));
    j.push(part(2, 2, 3, "\"")); 
    const s = JSON.stringify(msg(3));
    const out = j.push(s);
    expect(out).toHaveLength(2); unknownAt(out[0], 2); expect(out[1]).toBe(s);
    expect(j.push(s)).toEqual([s]);
  });
  it("ac2_an_item_that_is_not_an_entry_leaves_the_open_group_open", () => {
    const j = joiner();
    const ps = partsOf(msg(4, "x".repeat(100)), 40);
    const turn = JSON.stringify({ _meta: { watane: { type: "turn", state: { state: "running", since: 1 } } } });
    expect(j.push(ps[0])).toEqual([]);
    expect(j.push(turn)).toEqual([turn]);
    const out = ps.slice(1).flatMap((p) => j.push(p));
    expect(out).toHaveLength(1); expect(JSON.parse(out[0])).toEqual(msg(4, "x".repeat(100)));
  });
});
