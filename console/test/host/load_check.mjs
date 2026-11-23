// S36 r2 F2.3: the stage B runner's acceptance checks (test/host/load_b.mjs), kept pure so the component run proves them
// (test/host/load_check.test.ts).

/** A turn state (object or the kernel's turn JSON) that is exactly `idle {interrupted}`. */
export function isInterrupted(state) {
  let s = state;
  if (typeof s === "string") { try { s = JSON.parse(s); } catch { return false; } }
  return !!s && typeof s === "object" && s.state === "idle" && !!s.stop && typeof s.stop === "object" && s.stop.kind === "interrupted";
}

/** The common window in which every source loaded: from the latest first load line to `end` (the first Stop, ms). The window must
 * last `minSeconds`, and each source must write at least `share` of `rate` deltas and entries per second in every window
 * [t, t + `step` s) inside it, with t at the window's start, at each of the source's stamps, and at `end` - `step` s (S36 r4
 * F4.3, the deputy's rule: sliding windows, no bins, so no gap between stamps can hide); else a fault each (the first short
 * window of each source and kind). */
export function loadWindow(by, end, rate, minSeconds, share = 0.8, step = 2) {
  const faults = [];
  let from = -Infinity;
  for (const [name, xs] of by) {
    const first = xs.find((x) => x.at < end);
    if (!first) { faults.push(`${name} wrote no load line before Stop`); continue; }
    from = Math.max(from, first.at);
  }
  const seconds = (end - from) / 1000;
  if (!(seconds >= minSeconds)) faults.push(`the common load window is ${Number.isFinite(seconds) ? seconds.toFixed(1) : 0} s, under ${minSeconds} s`);
  else {
    const len = step * 1000;
    for (const [name, xs] of by) {
      const starts = [from, ...xs.map((x) => x.at), end - len].filter((t) => t >= from && t + len <= end);
      for (const kind of ["deltas", "entries"]) {
        const want = Math.floor(rate[kind] * step * share);
        const ats = xs.filter((x) => x.kind === (kind === "deltas" ? "delta" : "entry")).map((x) => x.at);
        for (const t of starts) {
          const got = ats.filter((at) => at >= t && at < t + len).length;
          if (got < want) {
            faults.push(`${name} wrote ${got} ${kind} in ${((t - from) / 1000).toFixed(2)}-${((t + len - from) / 1000).toFixed(2)} s of the common ${seconds.toFixed(1)} s, under ${want} (${share * 100}% of ${rate[kind]}/s)`);
            break;
          }
        }
      }
    }
  }
  return { from, to: end, seconds, faults };
}
