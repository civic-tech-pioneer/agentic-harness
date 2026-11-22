// TECHNICAL 18.2: served by watane, the page runs on the real kernel: the owner's newest session (or the new-session view), and the sidebar from `sessions.list`.
import { wataneKernel, type Kernel, type Watane } from "../kit";
import type { SessionInfo } from "./Header";
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

/** The facts the kernel's stream does not carry (18.4): stage B has Claude Code sessions in a scratch folder, which no judge checks. */
const infoOf = (s: Listed): SessionInfo => ({
  id: s.id, title: s.title, profile: s.profile,
  provider: /claude/i.test(s.backend) ? "Claude Code (unchecked)" : s.backend,
  workspace: s.profile === "scratch" ? "Scratch folder" : "",
});
const rowOf = (s: Listed): SidebarSession => ({ id: s.id, title: s.title, place: s.backend, meta: `${s.backend} · ${s.profile}`, state: shapeOf(s.turn as Parameters<typeof shapeOf>[0]) });

export async function served(w: Watane): Promise<Served> {
  const kernel = wataneKernel(w);
  const listed = async () => {
    const list: Listed[] = await ok(w, "sessions.list", { after: null, max: 256 });
    return { list, sessions: list.map(rowOf), info: (id: string) => { const s = list.find((x) => x.id === id); return s && infoOf(s); } };
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
