import { isInterrupted } from "../load_check.mjs";

export function p95(samples) {
  if (!samples.length) throw new Error("no measurements");
  return [...samples].sort((a, b) => a - b)[Math.ceil(samples.length * 0.95) - 1];
}

// Clip completed commit intervals to the same wall-clock window as the items.
// Take their union so that a shared endpoint or overlapping trace never counts twice.
export function writerBusy(intervals, from, to) {
  if (!intervals.length || !(to > from)) throw new Error("no writer telemetry or invalid load window");
  let edge = from;
  let busy = 0;
  for (const [start, end] of [...intervals].sort((a, b) => a[0] - b[0])) {
    const right = Math.min(end, to);
    const left = Math.max(start, edge);
    if (right > left) {
      busy += right - left;
      edge = right;
    }
  }
  return 100 * busy / (to - from);
}

// Keep the one-second grid in sync with the paced source in watane.patch.
const STEP_INTERVAL_MS = 1000;
const STEP_SLACK_MS = 200;
const MAX_STEP_GAP_MS = STEP_INTERVAL_MS + STEP_SLACK_MS;

// One step a second through the window. The window starts no earlier than the session's second
// step: its first request may wait for the toolset to commit (the ~1 s describe retry, TECHNICAL 9.5),
// so a late first step is not a missed beat (S131 P2.1); a late later step still is.
export function openaiCadence(stamps, from, to) {
  const all = [...stamps].sort((a, b) => a - b);
  if (all.length < 2) return false;
  const begin = Math.max(from, all[1]);
  const inside = all.filter((at) => at >= begin && at < to);
  const gaps = inside.slice(1).map((at, i) => at - inside[i]);
  return inside.length >= 5 && inside[0] - begin <= MAX_STEP_GAP_MS && to - inside.at(-1) <= MAX_STEP_GAP_MS
    && gaps.every((gap) => gap >= STEP_INTERVAL_MS - STEP_SLACK_MS && gap <= MAX_STEP_GAP_MS);
}

// The same consecutive marker pair at each end locates a spacing gap.
export function claudeGaps(stamps, other, from, to) {
  const bySourceAndKind = new Map();
  for (const [marker, at] of stamps) {
    if (at < from || at >= to) continue;
    const group = marker.slice(0, marker.lastIndexOf("-"));
    if (!bySourceAndKind.has(group)) bySourceAndKind.set(group, []);
    bySourceAndKind.get(group).push({ marker, at });
  }
  const maxima = {};
  for (const [group, items] of bySourceAndKind) {
    const kind = group.slice(group.lastIndexOf("-") + 1);
    items.sort((a, b) => a.at - b.at);
    for (let i = 1; i < items.length; i++) {
      const before = items[i - 1], after = items[i];
      const ms = after.at - before.at;
      if (ms <= (maxima[kind]?.ms ?? 0)) continue;
      const otherMs = other.get(after.marker) - other.get(before.marker);
      maxima[kind] = { ms, otherMs: Number.isFinite(otherMs) ? otherMs : null, before: before.marker, after: after.marker };
    }
  }
  return maxima;
}

export const ITEM_DELAY_MS = 100;
// Only the final paced step can be cut. Allow one grid interval plus the same
// scheduling slack as the cadence check between its first write and durable Stop.
const INTERRUPT_WINDOW_MS = MAX_STEP_GAP_MS;
const sourceMarker = (text) => /^(oa-s\d+-\d+|cl-[a-f\d]+-(?:delta|entry)-\d+)(?: |$)/.exec(text)?.[1];

