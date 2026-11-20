// S4-fix5: one rate governor for attach and read, and short pages read on.
import { describe, expect, it, vi } from "vitest";
import { openSession, type Kernel } from "./kernel";
import type { SessionView } from "./model";

const entry = (i: number) => JSON.stringify({ v: 1, i, kind: "message", at: i, body: { role: "user", content: [{ type: "text", text: `m${i}` }] } });
const base = { say: async () => 0, answer: async () => {}, interrupt: async () => {}, takeBack: async () => {} };

describe("S4-fix5", () => {
  it("ac2_one_session_never_exceeds_8_plus_4t_calls", async () => {
    vi.useFakeTimers();
    try {
      let calls = 0;
      const k: Kernel = { ...base, attach: async () => { calls++; return { behind: 1_000_000_000 }; },
        read: async (_s, after) => { calls++; return Array.from({ length: 256 }, (_, n) => entry(after + n + 1)); } };
      const s = openSession(k, "s", () => {});
      await vi.advanceTimersByTimeAsync(0);
      expect(calls).toBeLessThanOrEqual(8);
      let el = 0;
      for (const t of [1, 2, 5]) { await vi.advanceTimersByTimeAsync((t - el) * 1000); el = t; expect(calls).toBeLessThanOrEqual(8 + 4 * t); }
      s.close();
    } finally { vi.useRealTimers(); }
  });

  it("ac2_ac4_short_pages_of_100_read_on_and_300_entries_show_in_order_within_2s", async () => {
    vi.useFakeTimers();
    try {
      const all = Array.from({ length: 300 }, (_, n) => entry(n + 1));
      const k: Kernel = { ...base, attach: async (_s, after) => (after < 300 ? { behind: 300 } : { ok: true }),
        read: async (_s, after, max) => all.slice(after, after + Math.min(100, max)) };
      let last: SessionView | undefined;
      const s = openSession(k, "s", (v) => { last = v; });
      await vi.advanceTimersByTimeAsync(2000);
      expect(last!.rows.map((r) => r.key)).toHaveLength(300);
      expect(last!.lastIndex).toBe(300);
      s.close();
    } finally { vi.useRealTimers(); }
  });
});
