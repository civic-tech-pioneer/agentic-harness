// TECHNICAL 18.2, 18.3, 18.6: the rows inside assistant-ui's thread, fed by its External-store runtime; P14: only the
// rows near the viewport are in the DOM (the transcript is windowed), so a long session costs what a short one does.
import { memo, useLayoutEffect, useMemo, useReducer, useRef, type ReactNode } from "react";
import { AssistantRuntimeProvider, ThreadPrimitive, useExternalStoreRuntime, type ThreadMessageLike } from "@assistant-ui/react";
import type { Actions, Row, SessionView } from "./model";
import { FoldStore } from "./parts/disclosure";
import { RowView } from "./parts/rows";

function convert(row: Row): ThreadMessageLike {
  const text = row.kind === "user" || row.kind === "agent" || row.kind === "thought" ? row.text : "";
  return { id: row.key, role: row.kind === "user" ? "user" : "assistant", content: [{ type: "text", text }] };
}

/** The runtime keeps the newest rows only: it converts and re-indexes every message it holds on each update (about 80 ms
 *  for 5 000 rows), which a stream cannot pay once per frame. The transcript below draws from the view, not from the runtime. */
const RUNTIME_TAIL = 100;

export function ThreadProvider(props: { view: SessionView; actions: Actions; children: ReactNode }) {
  const { actions } = props;
  const { rows } = props.view;
  const isRunning = props.view.turn?.state === "running";
  const store = useMemo(() => ({
    messages: rows.length > RUNTIME_TAIL ? rows.slice(-RUNTIME_TAIL) : rows,
    convertMessage: convert,
    isRunning,
    onNew: async (m: { content: readonly { type: string; text?: string }[] }) => {
      const t = m.content.map((p) => (p.type === "text" ? p.text ?? "" : "")).join("");
      if (t) actions.say(t);
    },
  }), [rows, isRunning, actions]);
  const runtime = useExternalStoreRuntime<Row>(store as Parameters<typeof useExternalStoreRuntime<Row>>[0]);
  return <AssistantRuntimeProvider runtime={runtime}>{props.children}</AssistantRuntimeProvider>;
}

/** The height a row that was never measured stands for, the gap between rows (kit.css), the rows kept near the viewport, and the extra px around it. */
export const ROW_ESTIMATE = 76;
export const ROW_GAP = 8;
export const WINDOW_CAP = 100;
export const OVERSCAN = 800;
const NEAR_END = 40;
/** The event the page sends its scroller when the column changed size: the rows' heights are read again. */
export const MEASURE = "wa-measure";

/** A row whose ask or check is open must stay in the DOM: its typed reason and its ask bar live there. */
const holdsAsk = (r: Row) => r.kind === "tool" && !!r.ask && !r.ask.decision;

const Item = memo(function Item({ row, actions }: { row: Row; actions: Actions }) {
  return <RowView row={row} actions={actions} />;
});