// TLS writes, request sink receipts and stored entries cover successive points in the path.
// Stop may cut the final in-flight batch before its missing markers reach the sink. An earlier
// gap still fails, as does a missing marker backed by a sink receipt or durable admission.
export function openaiPageCoverage(wire, seen, stored, from, to, stopAt, receipts = []) {
  const faults = [];
  const interrupted = stored.findLast(({ kind }) => kind === "turn");
  const confirmed = isInterrupted(interrupted?.body);
  if (!confirmed) faults.push("no stored interrupted turn");
  // OpenAI's pending calls are live events. The kernel writes a terminal tool row
  // when a job completes or fails, and writes aborted outcomes before the interrupted
  // turn for any jobs still open. Presence of any row therefore proves admission.
  const acceptedTools = new Set(stored
    .filter(({ kind, i, body }) => kind === "tool" && (!confirmed || i < interrupted.i)
      && (body.status === "completed" || body.status === "failed"))
    .map(({ body }) => body.toolCallId));
  const acceptedMessages = new Set(stored
    .filter(({ kind, i, body }) => kind === "message" && body.role === "agent" && confirmed && i < interrupted.i)
    .map(({ body }) => sourceMarker(body.content?.[0]?.text ?? "")));
  const steps = wire.filter(({ marker, step }) => marker === step).sort((a, b) => a.at - b.at);
  const tail = confirmed ? steps.findLast(({ at }) => at <= interrupted.at) : undefined;
  // A cancelled stream can still answer taken for a batch it discarded. The
  // stored turn uses whole milliseconds, so count receipts in its millisecond.
  const taken = new Set(receipts
    .filter(({ at }) => !confirmed || Math.floor(at) <= interrupted.at)
    .map(({ marker }) => marker));
  let cut = [];
  for (const start of steps.filter(({ at }) => at >= from && at < to)) {
    const writes = wire.filter(({ step }) => step === start.marker);
    const calls = writes.filter(({ marker }) => marker.startsWith("call_"));
    const ids = new Set(calls.map(({ marker }) => marker));
    const prefix = /^call_\d+_/.exec(calls[0]?.marker)?.[0];
    const validCalls = ids.size === calls.length
      && [...ids].every((id) => prefix && id.startsWith(prefix) && /^[0-3]$/.test(id.slice(prefix.length)));
    const completions = writes.filter(({ marker }) => marker === `${start.marker}/completed`);
    const missing = [start, ...calls].filter(({ marker }) => !seen.has(marker));
    const complete = completions.length === 1 && calls.length === 4;
    // Bound the last wire write and any missing marker by the page item-delay budget
    // around the actual click. A partial batch must also overlap the durable interrupt.
    const overlaps = completions.length === 0 && writes.some(({ at }) => at >= interrupted?.at);
    const lastWrite = Math.max(...writes.map(({ at }) => at));
    const nearClick = Number.isFinite(stopAt) && Math.abs(lastWrite - stopAt) <= ITEM_DELAY_MS
      && missing.every(({ at }) => at >= stopAt - ITEM_DELAY_MS);
    const nearInterrupt = interrupted?.at - start.at <= INTERRUPT_WINDOW_MS;
    if (start === tail && nearClick && validCalls && ((complete && nearInterrupt) || overlaps)
        && !missing.some(({ marker }) => taken.has(marker))
        && !acceptedMessages.has(start.marker) && !calls.some(({ marker }) => acceptedTools.has(marker))) {
      if (missing.length) cut = writes;
      continue;
    }
    if (!validCalls || calls.length !== 4) {
      faults.push(`${start.marker}: source wrote ${[...ids].join(", ") || "no calls"} instead of four distinct calls for this step`);
    }
    if (completions.length !== 1) faults.push(`${start.marker}: expected one completion write, got ${completions.length}`);
    for (const { marker } of missing) faults.push(`missing page marker ${marker}`);
  }
  const hasStoredMarker = steps.some(({ at, marker }) => at >= from && at < to && acceptedMessages.has(marker));
  return { faults, interrupted, cut, hasStoredMarker };
}

export function clickStop() {
  const button = document.querySelector(".wa-stop");
  if (!button) return null;
  const at = Date.now();
  button.click();
  return at;
}

export function pageProbe() {
  window.loadItems = [];
  let bridge;
  Object.defineProperty(window, "watane", {
    configurable: true,
    get: () => bridge,
    set(real) {
      bridge = { ...real,
        async call(fn, args) {
          const result = await real.call(fn, args);
          if (fn === "sessions.attach" && result && "ok" in result) {
            window.loadAttach = { id: args.session, stream: result.ok };
          }
          return result;
        },
        on(handler) {
          return real.on((event) => {
            const at = Date.now();
            for (const item of event.type === "stream" ? event.items : []) {
              window.loadItems.push({ at, stream: event.stream, body: item.body });
            }
            handler(event);
          });
        },
      };
    },
  });
}

export function pageItem({ at, body }) {
  const o = JSON.parse(body);
  const w = o._meta?.watane;
  const text = o.content?.text ?? w?.entry?.body?.content?.[0]?.text ?? "";
  const marker = sourceMarker(text);
  return {
    at,
    // OpenAI's final text also arrives as an indexed update or entry. It is
    // evidence of admission in the transcript, never a substitute for the delta.
    marker: marker?.startsWith("oa-") && (w?.i !== undefined || w?.entry) ? undefined
      : marker ?? (o.sessionUpdate === "tool_call" && o.status === "pending" ? o.toolCallId : undefined),
    turn: w?.type === "turn" ? w.state : w?.entry?.kind === "turn" ? w.entry.body : undefined,
  };
}
