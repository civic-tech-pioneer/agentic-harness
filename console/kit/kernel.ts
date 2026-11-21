// TECHNICAL 2.3, 9.4, 18.3: the kernel as a page sees it, and the attach / behind / read / reattach loop.
import { emptyView, type SessionView } from "./model";
import { parse } from "./wire";
import { safeReduce, settle, withUnknown } from "./reduce";
import { joiner, partOf } from "./join";

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
const posInt = (x: unknown): x is number => typeof x === "number" && Number.isInteger(x) && x > 0;

/** The entries of pages, in index order and each index once, joined into whole entries (9.4). */
function assemble(pages: string[][]): string[] {
  const seen = new Map<number, string>();
  const rest: string[] = [];
  for (const e of pages.flat()) {
    const m = parse(e);
    if ("i" in m && typeof m.i === "number") seen.set(m.i, e); else rest.push(e);
  }
  const j = joiner();
  return [...[...seen.entries()].sort((a, b) => a[0] - b[0]).map((x) => x[1]), ...rest].flatMap((e) => j.push(e));
}
const RETRY_MS = 1000;
const MAX_DELAY_MS = 30_000;
const MAX_TRIES = 5;
const STABLE_MS = 10_000;

const BURST = 8;
const REFILL_PER_S = 4;

/** The one rate governor: a token bucket of 8 calls refilled at 4 per second; a call without a token waits for one. */
export function governor() {
  let tokens = BURST;
  let last = Date.now();
  const queue: Array<() => void> = [];
  let timer: ReturnType<typeof setTimeout> | undefined;
  const drain = () => {
    timer = undefined;
    const now = Date.now();
    tokens = Math.min(BURST, tokens + ((now - last) / 1000) * REFILL_PER_S);
    last = now;
    while (queue.length && tokens >= 1) { tokens--; queue.shift()!(); }
    if (queue.length) timer = setTimeout(drain, Math.ceil(((1 - tokens) / REFILL_PER_S) * 1000));
  };
  return {
    take(): Promise<void> { return new Promise((r) => { queue.push(r); if (!timer) drain(); }); },
    stop() { queue.length = 0; if (timer) clearTimeout(timer); timer = undefined; },
  };
}

/** Opens a session: attaches after the last index shown, pages when behind, reattaches when the stream drops. */
export function openSession(rawKernel: Kernel, session: string, onView: (v: SessionView) => void): { close(): void } {
  const gov = governor();
  const kernel: Kernel = {
    ...rawKernel,
    attach: async (...a) => { await gov.take(); return rawKernel.attach(...a); },
    read: async (...a) => { await gov.take(); return rawKernel.read(...a); },
  };
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
      try {
        e = (await kernel.read(session, n - 1, 1))[0] ?? null;
        // an entry stored as parts: read the whole group (9.4) and join it
        const p = e === null ? null : partOf(e);
        if (p && posInt(p.n) && p.n > 1 && p.n <= PAGE) {
          const j = joiner();
          let out: string[] = [];
          for (const x of await kernel.read(session, n - p.n, p.n)) out = out.concat(j.push(x));
          e = out[out.length - 1] ?? e;
        }
      } catch { e = null; }
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
    const chunk: string[][] = [];
    let hi = through;
    if (!Number.isInteger(through) || through > Number.MAX_SAFE_INTEGER) return "the kernel's `behind` is not an index";
    if (through <= base.lastIndex) return "the kernel's `behind` is not past the last index shown";
    while (hi > base.lastIndex && !closed && g === gen) {
      const lo = Math.max(base.lastIndex, hi - PAGE);
      if (lo === hi) return "the kernel's `behind` is too large to page";
      // a short page (the kernel's size cap) is read on from its last index until lo .. hi is covered
      let c = lo;
      while (c < hi && !closed && g === gen) {
        const page = await kernel.read(session, c, hi - c);
        if (closed || g !== gen) return undefined;
        if (!page.length) return "the kernel returned no entries for a page it is behind on";
        let top = c;
        // the range began inside a group: read the group's start again (9.4), once per range
        const first = c === lo ? partOf(page[0]) : null;
        if (first && posInt(first.n) && first.n > 1 && first.n <= PAGE) {
          const back = await kernel.read(session, first.i - first.n, first.n);
          if (closed || g !== gen) return undefined;
          chunk.push(back);
        }
        for (const e of page) { const m = parse(e); if ("i" in m && typeof m.i === "number" && m.i > top && m.i <= hi) top = m.i; }
        if (top === c) return "the kernel returned a page that adds no entry inside the range it asked for";
        chunk.push(page);
        c = top;
      }
      if (closed || g !== gen) return undefined;
      pages.unshift(chunk.splice(0).flat());
      hi = lo;
      if (pages.length === 1) show(assemble(pages).reduce(safeReduce, base)); // the newest range shows at once; the whole is reduced once below
    }
    if (closed || g !== gen) return undefined;
    show(assemble(pages).reduce(safeReduce, base));
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
        const join = joiner(); // replay and the live stream of this attach share it
        const r = await kernel.attach(session, view.lastIndex,
          (json) => {
            if (g !== gen || closed) return;
            const was = view.synced;
            for (const m of join.push(json)) show(safeReduce(view, m));
            if (was) tries = 0; // a live item resets the try count, not the one quick reattach
            else if (view.synced) {
              reached = true;
              if (stable) clearTimeout(stable);
              stable = setTimeout(() => { if (!closed && g === gen) healthy(); }, STABLE_MS);
            }
          },
          (reason) => {
            if (g !== gen || closed) return; // an old connection's end leaves the new one's timer alone
            if (stable) clearTimeout(stable);
            if (reason === "replaced") return;
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
  return { close() { closed = true; gen++; gov.stop(); if (timer) clearTimeout(timer); if (stable) clearTimeout(stable); } };
}
