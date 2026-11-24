// TECHNICAL 18.2: served by watane, the page runs on the real kernel: the owner's newest session (or the new-session view), and the sidebar from `sessions.list`.
import { wataneKernel, type Kernel, type Watane } from "../kit";
import { joiner } from "../kit/join";
import { parse } from "../kit/wire";
import { metaOf, titleOf, type SessionInfo } from "./Header";
import { shapeOf, type SidebarSession } from "./Sidebar";

interface Listed { id: string; title: string; profile: string; backend: string; workspace?: string | null; turn?: string; updated: number }

/** `info` is null when the owner has no session: the page then shows the new-session view. `list` reads the sidebar again (a session just started). */
export interface Served { kernel: Kernel; info: SessionInfo | null; sessions: SidebarSession[]; list(): Promise<{ sessions: SidebarSession[]; info(id: string): SessionInfo | undefined }> }

async function ok(w: Watane, fn: string, args: object): Promise<any> {
  const r = (await w.call(fn, args)) as { ok?: unknown; err?: unknown } | null;
  if (!r || !("ok" in r)) throw new Error(`${fn} failed: ${JSON.stringify(r?.err ?? r)}`);
  return r.ok;
}

/** A fresh client key per start: kept keys outlive a delete (9.1), so a fixed key would return a deleted session's id. */
export const startKey = (): string => `console-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;

/** Served by watane (host 8.1): a `<local>.localhost` origin. Vite's `localhost:<port>` and the slow tests' 127.0.0.1 are not; index.html asks for /watane.js everywhere, so that request does not tell. */
export const isServed = (host: string): boolean => /^[^.]+\.localhost$/.test(host);

/** The facts the kernel's stream does not carry (18.4): Claude Code sessions currently use the unchecked provider. The kernel reports a picked path,
 * and scratch sessions retain their default label. No model or mode: a stored `profile/scratch` lists
 * as the built-in does, so the console cannot know them until the kernel reports them (S35 r2 F2.5). */
const infoOf = (s: Listed): SessionInfo => {
  const claude = /claude/i.test(s.backend), scratch = claude && s.profile === "scratch";
  return {
    id: s.id, title: s.title, profile: s.profile,
    provider: claude ? "Claude Code (unchecked)" : s.backend,
    workspace: s.workspace ?? (scratch ? "Scratch folder" : ""),
  };
};
/** 2.3 `list`: a session's `turn` is the turn JSON of 9.3 (`{"state":"running",...}`); its `state` names the sidebar's shape (a bare word is read as one too). */
const stateOf = (turn?: string): Parameters<typeof shapeOf>[0] => {
  if (turn === undefined) return undefined;
  try { const o = JSON.parse(turn); if (o && typeof o === "object" && typeof o.state === "string") return o.state; } catch { /* a bare word */ }
  return turn as Parameters<typeof shapeOf>[0];
};
const rowOf = (s: Listed, firstUser?: string): SidebarSession => { const i = infoOf(s); return { id: s.id, title: titleOf(i.title, firstUser), place: i.workspace || i.profile, meta: metaOf(i), state: shapeOf(stateOf(s.turn)) }; };

/** 18.4's title rule for a row the kernel has no title for: its first user message, from the first 8 entries, a message stored as `part` entries
 * joined first (9.4, S35 r2 F2.3); none (or a failed read) leaves "New session". */
async function firstUserOf(w: Watane, session: string): Promise<string | undefined> {
  try {
    const join = joiner();
    let after = 0, seen = 0;
    // a group is at most 4096 parts (kit/join.ts), so the read stops after 8 joined entries or 4096 + 8 stored ones
    for (let raw = 0; seen < 8 && raw < 4096 + 8;) {
      const page = await ok(w, "sessions.read", { session, after, before: null, max: 64 });
      const entries = (page?.entries ?? []) as string[];
      if (!entries.length) break;
      raw += entries.length;
      for (const e of entries) {
        for (const out of join.push(e)) {
          const m = parse(out);
          if (!("i" in m)) continue;
          if (m.t === "message" && m.role === "user") return m.text;
          if (++seen >= 8) return undefined;
        }
      }
      // `last` is the session's high-water index (2.3): the next page starts after this page's last entry
      let end: unknown; try { end = JSON.parse(entries[entries.length - 1]!).i; } catch { break; }
      if (typeof end !== "number" || end <= after || end >= (page?.last ?? 0)) break;
      after = end;
    }
  } catch { /* the row keeps "New session" */ }
  return undefined;
}

export async function served(w: Watane): Promise<Served> {
  const kernel = wataneKernel(w);
  const listed = async () => {
    const list: Listed[] = await ok(w, "sessions.list", { after: null, max: 256 });
    const firsts = await Promise.all(list.map((s) => (s.title ? undefined : firstUserOf(w, s.id))));
    return { list, sessions: list.map((s, n) => rowOf(s, firsts[n])), info: (id: string) => { const s = list.find((x) => x.id === id); return s && infoOf(s); } };
  };
  const first = await listed();
  const newest = [...first.list].sort((a, b) => b.updated - a.updated)[0];
  return {
    kernel, sessions: first.sessions, info: newest ? infoOf(newest) : null,
    list: async () => { const l = await listed(); return { sessions: l.sessions, info: l.info }; },
  };
}

/** A served page that could not open a session shows the cause as text in `root`, never the fake. */
export function openFailed(root: HTMLElement, e: unknown) {
  root.textContent = `The console could not open a session: ${e instanceof Error ? e.message : String(e)}`;
}

export const NO_WATANE_JS = "watane.js did not load";
