// @vitest-environment node
// TECHNICAL 9.4, 18.3: openSession against the fake kernel: reattach, reload, behind.
import { describe, expect, it, vi } from "vitest";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { openSession } from "./kernel";
import type { SessionView } from "./model";

const flush = () => new Promise((r) => setTimeout(r, 80)); // P14: a view reaches the page once per frame (a 16 ms timer here)
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
    await flush();
    const before = a.last();
    expect(before.lastIndex).toBe(6);
    k.drop();
    await flush();
    expect(k.calls.filter((c) => c.name === "attach")[1]!.args).toEqual(["s1", before.lastIndex]);
    expect(a.last().rows).toEqual(before.rows);
    k.play(2);
    await flush();
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
  it("ac3_stuck_behind_backs_off_1s_2s_4s_and_stops_after_5_with_an_unknown_row", async () => {
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
      await vi.advanceTimersByTimeAsync(1_000); // t=2.1 s: 2nd wait is 2 s, ends at 3 s
      expect(attaches).toBe(2);
      await vi.advanceTimersByTimeAsync(1_000);
      expect(attaches).toBe(3);
      await vi.advanceTimersByTimeAsync(3_000); // 4 s wait ends at 7 s
      expect(attaches).toBe(3);
      await vi.advanceTimersByTimeAsync(1_000);
      expect(attaches).toBe(4);
      await vi.advanceTimersByTimeAsync(100_000);
      expect(attaches).toBe(5);
      expect(last!.rows.filter((r) => r.kind === "unknown" && String(r.raw).includes("not past the last index"))).toHaveLength(5);
      s.close();
    } finally { vi.useRealTimers(); }
  });
  it("ac3_non_integer_behind_is_named_not_an_index_and_never_paged", async () => {
    vi.useFakeTimers();
    try {
      for (const behind of [1.5, Number.NaN, 2 ** 60]) {
        let reads = 0;
        const k = { async attach() { return { behind }; }, read: async () => { reads++; return ["{}"]; }, say: async () => 0, answer: async () => {}, interrupt: async () => {}, takeBack: async () => {} };
        let last: SessionView | undefined;
        const s = openSession(k, "s1", (v) => { last = v; });
        await vi.advanceTimersByTimeAsync(30);
        s.close();
        expect(reads).toBe(0);
        expect(last!.rows.some((r) => r.kind === "unknown" && String(r.raw).includes("not an index"))).toBe(true);
      }
    } finally { vi.useRealTimers(); }
  });
});

