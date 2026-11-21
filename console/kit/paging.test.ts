// @vitest-environment node
// S4-fix4 AC3: pages are reduced incrementally, linear in the pages.
import { describe, expect, it, vi } from "vitest";

const counter = vi.hoisted(() => ({ n: 0 }));
vi.mock("./reduce", async (orig) => {
  const m = await orig<typeof import("./reduce")>();
  return { ...m, safeReduce: (v: Parameters<typeof m.safeReduce>[0], j: string) => { counter.n++; return m.safeReduce(v, j); } };
});
import { fakeKernel } from "../fake/kernel";
import { openSession } from "./kernel";

describe("S4-fix4 AC3 paging work", () => {
  it("ac3_ten_pages_are_reduced_with_linear_work", async () => {
    const pre = Array.from({ length: 2560 }, (_, n) => JSON.stringify({ v: 1, i: n + 1, kind: "message", at: n, body: { role: "user", content: [{ type: "text", text: `m${n + 1}` }] } }));
    const k = fakeKernel([], { preload: pre });
    vi.useFakeTimers();
    try {
      const s = openSession(k, "s1", () => {});
      await vi.advanceTimersByTimeAsync(1000); // the governor (S4-fix5): 8 calls at once, then 4 per second
      s.close();
    } finally { vi.useRealTimers(); }
    expect(counter.n).toBeGreaterThanOrEqual(2560); // the pages were read in
    expect(counter.n).toBeLessThanOrEqual(2560 + 256 + 10);
  });
});
