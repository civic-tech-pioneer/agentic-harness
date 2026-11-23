// S36 r2 F2.3: the stage B runner's acceptance checks (test/host/load_b.mjs), kept pure so the component run proves them
// (test/host/load_check.test.ts).

/** A turn state (object or the kernel's turn JSON) that is exactly `idle {interrupted}`. */
export function isInterrupted(state) {
  let s = state;
  if (typeof s === "string") { try { s = JSON.parse(s); } catch { return false; } }
  return !!s && typeof s === "object" && s.state === "idle" && !!s.stop && typeof s.stop === "object" && s.stop.kind === "interrupted";
}

/** The common window in which every source loaded: from the latest first load line to `end` (the first Stop, ms). The window must
 * last `minSeconds`, and each source must write at least `share` of `rate` deltas and entries per second in every shared subwindow
 * of about `step` seconds that tiles it (S36 r3 F3.5: a total over the window lets one early line and a short burst pass); else a
 * fault each (the first short subwindow of each source and kind). */
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
    const parts = Math.max(1, Math.floor(seconds / step)), len = (end - from) / parts; // equal subwindows that cover it exactly
    for (const [name, xs] of by) for (const kind of ["deltas", "entries"]) {
      const want = Math.floor(rate[kind] * (len / 1000) * share);
      for (let k = 0; k < parts; k++) {
        const a = from + k * len, z = k === parts - 1 ? end : a + len;
        const got = xs.filter((x) => x.at >= a && x.at < z && x.kind === (kind === "deltas" ? "delta" : "entry")).length;
        if (got < want) {
          faults.push(`${name} wrote ${got} ${kind} in ${((a - from) / 1000).toFixed(1)}-${((z - from) / 1000).toFixed(1)} s of the common ${seconds.toFixed(1)} s, under ${want} (${share * 100}% of ${rate[kind]}/s)`);
          break;
        }
      }
    }
  }
  return { from, to: end, seconds, faults };
}