describe("S4-fix3 AC2 reattach policy", () => {
  const mk = (onAttach: (n: number, on: (j: string) => void, end: (r: string) => void) => Promise<{ ok: true } | { behind: number }>) => {
    const c = { attaches: 0 };
    const k = { async attach(_s: string, _a: number, on: (j: string) => void, end: (r: string) => void) { c.attaches++; return onAttach(c.attaches, on, end); },
      read: async () => [] as string[], say: async () => 0, answer: async () => {}, interrupt: async () => {}, takeBack: async () => {} };
    return { c, k };
  };
  it("ac2_end_right_after_ok_waits_1s_then_2s", async () => {
    vi.useFakeTimers();
    try {
      const { c, k } = mk(async (_n, _on, end) => { queueMicrotask(() => end("consumer-failed")); return { ok: true }; });
      const s = openSession(k, "s1", () => {});
      await vi.advanceTimersByTimeAsync(900);
      expect(c.attaches).toBe(1);
      await vi.advanceTimersByTimeAsync(200);
      expect(c.attaches).toBe(2);
      await vi.advanceTimersByTimeAsync(1_800);
      expect(c.attaches).toBe(2);
      await vi.advanceTimersByTimeAsync(200);
      expect(c.attaches).toBe(3);
      s.close();
    } finally { vi.useRealTimers(); }
  });
  it("ac2_end_after_synced_reattaches_at_once_and_only_once_then_waits_and_resets", async () => {
    vi.useFakeTimers();
    try {
      const synced = JSON.stringify({ _meta: { watane: { type: "synced", through: 0 } } });
      const { c, k } = mk(async (n, on, end) => {
        if (n === 1) queueMicrotask(() => { on(synced); end("idle-timeout"); });
        else queueMicrotask(() => end("idle-timeout"));
        return { ok: true };
      });
      const s = openSession(k, "s1", () => {});
      await vi.advanceTimersByTimeAsync(0);
      expect(c.attaches).toBe(2); // at once, once
      await vi.advanceTimersByTimeAsync(900);
      expect(c.attaches).toBe(2);
      await vi.advanceTimersByTimeAsync(200);
      expect(c.attaches).toBe(3);
      s.close();
    } finally { vi.useRealTimers(); }
  });
  it("ac2_streams_that_sync_and_end_at_once_reattach_at_once_only_once_in_a_row", async () => {
    vi.useFakeTimers();
    try {
      const synced = JSON.stringify({ _meta: { watane: { type: "synced", through: 0 } } });
      const { c, k } = mk(async (n, on, end) => { if (n < 1_000) queueMicrotask(() => { on(synced); end("idle-timeout"); }); return { ok: true }; });
      const s = openSession(k, "s1", () => {});
      await vi.advanceTimersByTimeAsync(0);
      expect(c.attaches).toBe(2);
      await vi.advanceTimersByTimeAsync(900);
      expect(c.attaches).toBe(2);
      await vi.advanceTimersByTimeAsync(200);
      expect(c.attaches).toBe(4); // 3 after the wait syncs and ends: 4 at once, then a wait again
      s.close();
    } finally { vi.useRealTimers(); }
  });
  it("ac2_stream_that_ends_five_times_without_sync_stops_with_an_unknown_row", async () => {
    vi.useFakeTimers();
    try {
      const { c, k } = mk(async (_n, _on, end) => { queueMicrotask(() => end("lost-grant")); return { ok: true }; });
      let last: SessionView | undefined;
      const s = openSession(k, "s1", (v) => { last = v; });
      await vi.advanceTimersByTimeAsync(200_000);
      expect(c.attaches).toBe(5);
      expect(last!.rows.some((r) => r.kind === "unknown" && String(r.raw).includes("lost-grant"))).toBe(true);
      s.close();
    } finally { vi.useRealTimers(); }
  });
  it("ac2_failed_attach_backs_off_and_a_sync_resets_the_count", async () => {
    vi.useFakeTimers();
    try {
      const synced = JSON.stringify({ _meta: { watane: { type: "synced", through: 0 } } });
      const { c, k } = mk(async (n, on, end) => {
        if (n === 4) { queueMicrotask(() => { on(synced); on(JSON.stringify({ v: 1, i: 1, kind: "message", at: 1, body: { role: "user", content: [{ type: "text", text: "live" }] } })); end("idle-timeout"); }); return { ok: true }; }
        throw new Error("down");
      });
      const s = openSession(k, "s1", () => {});
      await vi.advanceTimersByTimeAsync(1_000 + 2_000 + 4_000 + 10); // t=7.01 s: 4 syncs and ends, 5 at once fails
      expect(c.attaches).toBe(5);
      await vi.advanceTimersByTimeAsync(900); // reset: 1 s, not the 8 s of a 4th failure in a row
      expect(c.attaches).toBe(5);
      await vi.advanceTimersByTimeAsync(200);
      expect(c.attaches).toBe(6);
      await vi.advanceTimersByTimeAsync(1_800); // then 2 s
      expect(c.attaches).toBe(6);
      await vi.advanceTimersByTimeAsync(200);
      expect(c.attaches).toBe(7);
      s.close();
    } finally { vi.useRealTimers(); }
  });
});

