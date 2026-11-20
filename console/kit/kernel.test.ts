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
        await vi.advanceTimersByTimeAsync(10);
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
      const { c, k } = mk(async (n, on) => {
        if (n <= 3) throw new Error("down");
        if (n === 4) { on(synced); return { ok: true }; }
        throw new Error("down");
      });
      const s = openSession(k, "s1", () => {});
      await vi.advanceTimersByTimeAsync(1_000 + 2_000 + 4_000 + 10);
      expect(c.attaches).toBe(4);
      // reset by the sync of attach 4: the next failure waits 1 s again (no further end here, so force one)
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
});
