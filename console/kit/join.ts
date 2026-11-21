// TECHNICAL 9.4: joins the `part` entries of an entry over 256 KiB back into the one entry, before the reducer sees them.
import { parse } from "./wire";

type Rec = Record<string, unknown>;
const isRec = (x: unknown): x is Rec => typeof x === "object" && x !== null && !Array.isArray(x);

/** The memory bound of this console: a group is at most 4096 parts and 64 MiB of text. */
const MAX_M = 4096;
const MAX_BYTES = 64 * 1024 * 1024;

export interface PartInfo { i: number; n: unknown; m: unknown; body: Rec }

/** The `part` entry of a stream message (a bare entry, or an `entry` item), or null for any other message. */
export function partOf(message: string): PartInfo | null {
  let o: unknown;
  try { o = JSON.parse(message); } catch { return null; }
  if (!isRec(o)) return null;
  const w = isRec(o._meta) && isRec(o._meta.watane) ? o._meta.watane : undefined;
  const e = w ? (w.type === "entry" && isRec(w.entry) ? w.entry : undefined) : o;
  if (!e || e.kind !== "part" || typeof e.i !== "number" || !Number.isInteger(e.i)) return null;
  const body = isRec(e.body) ? e.body : {};
  return { i: e.i, n: body.n, m: body.m, body };
}

const posInt = (x: unknown): x is number => typeof x === "number" && Number.isInteger(x) && x > 0;
const size = (s: string) => new TextEncoder().encode(s).length;

/** Takes stream messages in order; returns what to pass on. A group that cannot be joined becomes one `unknown` entry. */
export function joiner(): { push(message: string): string[] } {
  let open: { m: number; n: number; i: number; texts: string[]; bodies: Rec[]; bytes: number } | null = null;
  /** The rest of a failed group, absorbed so each failure shows one row. */
  let skip: { m: number; n: number; i: number } | null = null;

  const unknown = (i: number, raw: unknown) => JSON.stringify({ v: 1, i, kind: "unknown", body: { raw } });

  function fail(i: number, extra?: Rec, summary?: boolean): string {
    const g = open;
    open = null;
    const bodies = g ? g.bodies : [];
    if (extra) bodies.push(extra);
    const raw = summary ? bodies.map((b) => ({ n: b.n, m: b.m })) : bodies;
    return unknown(i, raw);
  }

  return {
    push(message) {
      const p = partOf(message);
      if (!p) {
        if (!open) { skip = null; return [message]; }
        // only an entry fails the open group; an item with no index (synced, turn, queue, usage) passes on
        const e = parse(message);
        if (!("i" in e && typeof e.i === "number")) return [message];
        const g = open;
        return [fail(g.i), message];
      }
      const { i, n, m, body } = p;
      if (skip) {
        const s = skip;
        if (n === s.n + 1 && m === s.m && i === s.i + 1) {
          skip = n === s.m ? null : { m: s.m, n, i };
          return [];
        }
        skip = null;
      }
      const stop = (out: string): string[] => {
        // the rest of this group (parts n+1 .. m) is absorbed
        skip = posInt(n) && posInt(m) && n < m && m <= MAX_M ? { m, n, i } : null;
        return [out];
      };
      const text = body.text;
      if (!posInt(n) || !posInt(m) || n > m || typeof text !== "string") return stop(fail(i, body));
      if (m > MAX_M) return stop(fail(i, { n, m }, true));
      if (open) {
        if (n !== open.n + 1 || m !== open.m || i !== open.i + 1) return stop(fail(i, body));
      } else if (n !== 1) {
        return stop(fail(i, body));
      } else {
        open = { m, n: 0, i: i - 1, texts: [], bodies: [], bytes: 0 };
      }
      const g = open;
      g.n = n; g.i = i;
      g.bytes += size(text);
      if (g.bytes > MAX_BYTES) return stop(fail(i, { n, m }, true));
      g.texts.push(text);
      g.bodies.push(body);
      if (n < m) return [];
      open = null;
      let joined: unknown;
      const joinedText = g.texts.join("");
      try { joined = JSON.parse(joinedText); } catch { joined = undefined; }
      if (!isRec(joined) || joined.i !== i) { open = g; return [fail(i)]; }
      return [joinedText];
    },
  };
}