export function Transcript(props: { view: SessionView; actions: Actions }) {
  const { rows } = props.view;
  const root = useRef<HTMLDivElement>(null);
  const scroller = useRef<HTMLElement | null>(null);
  const heights = useRef(new Map<string, number>());
  const version = useRef(0);
  const folds = useRef(new Map<string, boolean>()).current;
  const [, tick] = useReducer((x: number) => x + 1, 0);
  /** The transcript's top inside the scroller's content (its padding), and whether the reader is within 40 px of the end. */
  const origin = useRef(0);
  const nearEnd = useRef(true);
  /** The row the reader looks at and where it sits in the view: held still while heights above it change. */
  const anchor = useRef<{ key: string; rel: number } | null>(null);

  // offs[i] is the top of row i inside the transcript; a row never measured stands for the estimate.
  const offs = useMemo(() => {
    const o = new Float64Array(rows.length + 1);
    for (let i = 0; i < rows.length; i++) o[i + 1] = o[i]! + (heights.current.get(rows[i]!.key) ?? ROW_ESTIMATE) + ROW_GAP;
    return o;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rows, version.current]);
  const total = Math.max(0, offs[rows.length]! - ROW_GAP);
  const layout = useRef({ rows, offs });
  layout.current = { rows, offs };
  /** The row that holds offset y. */
  const find = (o: Float64Array, n: number, y: number) => {
    let lo = 0, hi = n - 1;
    while (lo < hi) { const m = (lo + hi + 1) >> 1; if (o[m]! <= y) lo = m; else hi = m - 1; }
    return lo;
  };

  const el = scroller.current;
  const view = el?.clientHeight ?? 0;
  let from: number, to: number; // the window: rows from..to
  if (!el || view <= 0 || rows.length === 0) { to = rows.length - 1; from = Math.max(0, to - WINDOW_CAP + 1); } // no layout known: the end
  else {
    const top = (nearEnd.current ? Math.max(0, origin.current + total - view) : el.scrollTop) - origin.current;
    from = find(offs, rows.length, Math.max(0, top - OVERSCAN));
    to = find(offs, rows.length, Math.max(0, top + view + OVERSCAN));
    if (to - from + 1 > WINDOW_CAP) { if (nearEnd.current) from = to - WINDOW_CAP + 1; else to = from + WINDOW_CAP - 1; }
  }
  const shown = new Set<number>();
  for (let i = from; i <= to; i++) shown.add(i);
  // kept mounted: the row with focus and every row with an open ask or check. Focus is read from the document, not from
  // focus events: when the window loses focus, the element keeps it (and gets it back), though a blur event fired.
  const active = root.current?.ownerDocument.activeElement;
  const focused = active && root.current!.contains(active) ? active.closest<HTMLElement>("[data-key]")?.dataset.key ?? null : null;
  rows.forEach((r, i) => { if (!shown.has(i) && (r.key === focused || holdsAsk(r))) shown.add(i); });
  const list = [...shown].sort((a, b) => a - b);

  const rowEls = () => root.current?.querySelectorAll<HTMLElement>(":scope > [data-key]") ?? [];
  /** Takes the heights of the rows in the DOM; true if one differs from what the layout holds. */
  const measure = () => {
    let dirty = false;
    rowEls().forEach((n) => {
      const h = n.offsetHeight, k = n.dataset.key!;
      if (h > 0 && heights.current.get(k) !== h) { heights.current.set(k, h); dirty = true; }
    });
    if (dirty) version.current++;
    return dirty;
  };
  /** The anchor is the row at the top of the view, with the distance of its top from the view's top. */
  const pick = () => {
    const e = scroller.current, { rows: rs, offs: o } = layout.current;
    if (!e || !rs.length) { anchor.current = null; return; }
    const i = find(o, rs.length, Math.max(0, e.scrollTop - origin.current));
    anchor.current = { key: rs[i]!.key, rel: o[i]! + origin.current - e.scrollTop };
  };

  useLayoutEffect(() => {
    const r = root.current;
    const e = r?.closest<HTMLElement>(".wa-scroll");
    if (!r || !e) return;
    scroller.current = e;
    origin.current = r.getBoundingClientRect().top - e.getBoundingClientRect().top + e.scrollTop;
    const onScroll = () => {
      nearEnd.current = e.scrollHeight - e.clientHeight - e.scrollTop <= NEAR_END;
      pick();
      tick();
    };
    e.addEventListener("scroll", onScroll, { passive: true });
    // the page's one resize observer (App) tells the scroller when the column changed size: a fold opened, a font came in
    const onResize = () => { if (measure()) tick(); };
    e.addEventListener(MEASURE, onResize);
    tick(); // the viewport is known now
    return () => { e.removeEventListener("scroll", onScroll); e.removeEventListener(MEASURE, onResize); scroller.current = null; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // after every commit: take the measured heights; hold the anchor row still when heights above it moved
  useLayoutEffect(() => {
    const e = scroller.current;
    if (!e) return;
    if (measure()) { tick(); return; }
    if (nearEnd.current) { // following the end: stay on it as measured heights change the content (App keeps it on as rows arrive)
      if (e.scrollHeight - e.clientHeight - e.scrollTop > 0.5) e.scrollTop = e.scrollHeight;
      return;
    }
    const a = anchor.current;
    const { rows: rs, offs: o } = layout.current;
    const i = a ? rs.findIndex((x) => x.key === a.key) : -1;
    if (!a || i < 0) { pick(); return; }
    const want = o[i]! + origin.current - a.rel;
    if (Math.abs(want - e.scrollTop) >= 1) { e.scrollTop = want; tick(); }
  });

  const out: ReactNode[] = [];
  let prev = -1;
  const gap = (key: string, upTo: number) => <div key={key} className="wa-gap" aria-hidden="true" style={{ height: offs[upTo]! - offs[prev + 1]! - ROW_GAP }} />;
  for (const i of list) {
    if (i > prev + 1) out.push(gap(`gap${prev}`, i));
    const r = rows[i]!;
    out.push(<div key={r.key} className="wa-row" data-kind={r.kind} data-key={r.key} data-index={i}><Item row={r} actions={props.actions} /></div>);
    prev = i;
  }
  if (prev < rows.length - 1) out.push(gap("gap-end", rows.length));

  return (
    <FoldStore.Provider value={folds}>
      <ThreadPrimitive.Root ref={root} className="wa-transcript" data-total={total}>
        {out}
      </ThreadPrimitive.Root>
    </FoldStore.Provider>
  );
}
