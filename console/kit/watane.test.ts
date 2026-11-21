// S32-3 AC1-AC3: the Kernel over `window.watane`.
import { describe, expect, it, vi } from "vitest";
import { wataneKernel } from "./watane";
import { isAskRefusal } from "./model";
import { openSession } from "./kernel";
import { emptyView } from "./model";
import { entry, fakeWatane, items, synced } from "./watane_fake";

const noop = () => {};

describe("S32-3 AC1", () => {
  it("ac1_attach_sends_the_call_and_maps_ok_and_behind", async () => {
    const f = fakeWatane({ "sessions.attach": (a) => (a.after === 0 ? { ok: "st1" } : { err: { behind: 300 } }) });
    const k = wataneKernel(f.w);
    expect(await k.attach("s1", 0, noop, noop)).toEqual({ ok: true });
    expect(f.calls[0]).toEqual({ fn: "sessions.attach", args: { session: "s1", after: 0, caps: "{}" } });
    expect(await k.attach("s1", 5, noop, noop)).toEqual({ behind: 300 });
  });
  it("ac1_other_errors_reject_with_the_case_as_kind", async () => {
    const f = fakeWatane({ "sessions.attach": () => ({ err: { backend: "no provider" } }), "sessions.say": () => ({ err: "unknown-session" }) });
    const k = wataneKernel(f.w);
    const e = await k.attach("s", 0, noop, noop).catch((x) => x);
    expect(e).toBeInstanceOf(Error);
    expect(e.kind).toBe("backend");
    expect(e.message).toContain("no provider");
    expect((await k.say("s", "k", "hi").catch((x) => x)).kind).toBe("unknown-session");
  });
  it("ac1_a_host_failure_rejects", async () => {
    const k = wataneKernel(fakeWatane({}).w);
    await expect(k.say("s", "k", "x")).rejects.toThrow(/sessions.say/);
  });
  it("ac1_read_and_the_calls_use_the_names", async () => {
    const f = fakeWatane({
      "sessions.read": () => ({ ok: { entries: [entry(1, "a")], last: 1 } }),
      "sessions.say": () => ({ ok: 7 }), "sessions.answer": () => ({ ok: null }),
      "sessions.interrupt": () => ({ ok: null }), "sessions.take-back": () => ({ ok: null }),
    });
    const k = wataneKernel(f.w);
    expect(await k.read("s", 0, 10)).toEqual([entry(1, "a")]);
    expect(await k.say("s", "k", "hi")).toBe(7);
    await k.answer("s", "k", "a1", '{"allow":true}');
    await k.interrupt("s");
    await k.takeBack("s", "al1");
    expect(f.calls.map((c) => c.fn)).toEqual(["sessions.read", "sessions.say", "sessions.answer", "sessions.interrupt", "sessions.take-back"]);
    expect(f.calls[2].args).toEqual({ session: "s", key: "k", ask: "a1", decision: '{"allow":true}' });
    expect(f.calls[3].args).toEqual({ session: "s", replacement: null });
    expect(f.calls[4].args).toEqual({ session: "s", allow: "al1" });
  });
  it("ac1_refused_carries_refusals_for_isAskRefusal", async () => {
    const f = fakeWatane({ "sessions.answer": () => ({ err: { refused: [{ field: "ask", backend: "", model: "", reason: "decided" }] } }) });
    const e = await wataneKernel(f.w).answer("s", "k", "a", "{}").catch((x) => x);
    expect(e.kind).toBe("refused");
    expect(isAskRefusal(e)).toBe(true);
  });
});