describe("S4-fix3 AC3 old connection", () => {
  it("ac3_late_end_timer_and_page_of_an_old_connection_leave_the_new_view_alone", async () => {
    vi.useFakeTimers();
    try {
      const ends: Array<(r: string) => void> = [];
      let release!: (p: string[]) => void;
      let n = 0;
      const k = {
        async attach(_s: string, _a: number, _on: (j: string) => void, end: (r: string) => void) {
          n++; ends.push(end);
          if (n === 1) return { behind: 1 };
          return { ok: true } as const;
        },
        read: () => new Promise<string[]>((r) => { release = r; }),
        say: async () => 0, answer: async () => {}, interrupt: async () => {}, takeBack: async () => {},
      };
      const views: SessionView[] = [];
      const s = openSession(k, "s1", (v) => views.push(v));
      await vi.advanceTimersByTimeAsync(0);
      expect(n).toBe(1); // old connection waits on its page
      ends[0]("consumer-failed"); // its end fires: schedules a retry (1 s)
      await vi.advanceTimersByTimeAsync(1_100);
      expect(n).toBe(2); // new connection
      const shown = views.length;
      release([JSON.stringify({ i: 0, type: "x" })]); // old page lands late
      await vi.advanceTimersByTimeAsync(10);
      ends[0]("again");
      await vi.advanceTimersByTimeAsync(60_000);
      expect(views.length).toBe(shown);
      expect(n).toBe(2);
      s.close();
    } finally { vi.useRealTimers(); }
  });
  it("ac3_late_event_and_end_of_an_old_synced_connection_leave_the_new_one_alone", async () => {
    vi.useFakeTimers();
    try {
      const synced = JSON.stringify({ _meta: { watane: { type: "synced", through: 0 } } });
      const ons: Array<(j: string) => void> = [];
      const ends: Array<(r: string) => void> = [];
      let n = 0;
      const k = {
        async attach(_s: string, _a: number, on: (j: string) => void, end: (r: string) => void) {
          n++; ons.push(on); ends.push(end);
          if (n === 1) queueMicrotask(() => { on(synced); end("idle-timeout"); }); // 2 at once
          if (n === 2) queueMicrotask(() => end("lost-grant")); // 3 after 1 s
          return { ok: true } as const;
        },
        read: async () => [] as string[], say: async () => 0, answer: async () => {}, interrupt: async () => {}, takeBack: async () => {},
      };
      const views: SessionView[] = [];
      const s = openSession(k, "s1", (v) => views.push(v));
      await vi.advanceTimersByTimeAsync(1_100);
      expect(n).toBe(3);
      const shown = views.length;
      ons[0](JSON.stringify({ i: 0, type: "x" }));
      ends[0]("again");
      await vi.advanceTimersByTimeAsync(60_000);
      expect(views.length).toBe(shown);
      expect(n).toBe(3);
      s.close();
    } finally { vi.useRealTimers(); }
  });
});

