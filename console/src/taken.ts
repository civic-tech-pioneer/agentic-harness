// TECHNICAL 18.6 "Queue" (S36 r4, the deputy's rule): what the queue's actions took, per session, outside the dock's state, so a
// reply that ends after she switched sessions still writes to it; and each session's capacity epoch. The list is kept in
// `sessionStorage` (back after a reload, as the kept draft is), in memory when the page has none.
import { useSyncExternalStore } from "react";

/** A taken input: its text, what was being tried with it, and the error of its last refusal. `out`: a call is in flight. */
export type Taken = { id: string; text: string; tried: "edit" | "stop" | "undo"; error?: string; out?: boolean };
type Kept = Omit<Taken, "out">;

const UNDO_MS = 6000;
const memory = new Map<string, string>(); // a page with no `sessionStorage`
const out = new Set<string>(); // calls in flight are this page's, never kept
const listeners = new Set<() => void>();
const epochs = new Map<string, number>();
let ids = 0;
const keyOf = (session: string) => `wa-taken:${session}`;
const read = (session: string): string => { try { return sessionStorage.getItem(keyOf(session)) ?? ""; } catch { return memory.get(session) ?? ""; } };
const write = (session: string, xs: Kept[]) => {
  const raw = xs.length ? JSON.stringify(xs) : "";
  try { if (raw) sessionStorage.setItem(keyOf(session), raw); else sessionStorage.removeItem(keyOf(session)); }
  catch { if (raw) memory.set(session, raw); else memory.delete(session); }
  listeners.forEach((f) => f());
};
const kept = (session: string): Kept[] => { try { const xs: unknown = JSON.parse(read(session) || "[]"); return Array.isArray(xs) ? xs as Kept[] : []; } catch { return []; } };

const snaps = new Map<string, { raw: string; out: string; xs: Taken[] }>();
/** The session's list (the same array while it is unchanged, for React). */
export function takenOf(session: string): Taken[] {
  const raw = read(session), flying = [...out].join(" "), s = snaps.get(session);
  if (s && s.raw === raw && s.out === flying) return s.xs;
  const xs = kept(session).map((x) => (out.has(x.id) ? { ...x, out: true } : x));
  snaps.set(session, { raw, out: flying, xs });
  return xs;
}
const has = (session: string, id: string) => kept(session).some((t) => t.id === id);

/** An input enters when its `unqueue` succeeded. Remove's (`tried: "undo"`) shows its Undo for 6 s: the timer only hides an
 * Undo that was never refused and is not in flight. Nothing else removes an item but `drop`. */
export function take(session: string, text: string, tried: Taken["tried"], flying = false): string {
  const id = `${Date.now().toString(36)}.${++ids}`;
  if (flying) out.add(id);
  write(session, [...kept(session), { id, text, tried }]);
  if (tried === "undo") setTimeout(() => { const x = kept(session).find((t) => t.id === id); if (x && !out.has(id) && x.error === undefined) drop(session, id); }, UNDO_MS);
  return id;
}
/** Marks a call of item `id` in flight (one at a time: false when one is). */
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
/** Leaves the list: its `say` or `interrupt` was accepted, the open composer took it, or she discarded it. */
export function drop(session: string, id: string) {
  out.delete(id);
  write(session, kept(session).filter((t) => t.id !== id));
}

const subscribe = (f: () => void) => { listeners.add(f); return () => { listeners.delete(f); }; };
export const useTaken = (session: string) => useSyncExternalStore(subscribe, () => takenOf(session));

/** The session's capacity epoch: one more for every queue or turn update the console receives (F4.2). */
export const epochOf = (session: string) => epochs.get(session) ?? 0;
export const bump = (session: string) => { epochs.set(session, epochOf(session) + 1); };