describe("S32-3 AC2", () => {
  it("ac2_items_go_in_seq_order_and_a_redelivered_batch_shows_nothing_twice", async () => {
    const f = fakeWatane({ "sessions.attach": () => ({ ok: "st" }) });
    const got: string[] = [];
    await wataneKernel(f.w).attach("s", 0, (j) => got.push(j), noop);
    f.push({ ...items([2, "b"], [1, "a"]), stream: "st" });
    f.push({ ...items([1, "a"], [2, "b"], [3, "c"]), stream: "st" });
    expect(got).toEqual(["a", "b", "c"]);
  });
  it("ac2_items_before_the_attach_answers_are_held_then_delivered_in_order", async () => {
    let answer!: (v: unknown) => void;
    const f = fakeWatane({ "sessions.attach": () => new Promise((r) => { answer = r; }) });
    const got: string[] = [];
    const p = wataneKernel(f.w).attach("s", 0, (j) => got.push(j), noop);
    f.push({ ...items([2, "b"]), stream: "st" });
    f.push({ ...items([1, "a"]), stream: "st" });
    f.push({ ...items([1, "x"]), stream: "other" });
    expect(got).toEqual([]);
    answer({ ok: "st" });
    await p;
    expect(got).toEqual(["a", "b"]);
  });
  it("ac2_held_items_expire_after_10s_and_cap_at_1000", async () => {
    vi.useFakeTimers();
    try {
      let answer!: (v: unknown) => void;
      const f = fakeWatane({ "sessions.attach": () => new Promise((r) => { answer = r; }) });
      const got: string[] = [];
      const k = wataneKernel(f.w);
      const p = k.attach("s", 0, (j) => got.push(j), noop);
      f.push({ ...items(...Array.from({ length: 1200 }, (_, n): [number, string] => [n + 1, "x"])), stream: "st" });
      answer({ ok: "st" });
      await p;
      expect(got.length).toBe(1000);
      let answer2!: (v: unknown) => void;
      f.w.call = async () => new Promise((r) => { answer2 = r; });
      const got2: string[] = [];
      const p2 = k.attach("s", 0, (j) => got2.push(j), noop);
      f.push({ ...items([1, "old"]), stream: "st2" });
      vi.advanceTimersByTime(10_001);
      answer2({ ok: "st2" });
      await p2;
      expect(got2).toEqual([]);
    } finally { vi.useRealTimers(); }
  });
  it("ac2_items_of_unknown_streams_are_ignored", async () => {
    const f = fakeWatane({ "sessions.attach": () => ({ ok: "st" }) });
    const got: string[] = [];
    await wataneKernel(f.w).attach("s", 0, (j) => got.push(j), noop);
    f.push({ ...items([1, "x"]), stream: "zzz" });
    expect(got).toEqual([]);
  });
});

describe("S32-3 AC3", () => {
  it("ac3_stream_end_calls_onEnd_once_with_the_case_and_later_items_are_ignored", async () => {
    const f = fakeWatane({ "sessions.attach": () => ({ ok: "st" }) });
    const got: string[] = [], ends: string[] = [];
    await wataneKernel(f.w).attach("s", 0, (j) => got.push(j), (r) => ends.push(r));
    f.push({ type: "stream-end", stream: "st", outcome: { failed: "boom" } });
    f.push({ type: "stream-end", stream: "st", outcome: "lagging" });
    f.push({ ...items([1, "a"]), stream: "st" });
    expect(ends).toEqual(["failed"]);
    expect(got).toEqual([]);
  });
  it("ac3_lagging_then_reattach_shows_1_to_5_once_each", async () => {
    vi.useFakeTimers();
    try {
      let n = 0;
      const f = fakeWatane({ "sessions.attach": () => ({ ok: `st${++n}` }) });
      let view = emptyView;
      const h = openSession(wataneKernel(f.w), "s", (v) => { view = v; });
      await vi.advanceTimersByTimeAsync(50);
      f.push({ ...items([1, entry(1, "one")], [2, entry(2, "two")], [3, entry(3, "three")], [4, synced(0)]), stream: "st1" });
      await vi.advanceTimersByTimeAsync(50);
      f.push({ type: "stream-end", stream: "st1", outcome: "lagging" });
      await vi.advanceTimersByTimeAsync(500);
      expect(f.calls.map((c) => c.args.after)).toEqual([0, 3]);
      f.push({ ...items([1, entry(4, "four")], [2, entry(5, "five")]), stream: "st2" });
      await vi.advanceTimersByTimeAsync(50);
      expect(view.rows.map((r) => r.key)).toHaveLength(5);
      h.close();
    } finally { vi.useRealTimers(); }
  });
});
