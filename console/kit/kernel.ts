// TECHNICAL 2.3, 9.4, 18.3: the kernel as a page sees it, and the attach / behind / read / reattach loop.
import { emptyView, type SessionView } from "./model";
import { parse } from "./wire";
import { safeReduce, settle, withUnknown } from "./reduce";

export interface Kernel {
  attach(session: string, after: number, onMessage: (json: string) => void, onEnd: (reason: string) => void):
    Promise<{ ok: true } | { behind: number }>;
  read(session: string, after: number, max: number): Promise<string[]>;
  say(session: string, key: string, input: string): Promise<number>;
  answer(session: string, key: string, ask: string, decision: string): Promise<void>;
  interrupt(session: string, replacement?: string): Promise<void>;
  takeBack(session: string, allow: string): Promise<void>;
}

/** TECHNICAL 2.3: the `decision` JSON of `answer`, `{allow | deny, for?: "session", reason?, input?}`: the outcome is a key of its own. */
export function decisionJson(d: { outcome: "allow" | "deny"; for?: "session"; reason?: string }): string {
  return JSON.stringify({ [d.outcome]: true, ...(d.for ? { for: d.for } : {}), ...(d.reason ? { reason: d.reason } : {}) });
}

const PAGE = 256;
const RETRY_MS = 1000;
const MAX_DELAY_MS = 30_000;
const MAX_TRIES = 5;
const STABLE_MS = 10_000;

/** Opens a session: attaches after the last index shown, pages when behind, reattaches when the stream drops. */
export function openSession(kernel: Kernel, session: string, onView: (v: SessionView) => void): { close(): void } {
  let view = emptyView;
  let closed = false;
  let gen = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let tries = 0;
  /** The last attach was the immediate reattach after a synced stream: the next one waits. */
  let quick = false;
  /** Fires when a synced stream has stayed open 10 s: only then does the try count reset. */
  let stable: ReturnType<typeof setTimeout> | undefined;
  const healthy = () => { tries = 0; quick = false; };

  const fetching = new Set<number>();
  /** Reads entry n back for its `~p<n>` row: once if it is read, at most 3 times (1 s apart) if the read fails. */
  async function fetchEntry(n: number) {
    let e: string | null = null;
    for (let tries = 0; tries < 3 && e === null && !closed; tries++) {
      if (tries) await new Promise((r) => setTimeout(r, RETRY_MS));
      if (closed) return;
      try { e = (await kernel.read(session, n - 1, 1))[0] ?? null; } catch { e = null; }
    }
    if (!closed) show(settle(view, n, e));
  }
  const show = (v: SessionView) => {
    view = v;
    if (closed) return;
    onView(v);
    // a message the view saw only in part: fetch its entry, which replaces the row
    for (const r of v.rows) {
      const n = r.key.startsWith("~p") ? Number(r.key.slice(2)) : NaN;
      if (Number.isNaN(n) || fetching.has(n)) continue;
      fetching.add(n);
      void fetchEntry(n).finally(() => fetching.delete(n));
    }
  };

  /** Pages in the entries before `through`, newest page first; a page that adds no index inside its range fails. */
  async function pageIn(g: number, through: number): Promise<string | undefined> {
    const base = view;
    const pages: string[][] = [];
    let hi = through;
    if (!Number.isInteger(through) || through > Number.MAX_SAFE_INTEGER) return "the kernel's `behind` is not an index";
    if (through <= base.lastIndex) return "the kernel's `behind` is not past the last index shown";
    while (hi > base.lastIndex && !closed && g === gen) {
      const lo = Math.max(base.lastIndex, hi - PAGE);
      if (lo === hi) return "the kernel's `behind` is too large to page";
      const page = await kernel.read(session, lo, hi - lo);
      if (closed || g !== gen) return undefined;
      if (!page.length) return "the kernel returned no entries for a page it is behind on";
      if (!page.some((e) => { const m = parse(e); return "i" in m && typeof m.i === "number" && m.i > lo && m.i <= hi; }))
        return "the kernel returned a page that adds no entry inside the range it asked for";
      pages.unshift(page);
      hi = lo;
      if (pages.length === 1) show(page.reduce(safeReduce, base)); // the newest page shows at once; the whole is reduced once below
    }
    if (closed || g !== gen) return undefined;
    show(pages.flat().reduce(safeReduce, base));
    if (view.lastIndex < through) throw new Error("the kernel's pages stop short of its own `behind`");
    return undefined;
  }

  /** The one reattach policy: a failed try waits 1 s, 2 s, 4 s ... up to 30 s; the 5th in a row stops the console. */
  function fail(g: number, cause: string, shown = false) {
    if (closed || g !== gen) return;
    if (++tries >= MAX_TRIES) { if (!shown) show(withUnknown(view, cause)); return; }
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => { if (!closed && g === gen) { quick = false; void connect(); } }, Math.min(RETRY_MS * 2 ** (tries - 1), MAX_DELAY_MS));
  }

  async function connect() {
    if (timer) clearTimeout(timer);
    timer = undefined;
    const g = ++gen;
    let reached = false;
    try {
      for (;;) {
        if (closed || g !== gen) return;
        view = { ...view, synced: false };
        const r = await kernel.attach(session, view.lastIndex,
          (json) => {
            if (g !== gen || closed) return;
            const was = view.synced;
            show(safeReduce(view, json));
            if (was) healthy();
            else if (view.synced) {
              reached = true;
              if (stable) clearTimeout(stable);
              stable = setTimeout(() => { if (!closed && g === gen) healthy(); }, STABLE_MS);
            }
          },
          (reason) => {
            if (stable) clearTimeout(stable);
            if (g !== gen || closed || reason === "replaced") return;
            if (reached && !quick) { quick = true; void connect(); }
            else fail(g, `the stream ended (${reason}) ${reached ? "again right after it synced" : "before it synced"}`);
          });
        if (closed || g !== gen || !("behind" in r)) return;
        const cause = await pageIn(g, r.behind);
        if (closed || g !== gen) return;
        if (cause) {
          show(withUnknown(view, cause));
          fail(g, cause, true);
          return;
        }
      }
    } catch (e) {
      fail(g, `the kernel failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  void connect();
  return { close() { closed = true; gen++; if (timer) clearTimeout(timer); if (stable) clearTimeout(stable); } };
}
