// S36 r2 F2.3: the stage B runner's acceptance checks (test/host/load_b.mjs), kept pure so the component run proves them
// (test/host/load_check.test.ts).

/** A turn state (object or the kernel's turn JSON) that is exactly `idle {interrupted}`. */
export function isInterrupted(state) {
  let s = state;
  if (typeof s === "string") { try { s = JSON.parse(s); } catch { return false; } }
  return !!s && typeof s === "object" && s.state === "idle" && !!s.stop && typeof s.stop === "object" && s.stop.kind === "interrupted";
}

/** The common window in which every source loaded: from the latest first load line to `end` (the first Stop, ms). The window must
 * last `minSeconds`, and all sources must load together all through it, with no idle stretch, expressed as a maximum spacing
 * (S36 r5 F5.5, r6 F6.3 and r7 F7.3): for each source and kind, no gap between consecutive stamps inside the window, nor
 * from its start to the first stamp, nor from the last stamp to its end, may exceed `times` the nominal spacing 1 / `rate` s
 * (three times: 60 ms for deltas at 50/s, 600 ms for entries at 5/s). A pure function with no window sampling; else a fault each
 * (the first long gap of each source and kind). */
export function loadWindow(by, end, rate, minSeconds, times = 3) {
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
    for (const [name, xs] of by) {
      for (const kind of ["deltas", "entries"]) {
        const most = (times * 1000) / rate[kind];
        const ats = xs.filter((x) => x.kind === (kind === "deltas" ? "delta" : "entry") && x.at >= from && x.at <= end).map((x) => x.at).sort((a, b) => a - b);
        const marks = [from, ...ats, end];
        for (let k = 1; k < marks.length; k++) {
          const gap = marks[k] - marks[k - 1];
          if (gap > most) {
            faults.push(`${name} wrote no ${kind} for ${gap.toFixed(1)} ms at ${((marks[k - 1] - from) / 1000).toFixed(3)} s of the common ${seconds.toFixed(1)} s, over ${most.toFixed(1)} ms (${times} times the spacing of ${rate[kind]}/s)`);
            break;
          }
        }
      }
    }
  }
  return { from, to: end, seconds, faults };
}
