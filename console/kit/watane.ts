// TECHNICAL 18.2, 9.4: the console's `Kernel` over `window.watane` (host 8.4 `stream`, `stream-end`; 8.5 `watane.js`).
import type { Kernel } from "./kernel";

/** The part of `window.watane` the console uses: `call` resolves to the `{ok|err}` value, `on` opens the page's event stream. */
export interface Watane {
  call(fn: string, args: object): Promise<unknown>;
  on(handler: (message: any) => void): unknown;
}

const HOLD_MS = 10_000;
const HOLD_MAX = 1000;

type Item = { seq: number; body: string };
interface Live { last: number; onMessage: (json: string) => void; onEnd: (reason: string) => void }
interface Held { since: number; items: Item[]; end?: string }

function fail(kind: string, message: string, extra?: object): Error {
  return Object.assign(new Error(message), { kind }, extra);
}
const caseOf = (o: unknown): [string, unknown] => {
  if (typeof o === "string") return [o, undefined];
  const k = o && typeof o === "object" ? Object.keys(o)[0] : undefined;
  return k === undefined ? ["unknown", undefined] : [k, (o as Record<string, unknown>)[k]];
};
const outcomeOf = (o: unknown): string => caseOf(o)[0];

export function wataneKernel(w: Watane): Kernel {
  const live = new Map<string, Live>();
  const ended = new Set<string>();
  const held = new Map<string, Held>();
  let pending = 0;

  const deliver = (id: string, s: Live, items: Item[]) => {
    for (const it of [...items].sort((a, b) => a.seq - b.seq)) {
      if (typeof it.seq !== "number" || it.seq <= s.last) continue;
      s.last = it.seq;
      s.onMessage(it.body);
    }
  };
  const finish = (id: string, s: Live, reason: string) => {
    if (live.get(id) !== s) return;
    live.delete(id); ended.add(id);
    s.onEnd(reason);
  };
  const hold = (id: string, f: (h: Held) => void) => {
    if (!pending || ended.has(id)) return;
    const now = Date.now();
    let h = held.get(id);
    if (h && now - h.since > HOLD_MS) { held.delete(id); h = undefined; }
    if (!h) { h = { since: now, items: [] }; held.set(id, h); }
    f(h);
  };

  w.on((m: any) => {
    if (!m || (m.type !== "stream" && m.type !== "stream-end") || typeof m.stream !== "string") return;
    const id: string = m.stream;
    const s = live.get(id);
    if (m.type === "stream") {
      if (!Array.isArray(m.items)) return;
      if (s) deliver(id, s, m.items);
      else hold(id, (h) => { if (h.end === undefined) for (const it of m.items) if (h.items.length < HOLD_MAX) h.items.push(it); });
    } else if (s) finish(id, s, outcomeOf(m.outcome));
    else hold(id, (h) => { h.end ??= outcomeOf(m.outcome); });
  });

  /** The `{ok}` payload of a call, or an `Error` whose `kind` is the case name. */
  async function call(fn: string, args: object): Promise<unknown> {
    let r: any;
    try { r = await w.call(fn, args); }
    catch (e) { throw fail("host", `${fn} failed: ${(e as Error)?.message ?? String(e)}`); }
    if (r && typeof r === "object" && "ok" in r) return r.ok;
    if (r && typeof r === "object" && "err" in r) {
      const [kind, payload] = caseOf(r.err);
      const msg = typeof payload === "string" ? payload : payload === undefined ? "" : JSON.stringify(payload);
      throw fail(kind, `${fn}: ${kind}${msg ? ` (${msg})` : ""}`, { payload, ...(kind === "refused" && Array.isArray(payload) ? { refusals: payload } : {}) });
    }
    throw fail("unreadable", `${fn} returned an unreadable value`);
  }

  return {
    async attach(session, after, onMessage, onEnd) {
      pending++;
      try {
        let id: unknown;
        try { id = await call("sessions.attach", { session, after, caps: "{}" }); }
        catch (e) {
          const n = (e as any)?.kind === "behind" ? (e as any).payload : undefined;
          if (typeof n === "number") return { behind: n };
          throw e;
        }
        if (typeof id !== "string") throw fail("unreadable", "sessions.attach: the stream id is not a string");
        const s: Live = { last: 0, onMessage, onEnd };
        live.set(id, s); ended.delete(id);
        const h = held.get(id);
        held.delete(id);
        if (h && Date.now() - h.since <= HOLD_MS) {
          deliver(id, s, h.items);
          if (h.end !== undefined) finish(id, s, h.end);
        }
        return { ok: true };
      } finally {
        if (--pending === 0) held.clear();
      }
    },
    async read(session, after, max) {
      const o = (await call("sessions.read", { session, after, before: null, max })) as { entries?: unknown };
      if (!o || !Array.isArray(o.entries)) throw fail("unreadable", "sessions.read: no entries");
      return o.entries.map((e) => (typeof e === "string" ? e : JSON.stringify(e)));
    },
    async say(session, key, input) {
      const r = await call("sessions.say", { session, key, input });
      return typeof r === "number" ? r : 0;
    },
    async answer(session, key, ask, decision) { await call("sessions.answer", { session, key, ask, decision }); },
    async interrupt(session, replacement) { await call("sessions.interrupt", { session, replacement: replacement ?? null }); },
    async takeBack(session, allow) { await call("sessions.take-back", { session, allow }); },
  };
}
