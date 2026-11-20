// TECHNICAL 9.4, 18.3: openSession against the fake kernel: reattach, reload, behind.
import { describe, expect, it } from "vitest";
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
