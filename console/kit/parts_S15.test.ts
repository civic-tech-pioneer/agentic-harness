// S15-2 AC3-AC7: an entry stored as parts reads whole in the console (TECHNICAL 9.4).
import { describe, expect, it, vi } from "vitest";
import { fakeKernel, partsOf } from "../fake/kernel";
import { openSession } from "./kernel";
import type { SessionView } from "./model";

const flush = () => new Promise((r) => setTimeout(r, 0));
const msg = (i: number, text: string) => ({ v: 1, i, kind: "message", at: i, body: { role: "agent", content: [{ type: "text", text }] } });
const user = (i: number) => JSON.stringify({ v: 1, i, kind: "message", at: i, body: { role: "user", content: [{ type: "text", text: `u${i}` }] } });
const big = "é漢x".repeat(100 * 1024); // about 600 KB, multi-byte
const item = (p: string) => JSON.stringify({ _meta: { watane: { type: "entry", entry: JSON.parse(p) } } });
function open(k: ReturnType<typeof fakeKernel>) {
  const views: SessionView[] = [];
  const s = openSession(k, "s1", (v) => views.push(v));
  return { s, views, last: () => views[views.length - 1]! };
}
const agents = (v: SessionView) => v.rows.filter((r) => r.kind === "agent").map((r) => (r as any).text);

