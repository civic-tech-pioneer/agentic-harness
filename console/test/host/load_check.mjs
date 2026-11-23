// S36 r2 F2.3: the stage B runner's acceptance checks (test/host/load_b.mjs), kept pure so the component run proves them
// (test/host/load_check.test.ts).

/** A turn state (object or the kernel's turn JSON) that is exactly `idle {interrupted}`. */
export function isInterrupted(state) {
  let s = state;
  if (typeof s === "string") { try { s = JSON.parse(s); } catch { return false; } }
  return !!s && typeof s === "object" && s.state === "idle" && !!s.stop && typeof s.stop === "object" && s.stop.kind === "interrupted";
}

/** The common window in which every source loaded: from the latest first load line to `end` (the first Stop, ms). Each source must
 * write at least `share` of `rate` deltas and entries per second within it, and the window must last `minSeconds`; else a fault each. */
export function loadWindow(by, end, rate, minSeconds, share = 0.8) {
  const faults = [];
  let from = -Infinity;
  for (const [name, xs] of by) {
    const first = xs.find((x) => x.at < end);
    if (!first) { faults.push(`${name} wrote no load line before Stop`); continue; }
    from = Math.max(from, first.at);
  }
  const seconds = (end - from) / 1000;
  if (!(seconds >= minSeconds)) faults.push(`the common load window is ${Number.isFinite(seconds) ? seconds.toFixed(1) : 0} s, under ${minSeconds} s`);
  else for (const [name, xs] of by) {
    const inside = xs.filter((x) => x.at >= from && x.at < end);
    for (const kind of ["deltas", "entries"]) {
      const got = inside.filter((x) => x.kind === (kind === "deltas" ? "delta" : "entry")).length;
      const want = Math.floor(rate[kind] * seconds * share);
      if (got < want) faults.push(`${name} wrote ${got} ${kind} in the common ${seconds.toFixed(1)} s, under ${want} (${share * 100}% of ${rate[kind]}/s)`);
    }
  }
  return { from, to: end, seconds, faults };
}
