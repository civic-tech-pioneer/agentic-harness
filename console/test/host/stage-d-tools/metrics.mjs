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

// One step a second through the window. The window starts no earlier than the session's second
// step: its first request may wait for the toolset to commit (the ~1 s describe retry, TECHNICAL 9.5),
// so a late first step is not a missed beat (S131 P2.1); a late later step still is.
export function openaiCadence(stamps, from, to) {
  const all = [...stamps].sort((a, b) => a - b);
  if (all.length < 2) return false;
  const begin = Math.max(from, all[1]);
  const inside = all.filter((at) => at >= begin && at < to);
  const gaps = inside.slice(1).map((at, i) => at - inside[i]);
  return inside.length >= 5 && inside[0] - begin <= 1200 && to - inside.at(-1) <= 1200
    && gaps.every((gap) => gap >= 800 && gap <= 1200);
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
            const at = performance.timeOrigin + performance.now();
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
  const marker = /^(oa-s\d+-\d+|cl-[a-f\d]+-(?:delta|entry)-\d+)(?: |$)/.exec(text)?.[1];
  return {
    at,
    marker: marker ?? (o.sessionUpdate === "tool_call" && o.status === "pending" ? o.toolCallId : undefined),
    turn: w?.type === "turn" ? w.state : w?.entry?.kind === "turn" ? w.entry.body : undefined,
  };
}