describe("S15-2 parts in the console", () => {
  it("ac7_a_300_KiB_message_as_two_live_parts_reads_whole", async () => {
    const k = fakeKernel([[...partsOf(msg(2, big), 256 * 1024).map(item)]]);
    expect(partsOf(msg(2, big), 256 * 1024).length).toBeGreaterThan(1);
    const a = open(k);
    await flush();
    k.play(0);
    expect(agents(a.last())).toEqual([big]);
    expect(a.last().lastIndex).toBe(2);
    a.s.close();
  });
  it("ac7_parts_in_the_replay_read_whole_and_a_reload_doubles_nothing", async () => {
    const ps = partsOf(msg(3, big), 256 * 1024);
    const k = fakeKernel([], { preload: [user(1), ...ps] });
    const a = open(k);
    await flush();
    expect(agents(a.last())).toEqual([big]);
    const rows = a.last().rows;
    a.s.close();
    const b = open(k);
    await flush();
    expect(b.last().rows).toEqual(rows);
    k.drop();
    await flush();
    expect(agents(b.last())).toEqual([big]);
    expect(k.calls.filter((c) => c.name === "attach").at(-1)!.args).toEqual(["s1", b.last().lastIndex]);
    b.s.close();
  });
  it("ac3_a_new_attach_starts_a_clean_joiner", async () => {
    // part 1 of 2 comes live, the stream drops; the reattach replays both parts from the group's start
    const ps = partsOf(msg(3, "s".repeat(300)), 200);
    expect(ps).toHaveLength(2);
    const synced = (through: number) => JSON.stringify({ _meta: { watane: { type: "synced", through } } });
    let n = 0;
    const k = {
      ...fakeKernel([]),
      async attach(_s: string, after: number, on: (j: string) => void, end: (r: string) => void) {
        if (n++ === 0) { on(user(1)); on(synced(1)); on(item(ps[0])); setTimeout(() => end("dropped"), 0); }
        else { for (const e of [user(1), ...ps].filter((x) => JSON.parse(x).i > after)) on(e); on(synced(3)); }
        return { ok: true as const };
      },
    };
    const a = open(k as any);
    await flush(); await flush(); await flush();
    expect(n).toBe(2);
    expect(agents(a.last())).toEqual(["s".repeat(300)]);
    expect(a.last().rows.filter((r) => r.kind === "unknown")).toHaveLength(0);
    a.s.close();
  });
  it("ac6_attach_inside_a_group_starts_at_the_groups_first_part", async () => {
    const ps = partsOf(msg(20, "q".repeat(300)), 100);
    const k = fakeKernel([], { preload: [...Array.from({ length: 20 - ps.length }, (_, n) => user(n + 1)), ...ps] });
    const got: string[] = [];
    await k.attach("s1", 20 - ps.length + 2, (j) => got.push(j), () => {});
    expect(JSON.parse(got[0]).i).toBe(JSON.parse(ps[0]).i);
    expect(got.length).toBe(ps.length + 1);
  });
  it("ac7_a_group_split_over_a_page_boundary_joins", async () => {
    // 300 plain entries then a 3-part group straddling the 256 boundary of the second range
    const pre = Array.from({ length: 200 }, (_, n) => user(n + 1));
    const ps = partsOf(msg(500, "w".repeat(300)), 100);
    const first = 500 - ps.length;
    const fill = Array.from({ length: first - 200 }, (_, n) => user(201 + n));
    const k = fakeKernel([], { preload: [...pre, ...fill, ...ps] });
    vi.useFakeTimers();
    try {
      const a = open(k);
      await vi.advanceTimersByTimeAsync(3000);
      expect(agents(a.last())).toEqual(["w".repeat(300)]);
      expect(a.last().lastIndex).toBe(500);
      a.s.close();
    } finally { vi.useRealTimers(); }
  });
  it("ac7_a_page_starting_mid_group_reads_again_from_i_minus_n", async () => {
    // behind = 600: the ranges are (344,600] and (88,344]; the group at 346 has parts 342..346, so the first page starts at part 4 (after = 341, max = 4)
    const ps = partsOf(msg(346, "w".repeat(400)), 120);
    expect(ps.length).toBe(5);
    const pre = (a: number, b: number) => Array.from({ length: b - a + 1 }, (_, n) => user(a + n));
    const k = fakeKernel([], { preload: [...pre(1, 341), ...ps, ...pre(347, 600)] });
    vi.useFakeTimers();
    try {
      const a = open(k);
      await vi.advanceTimersByTimeAsync(5000);
      expect(agents(a.last())).toEqual(["w".repeat(400)]);
      expect(a.last().lastIndex).toBe(600);
      expect(a.last().rows.filter((r) => r.kind === "unknown")).toHaveLength(0);
      const reads = k.calls.filter((c) => c.name === "read");
      expect(reads.some((c) => c.args[1] === 341 && c.args[2] === 4)).toBe(true);
      expect(reads.length).toBeLessThanOrEqual(6);
      a.s.close();
    } finally { vi.useRealTimers(); }
  });
  it("ac5_a_part_row_refetch_reads_the_whole_group", async () => {
    // entry 5 was being written at the attach; its finishing update makes a `~p5` row; the store holds it as parts
    const ps = partsOf(msg(5, "r".repeat(300)), 100);
    const first = 5 - ps.length + 1;
    const store = fakeKernel([], { preload: [...Array.from({ length: first - 1 }, (_, n) => user(n + 1)), ...ps] });
    const k = {
      ...store,
      async attach(_s: string, _a: number, on: (j: string) => void) {
        on(user(1));
        on(JSON.stringify({ _meta: { watane: { type: "synced", through: 1, writing: true } } }));
        on(JSON.stringify({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: "end" }, _meta: { watane: { i: 5, at: 5 } } }));
        return { ok: true as const };
      },
    };
    const a = open(k as any);
    await flush(); await flush();
    expect(agents(a.last())).toEqual(["r".repeat(300)]);
    expect(a.last().rows.some((r) => r.key.startsWith("~p"))).toBe(false);
    a.s.close();
  });
  it("ac7_each_failure_gives_one_unknown_row_and_later_entries_show", async () => {
    const bad = [
      JSON.stringify({ v: 1, i: 2, kind: "part", body: { n: 1, m: 3, text: "{" } }),
      JSON.stringify({ v: 1, i: 3, kind: "part", body: { n: 3, m: 3, text: "x" } }),
    ];
    const k = fakeKernel([], { preload: [user(1), ...bad, user(4)] });
    const a = open(k);
    await flush();
    expect(a.last().rows.filter((r) => r.kind === "unknown")).toHaveLength(1);
    expect(a.last().rows.filter((r) => r.kind === "user")).toHaveLength(2);
    a.s.close();
  });
  it("ac7_a_multi_byte_character_on_a_cut", () => {
    const e = msg(2, "日本語日本語");
    const ps = partsOf(e, 4);
    for (const p of ps) expect(new TextEncoder().encode(JSON.parse(p).body.text).length).toBeLessThanOrEqual(4);
    expect(ps.map((p) => JSON.parse(p).body.text).join("")).toBe(JSON.stringify(e));
    expect(ps.map((p) => JSON.parse(p).i)).toEqual(ps.map((_, k) => 2 - ps.length + 1 + k));
  });
});
