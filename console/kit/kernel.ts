// TECHNICAL 2.3, 9.4, 18.3: the kernel as a page sees it, and the attach / behind / read / reattach loop.
import { emptyView, type SessionView } from "./model";
import { reduce } from "./reduce";

export interface Kernel {
  attach(session: string, after: number, onMessage: (json: string) => void, onEnd: (reason: string) => void):
    Promise<{ ok: true } | { behind: number }>;
  read(session: string, after: number, max: number): Promise<string[]>;
  say(session: string, key: string, input: string): Promise<number>;
  answer(session: string, key: string, ask: string, decision: string): Promise<void>;
  interrupt(session: string, replacement?: string): Promise<void>;
  takeBack(session: string, allow: string): Promise<void>;
}

const PAGE = 256;
const RETRY_MS = 1000;

/** Opens a session: attaches after the last index shown, pages when behind, reattaches when the stream drops. */
export function openSession(kernel: Kernel, session: string, onView: (v: SessionView) => void): { close(): void } {
  let view = emptyView;
  let closed = false;
  let gen = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const show = (v: SessionView) => { view = v; if (!closed) onView(v); };

  async function pageIn(through: number) {
    const base = view;
    const pages: string[][] = [];
    let hi = through;
    while (hi > base.lastIndex && !closed) {
      const lo = Math.max(base.lastIndex, hi - PAGE);
      const page = await kernel.read(session, lo, hi - lo);
      if (!page.length) break;
      pages.unshift(page);
      hi = lo;
      show(pages.flat().reduce(reduce, base));
    }
    if (view.lastIndex < through) throw new Error("the kernel's pages stop short of its own `behind`");
  }

  async function connect() {
    timer = undefined;
    const g = ++gen;
    try {
      for (;;) {
        if (closed) return;
        view = { ...view, synced: false };
        const r = await kernel.attach(session, view.lastIndex,
          (json) => { if (g === gen && !closed) show(reduce(view, json)); },
          (reason) => { if (g === gen && !closed && reason !== "replaced") void connect(); });
        if (!("behind" in r)) return;
        await pageIn(r.behind);
      }
    } catch {
      if (!closed && g === gen) timer = setTimeout(() => void connect(), RETRY_MS);
    }
  }

  void connect();
  return { close() { closed = true; gen++; if (timer) clearTimeout(timer); } };
}
