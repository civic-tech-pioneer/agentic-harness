// S36 r2 F2.3: the stage B runner accepts latency only when all 8 sources loaded at the expected rates over a
// common window, and times Stop to the exact `idle {interrupted}` (TECHNICAL 17 row B, test/host/load_b.mjs).
import { assert, describe, expect, it } from "vitest";
// @ts-expect-error: a plain ES module of the host runner
import { isInterrupted, loadWindow } from "./load_check.mjs";

const RATE = { deltas: 50, entries: 5 };
/** A fake's stamps at the rates: a delta every 20 ms and an entry every 200 ms from `from` to `to` (ms). */
const source = (from: number, to: number) => {
  const xs: { at: number; kind: string; n: number }[] = [];
  for (let t = from, n = 0; t < to; t += 20, n++) {
    xs.push({ at: t, kind: "delta", n: xs.length });
    if (n % 10 === 9) xs.push({ at: t + 1, kind: "entry", n: xs.length });
  }
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

describe("S36 r5 load checks", () => {
  it("regress_S36_r5_a_steady_source_a_little_under_the_rate_passes (F5.5, the r6 F6.3 ruling)", () => {
    // Codex's r5 control: deltas every 25.125 ms and an entry 20 ms after every eighth delta. Under the r6 ruling (no gap over
    // twice the nominal spacing) a source that loads steadily, if a little slower, is loading: it passes
    const by = new Map<string, { at: number; kind: string; n: number }[]>();
    for (let k = 0; k < 8; k++) {
      const xs: { at: number; kind: string; n: number }[] = [];
      for (let i = 0; i * 25.125 < 8000; i++) {
        xs.push({ at: i * 25.125, kind: "delta", n: xs.length });
        if (i % 8 === 7) xs.push({ at: i * 25.125 + 20, kind: "entry", n: xs.length });
      }
      xs.sort((a, b) => a.at - b.at);
      by.set(`source ${k}`, xs);
    }
    expect(loadWindow(by, 8000, { deltas: 50, entries: 5 }, 6).faults).toEqual([]);
  });
});

describe("S36 r6 load checks", () => {
  /** 8 sources stamping a delta every `d` ms and an entry every `e` ms from 0 to `to`, with one gap of `gap` ms (kind `k`) at 3 s. */
  const run = (d: number, e: number, to: number, gap?: { kind: "delta" | "entry"; ms: number }) => {
    const by = new Map<string, { at: number; kind: string; n: number }[]>();
    for (let s = 0; s < 8; s++) {
      const xs: { at: number; kind: string; n: number }[] = [];
      for (const [kind, every] of [["delta", d], ["entry", e]] as const) {
        for (let t = 0; t < to; t += every) {
          if (gap?.kind === kind && t > 3000 && t < 3000 + gap.ms) continue;
          xs.push({ at: t, kind, n: xs.length });
        }
      }
      xs.sort((a, b) => a.at - b.at);
      by.set(`source ${s}`, xs);
    }
    return by;
  };
  it("regress_S36_r6_the_real_runs_jitter_passes (F6.3)", () => {
    // the real stage B run's worst gaps: 28.5 ms between deltas, 205 ms between entries, window edges included
    expect(loadWindow(run(28.5, 205, 8000), 8000, RATE, 6).faults).toEqual([]);
  });
  it("regress_S36_r6_an_explicit_two_times_limit_still_fails (F6.3)", () => {
    expect(loadWindow(run(20, 200, 8000, { kind: "delta", ms: 41 }), 8000, RATE, 6, 2).faults.length).toBeGreaterThan(0);
    expect(loadWindow(run(20, 200, 8000, { kind: "entry", ms: 401 }), 8000, RATE, 6, 2).faults.length).toBeGreaterThan(0);
    expect(loadWindow(run(20, 200, 8000), 7980 + 41, RATE, 6, 2).faults.length).toBeGreaterThan(0); // the window's right edge
  });
});

describe("S144 Stage B/C source spacing", () => {
  it("ac2_three_times_spacing_accepts_jitter_and_covers_both_window_edges", () => {
    const steady = source(0, 8000);
    // The latest source opens the window at 100 ms. The first source's initial
    // delta after that is 160 ms, and its final delta precedes Stop by 60 ms.
    const jittered = steady.filter(({ at, kind }) => kind !== "delta" ||
      (at < 100 || at >= 160) && at !== 3020 && at !== 3040 && at < 7940);
    const by = new Map([["first", jittered], ["last", source(100, 7970)]]);
    expect(loadWindow(by, 7980, RATE, 6).faults).toEqual([]);
    for (const at of [160, 3060, 7920]) {
      by.set("first", jittered.filter(({ at: stamp, kind }) => kind !== "delta" || stamp !== at));
      expect(loadWindow(by, 7980, RATE, 6).faults).toHaveLength(1);
    }
    // 600 ms is the corresponding maximum for entries at 5/s.
    by.set("first", steady.filter(({ at, kind }) => kind !== "entry" || (at !== 3181 && at !== 3381)));
    expect(loadWindow(by, 7980, RATE, 6).faults).toEqual([]);
    by.set("first", by.get("first")!.filter(({ at, kind }) => kind !== "entry" || at !== 3581));
    expect(loadWindow(by, 7980, RATE, 6).faults).toHaveLength(1);
  });
});
