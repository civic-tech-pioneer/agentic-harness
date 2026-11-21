// @vitest-environment node
// S7-2 AC6 (TECHNICAL 18.7): the long fixture in fake/long.ts: n finished entries, one open ask near the start, then a running turn with a delta stream.
import { describe, expect, it } from "vitest";
import { fakeKernel } from "../fake/kernel";
import { longStory } from "../fake/long";
import { emptyView } from "./model";
import { reduce } from "./reduce";

describe("longStory", () => {
  it("ac6_n_finished_entries_of_every_kind_one_open_ask_near_the_start_then_a_running_turn_and_deltas", () => {
    const script = longStory(400, 30);
    expect(script).toHaveLength(31);
    const v = script[0]!.reduce(reduce, emptyView);
    expect(v.lastIndex).toBe(400);
    const kinds = new Set(v.rows.map((r) => r.kind));
    for (const k of ["user", "agent", "thought", "explored", "tool", "decision"]) expect(kinds).toContain(k);
    expect(v.rows.some((r) => r.kind === "tool" && r.call.kind === "edit" && r.call.content.some((c) => c.type === "diff"))).toBe(true);
    expect(v.rows.some((r) => r.kind === "tool" && r.call.kind === "execute" && r.call.content.some((c) => c.type === "content"))).toBe(true);
    expect(v.rows.some((r) => r.kind === "decision" && /^Allowed:/.test(r.what))).toBe(true); // a decided ask
    expect(v.openAsks).toHaveLength(1);
    const at = v.rows.findIndex((r) => r.kind === "tool" && r.ask && !r.ask.decision);
    expect(at).toBeGreaterThanOrEqual(0);
    expect(at).toBeLessThan(10); // near the start, not at the end
    expect(v.turn?.state).toBe("running");
    const streamed = script.slice(1).reduce((a, m) => m.reduce(reduce, a), v);
    expect(streamed.rows.at(-1)).toMatchObject({ kind: "agent", writing: true });
    expect(streamed.lastIndex).toBe(400);
  });

  it("ac6_a_fake_kernel_plays_the_first_step_into_n_stored_entries", async () => {
    const k = fakeKernel(longStory(300, 3));
    k.play(0);
    const got = await k.read("s", 0, 256);
    expect(got).toHaveLength(256);
    const all = await k.read("s", 255, 256);
    expect(JSON.parse(all.at(-1)!).i).toBe(300);
  });
});

describe("longKernel (the page's ?long=N)", () => {
  it("ac6_attach_is_behind_then_the_stream_starts_after_the_session_is_attached", async () => {
    const { vi } = await import("vitest");
    const { longKernel } = await import("../fake/page");
    vi.useFakeTimers();
    try {
      const k = longKernel(300);
      const msgs: string[] = [];
      expect(await k.attach("s", 0, (m) => msgs.push(m), () => {})).toEqual({ behind: 300 });
      const r = await k.attach("s", 300, (m) => msgs.push(m), () => {});
      expect(r).toEqual({ ok: true });
      const before = msgs.length;
      await vi.advanceTimersByTimeAsync(200);
      expect(msgs.length).toBeGreaterThan(before + 5); // deltas, every 20 ms
    } finally { vi.useRealTimers(); }
  });
});
