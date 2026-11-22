// TECHNICAL 18.2: served by watane, the page runs on the real kernel: the owner's newest session (or the new-session view), and the sidebar from `sessions.list`.
import { wataneKernel, type Kernel, type Watane } from "../kit";
import { parse } from "../kit/wire";
import { metaOf, titleOf, type SessionInfo } from "./Header";
import { shapeOf, type SidebarSession } from "./Sidebar";

interface Listed { id: string; title: string; profile: string; backend: string; turn?: string; updated: number }

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

/** The facts the kernel's stream does not carry (18.4): stage B has Claude Code sessions in a scratch folder, which no judge checks. Only a Claude
 * session of `scratch` runs in a scratch folder (9.1); any other row reads `<backend> · <profile>`. */
const infoOf = (s: Listed): SessionInfo => {
  const claude = /claude/i.test(s.backend), scratch = claude && s.profile === "scratch";
  return {
    id: s.id, title: s.title, profile: s.profile,
    provider: claude ? "Claude Code (unchecked)" : s.backend,
    workspace: scratch ? "Scratch folder" : "",
  };
};
const rowOf = (s: Listed, firstUser?: string): SidebarSession => { const i = infoOf(s); return { id: s.id, title: titleOf(i.title, firstUser), place: i.workspace || i.profile, meta: metaOf(i), state: shapeOf(s.turn as Parameters<typeof shapeOf>[0]) }; };

/** 18.4's title rule for a row the kernel has no title for: its first user message, from the first entries; none (or a failed read) leaves "New session". */
async function firstUserOf(w: Watane, session: string): Promise<string | undefined> {
  try {
    const page = await ok(w, "sessions.read", { session, after: 0, before: null, max: 8 });
    for (const e of (page?.entries ?? []) as string[]) {
      const m = parse(e);
      if (m.t === "message" && m.role === "user") return m.text;
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