describe("S4-fix4", () => {
  const synced = JSON.stringify({ _meta: { watane: { type: "synced", through: 0 } } });
  const entry = (n: number) => JSON.stringify({ v: 1, i: n, kind: "message", at: n, body: { role: "user", content: [{ type: "text", text: `m${n}` }] } });
  it("ac4_streams_that_sync_and_end_at_once_back_off_and_stop_with_an_unknown_row", async () => {
    vi.useFakeTimers();
    try {
      let attaches = 0;
      const k = {
        async attach(_s: string, _a: number, on: (j: string) => void, end: (r: string) => void) { attaches++; queueMicrotask(() => { on(synced); end("idle-timeout"); }); return { ok: true } as const; },
        read: async () => [] as string[], say: async () => 0, answer: async () => {}, interrupt: async () => {}, takeBack: async () => {},
      };
      let last: SessionView | undefined;
      const s = openSession(k, "s1", (v) => { last = v; });
      await vi.advanceTimersByTimeAsync(200_000);
      expect(attaches).toBeGreaterThanOrEqual(5);
      expect(attaches).toBeLessThanOrEqual(10);
      const at = attaches;
      await vi.advanceTimersByTimeAsync(200_000);
      expect(attaches).toBe(at);
      expect(last!.rows.some((r) => r.kind === "unknown")).toBe(true);
      s.close();
    } finally { vi.useRealTimers(); }
  });
  it("ac4_a_stream_open_10_s_after_synced_resets_the_count", async () => {
    vi.useFakeTimers();
    try {
      let attaches = 0;
      let end!: (r: string) => void;
      const k = {
        async attach(_s: string, _a: number, on: (j: string) => void, e: (r: string) => void) { attaches++; end = e; queueMicrotask(() => on(synced)); return { ok: true } as const; },
        read: async () => [] as string[], say: async () => 0, answer: async () => {}, interrupt: async () => {}, takeBack: async () => {},
      };
      const s = openSession(k, "s1", () => {});
      await vi.advanceTimersByTimeAsync(11_000);
      end("idle-timeout"); // healthy: reattaches at once
      await vi.advanceTimersByTimeAsync(10);
      expect(attaches).toBe(2);
      await vi.advanceTimersByTimeAsync(11_000);
      end("idle-timeout");
      await vi.advanceTimersByTimeAsync(10);
      expect(attaches).toBe(3); // the count was reset, so the quick reattach is allowed again
      s.close();
    } finally { vi.useRealTimers(); }
  });
  it("ac4_a_stream_open_only_9_s_after_synced_does_not_reset_the_count", async () => {
    vi.useFakeTimers();
    try {
      let attaches = 0;
      let end!: (r: string) => void;
      const k = {
        async attach(_s: string, _a: number, on: (j: string) => void, e: (r: string) => void) { attaches++; end = e; queueMicrotask(() => on(synced)); return { ok: true } as const; },
        read: async () => [] as string[], say: async () => 0, answer: async () => {}, interrupt: async () => {}, takeBack: async () => {},
      };
      const s = openSession(k, "s1", () => {});
      await vi.advanceTimersByTimeAsync(9_000);
      end("idle-timeout");
      await vi.advanceTimersByTimeAsync(10);
      expect(attaches).toBe(2); // the one immediate reattach
      await vi.advanceTimersByTimeAsync(9_000);
      end("idle-timeout");
      await vi.advanceTimersByTimeAsync(10);
      expect(attaches).toBe(2); // not reset: this one waits 1 s
      await vi.advanceTimersByTimeAsync(1_000);
      expect(attaches).toBe(3);
      s.close();
    } finally { vi.useRealTimers(); }
  });
  it("ac4_a_late_end_of_an_old_stream_does_not_cancel_the_new_streams_reset", async () => {
    vi.useFakeTimers();
    try {
      const ends: Array<(r: string) => void> = [];
      const k = {
        async attach(_s: string, _a: number, on: (j: string) => void, e: (r: string) => void) {
          ends.push(e);
          queueMicrotask(() => { on(synced); if (ends.length === 1) e("idle-timeout"); });
          return { ok: true } as const;
        },
        read: async () => [] as string[], say: async () => 0, answer: async () => {}, interrupt: async () => {}, takeBack: async () => {},
      };
      const s = openSession(k, "s1", () => {});
      await vi.advanceTimersByTimeAsync(10);
      expect(ends.length).toBe(2); // 2 is the immediate reattach, open and synced
      ends[0]("late");
      await vi.advanceTimersByTimeAsync(11_000);
      ends[1]("idle-timeout");
      await vi.advanceTimersByTimeAsync(10);
      expect(ends.length).toBe(3); // 2 stayed open 10 s: reset, so it reattaches at once
      s.close();
    } finally { vi.useRealTimers(); }
  });
});

describe("S4-fix4 AC2 page budget", () => {
  it("ac2_pages_that_each_add_one_index_read_on_at_the_governors_pace", async () => {
    vi.useFakeTimers();
    try {
      let attaches = 0;
      let reads = 0;
      const k = {
        async attach() { attaches++; return { behind: 2_560 }; }, // budget: ceil(2560 / 256) = 10 pages
        read: async (_s: string, after: number) => { reads++; return [JSON.stringify({ v: 1, i: after + 1, kind: "message", at: 1, body: { role: "user", content: [{ type: "text", text: "x" }] } })]; },
        say: async () => 0, answer: async () => {}, interrupt: async () => {}, takeBack: async () => {},
      };
      let last: SessionView | undefined;
      const s = openSession(k, "s1", (v) => { last = v; });
      await vi.advanceTimersByTimeAsync(750); // the governor (S4-fix5) lets 8 calls through at once, then 4 per second
      expect(attaches).toBe(1);
      expect(reads).toBe(10); // each page adds one index in range, read one after the other at the governor's pace
      await vi.advanceTimersByTimeAsync(100_000);
      expect(reads).toBeLessThanOrEqual(8 + 4 * 100.75); // S4-fix5: short pages are read on, the governor is the pace
      expect(attaches).toBe(1); // no budget: a page that adds an index in range is progress (S4-fix5)
      expect(reads).toBeGreaterThan(256); // past the first range of 256 one-entry pages
      expect(last!.rows.length).toBeGreaterThanOrEqual(256);
      s.close();
    } finally { vi.useRealTimers(); }
  });
});
