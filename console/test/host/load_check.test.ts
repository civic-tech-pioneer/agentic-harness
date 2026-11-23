// S36 r2 F2.3: the stage B runner accepts latency only when all 8 sources loaded at the expected rates over a
// common window, and times Stop to the exact `idle {interrupted}` (TECHNICAL 17 row B, test/host/load_b.mjs).
import { describe, expect, it } from "vitest";
// @ts-expect-error: a plain ES module of the host runner
import { isInterrupted, loadWindow } from "./load_check.mjs";

const RATE = { deltas: 50, entries: 5 };
/** A fake's stamps: `deltas` and `entries` per second from `from` to `to` (ms). */
const source = (from: number, to: number) => {
  const xs: { at: number; kind: string; n: number }[] = [];
  for (let t = from, n = 0; t < to; t += 20, n++) xs.push({ at: t, kind: n % 10 === 9 ? "entry" : "delta", n });
  return xs;
};

describe("S36 r2 load checks", () => {
  it("regress_S36_r2_load_rejects_an_idle_source (F2.3)", () => {
    const by = new Map<string, unknown[]>();
    for (let k = 0; k < 4; k++) { by.set(`load ${k}`, source(1000, 9000)); by.set(`load client ${k}`, source(1000, 9000)); }
    expect(loadWindow(by, 9000, RATE, 5).faults).toEqual([]);
    by.set("load client 3", []); // Codex's negative control: one client never got `go`
    const w = loadWindow(by, 9000, RATE, 5);
    expect(w.faults.length).toBeGreaterThan(0);
    expect(w.faults.join(" ")).toContain("load client 3");
  });
  it("regress_S36_r2_load_rejects_too_short_a_common_window (F2.3)", () => {
    const by = new Map<string, unknown[]>();
    for (let k = 0; k < 4; k++) { by.set(`load ${k}`, source(1000, 9000)); by.set(`load client ${k}`, source(k === 2 ? 6000 : 1000, 9000)); }
    expect(loadWindow(by, 9000, RATE, 5).faults.length).toBeGreaterThan(0);
  });
  it("regress_S36_r2_stop_matches_exactly_idle_interrupted (F2.3)", () => {
    expect(isInterrupted({ state: "idle", stop: { kind: "interrupted" } })).toBe(true);
    expect(isInterrupted(JSON.stringify({ state: "idle", stop: { kind: "interrupted" }, seconds: 3 }))).toBe(true);
    expect(isInterrupted({ state: "idle", stop: { kind: "failed", error: "interrupted" } })).toBe(false);
    expect(isInterrupted({ state: "running", note: "interrupted" })).toBe(false);
  });
});

describe("S36 r3 load checks", () => {
  it("regress_S36_r3_load_rejects_disjoint_bursts (F3.5)", () => {
    // Codex's control: one stamp at 0 for each of 8 sources, then 320 deltas and 32 entries per source in disjoint bursts
    const by = new Map<string, { at: number; kind: string; n: number }[]>();
    for (let k = 0; k < 8; k++) {
      const xs = [{ at: 0, kind: "delta", n: 0 }];
      const at = 1000 + (k < 4 ? 0 : 3500); // the two owners' sources load in turn, never together
      for (let n = 0; n < 352; n++) xs.push({ at: at + n * 3, kind: n % 11 === 10 ? "entry" : "delta", n: n + 1 });
      by.set(`source ${k}`, xs);
    }
    expect(loadWindow(by, 8000, RATE, 6).faults.length).toBeGreaterThan(0);
  });
  it("regress_S36_r3_steady_sources_pass_the_subwindow_check (F3.5)", () => {
    const by = new Map<string, unknown[]>();
    for (let k = 0; k < 8; k++) by.set(`source ${k}`, source(0, 8000));
    expect(loadWindow(by, 8000, RATE, 6).faults).toEqual([]);
  });
});

describe("S36 r4 load checks", () => {
  it("regress_S36_r4_load_rejects_a_gap_between_bins (F4.3)", () => {
    // Codex's control: each of 8 sources has one initial stamp, then 80 deltas and 8 entries in each of four clusters near
    // 0, 3900, 4000 and 7900 ms: every 2 s bin is full, but [1000, 3000) is empty
    const by = new Map<string, { at: number; kind: string; n: number }[]>();
    for (let k = 0; k < 8; k++) {
      const xs = [{ at: 0, kind: "delta", n: 0 }];
      for (const c of [0, 3900, 4000, 7900]) for (let n = 0; n < 88; n++) xs.push({ at: c + n, kind: n % 11 === 10 ? "entry" : "delta", n: xs.length });
      by.set(`source ${k}`, xs);
    }
    expect(loadWindow(by, 8000, RATE, 6).faults.length).toBeGreaterThan(0);
  });
});
