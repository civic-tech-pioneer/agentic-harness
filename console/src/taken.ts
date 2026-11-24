// TECHNICAL 18.6 "Queue" (S36 r4, r5, the deputy's rules): what the queue's actions took, per session, outside the dock's
// state, so a reply that ends after she switched sessions still writes to it. Memory is the list's truth while the page lives;
// `sessionStorage` is only a mirror, written after memory (a failed write leaves memory as it is), read once per session on the
// page's first look, so the list is back after a reload as the kept draft is.
import { useSyncExternalStore } from "react";

/** A taken input: its text, what was being tried with it, the error of its last refusal, and for Remove's item the absolute
 * time (ms) its ordinary Undo ends. `out`: a call is in flight. `held`: the composer holds its text and is its one send path
 * (S36 r6 F6.1): its row shows no Undo or retry. */
export type Taken = { id: string; text: string; tried: "edit" | "stop" | "undo"; error?: string; until?: number; out?: boolean; held?: boolean };
type Kept = Omit<Taken, "out" | "held">;

export const UNDO_MS = 6000;
const lists = new Map<string, Kept[]>(); // the truth while the page lives
const out = new Set<string>(); // calls in flight are this page's, never kept
const listeners = new Set<() => void>();
let ids = 0;
const keyOf = (session: string) => `wa-taken:${session}`;
const holdKey = (session: string) => `wa-draft-of:${session}`; // the kept draft's item, beside the draft (`wa-draft:`)
const holds = new Map<string, string | undefined>(); // per session, the item the composer holds; mirrored as the list is
const loaded = (session: string): Kept[] => {
  try {
    const xs: unknown = JSON.parse(sessionStorage.getItem(keyOf(session)) || "[]");
    return Array.isArray(xs) ? xs.filter((x): x is Kept => !!x && typeof x.id === "string" && typeof x.text === "string") : [];
  } catch { return []; }
};
const kept = (session: string): Kept[] => {
  let xs = lists.get(session);
  if (!xs) { xs = loaded(session); lists.set(session, xs); }
  return xs;
};
const write = (session: string, xs: Kept[]) => {
  lists.set(session, xs);
  try { if (xs.length) sessionStorage.setItem(keyOf(session), JSON.stringify(xs)); else sessionStorage.removeItem(keyOf(session)); }
  catch { /* the mirror failed: memory stands */ }
  listeners.forEach((f) => f());
};

const has = (session: string, id: string) => kept(session).some((t) => t.id === id);

/** The item whose text the session's composer holds, if it is still listed (S36 r6 F6.1). */
export function heldOf(session: string): string | undefined {
  if (!holds.has(session)) { let id: string | undefined; try { id = sessionStorage.getItem(holdKey(session)) ?? undefined; } catch { /* none */ } holds.set(session, id); }
  const id = holds.get(session);
  return id !== undefined && has(session, id) ? id : undefined;
}
/** The composer holds item `id`'s text now (none: it holds none of an item's): memory first, then the mirror. */
export function hold(session: string, id?: string) {
  heldOf(session); // the mirror is read once
  if (holds.get(session) === id) return;
  holds.set(session, id);
  try { if (id === undefined) sessionStorage.removeItem(holdKey(session)); else sessionStorage.setItem(holdKey(session), id); }
  catch { /* the mirror failed: memory stands */ }
  listeners.forEach((f) => f());
}

const snaps = new Map<string, { xs: Kept[]; out: string; held?: string; shown: Taken[] }>();
/** The session's list (the same array while it is unchanged, for React). */
export function takenOf(session: string): Taken[] {
  const xs = kept(session), flying = [...out].join(" "), held = heldOf(session), s = snaps.get(session);
  if (s && s.xs === xs && s.out === flying && s.held === held) return s.shown;
  const shown = xs.map((x) => (out.has(x.id) || x.id === held ? { ...x, ...(out.has(x.id) ? { out: true } : {}), ...(x.id === held ? { held: true } : {}) } : x));
  snaps.set(session, { xs, out: flying, held, shown });
  return shown;
}

/** An input enters when its `unqueue` succeeded. Remove's (`tried: "undo"`) carries the absolute end of its ordinary Undo. No
 * timer touches the list: an item leaves only by `drop` (its `say` or `interrupt` accepted, or her Discard). */
export function take(session: string, text: string, tried: Taken["tried"], flying = false): string {
  const id = `${Date.now().toString(36)}.${++ids}`;
  if (flying) out.add(id);
  write(session, [...kept(session), { id, text, tried, ...(tried === "undo" ? { until: Date.now() + UNDO_MS } : {}) }]);
  return id;
}
/** Whether item `x` shows its Undo at time `now`: all but a Remove's that is past its deadline, never refused and not in flight. */
export const undoShown = (x: Taken, now = Date.now()) => x.tried !== "undo" || x.error !== undefined || !!x.out || x.until === undefined || now < x.until;
/** Marks a call of item `id` in flight (one at a time, keyed by its id, whichever control sends it: false when one is). */
export function send(session: string, id: string): boolean {
  if (out.has(id) || !has(session, id)) return false;
  out.add(id); listeners.forEach((f) => f());
  return true;
}
/** Its call was refused: it stays, with the error's words. */
export function refused(session: string, id: string, error?: string) {
  out.delete(id);
  write(session, kept(session).map((t) => (t.id === id ? { ...t, error } : t)));
}
/** Its call ended with nothing to record (a refused send of the composer): the item is free again. */
export function release(id: string) { if (out.delete(id)) listeners.forEach((f) => f()); }
/** Leaves the list: its `say` or `interrupt` was accepted, or she discarded it. The composer's link to it goes with it. */
export function drop(session: string, id: string) {
  out.delete(id);
  if (holds.get(session) === id) hold(session, undefined);
  if (has(session, id)) write(session, kept(session).filter((t) => t.id !== id));
  else listeners.forEach((f) => f());
}

const subscribe = (f: () => void) => { listeners.add(f); return () => { listeners.delete(f); }; };
export const useTaken = (session: string) => useSyncExternalStore(subscribe, () => takenOf(session));

/** Tests: the page's memory goes, as on a reload; `sessionStorage` stays. */
export function forgetTaken() { lists.clear(); snaps.clear(); out.clear(); holds.clear(); }
