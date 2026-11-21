// TECHNICAL 18.2: served by watane, the page runs on the real kernel: the owner's newest probe session (or a new one), and the sidebar from `sessions.list`.
import { wataneKernel, type Kernel, type Watane } from "../kit";
import type { SessionInfo } from "./Header";
import type { SidebarSession } from "./Sidebar";

interface Listed { id: string; title: string; profile: string; backend: string; updated: number }

export interface Served { kernel: Kernel; info: SessionInfo; sessions: SidebarSession[] }

async function ok(w: Watane, fn: string, args: object): Promise<any> {
  const r = (await w.call(fn, args)) as { ok?: unknown; err?: unknown } | null;
  if (!r || !("ok" in r)) throw new Error(`${fn} failed: ${JSON.stringify(r?.err ?? r)}`);
  return r.ok;
}

export async function served(w: Watane): Promise<Served> {
  const kernel = wataneKernel(w);
  const listed = (): Promise<Listed[]> => ok(w, "sessions.list", { after: null, max: 256 });
  let list = await listed();
  const probes = list.filter((s) => s.backend === "probe").sort((a, b) => b.updated - a.updated);
  let cur = probes[0] as Listed | undefined;
  if (!cur) {
    const id: string = await ok(w, "probe.start", { key: "console-probe", count: 1000, "every-ms": 1000 });
    list = await listed(); // the new session's own title, and the sidebar shows it
    cur = list.find((s) => s.id === id) ?? { id, title: "Stage A probe", profile: "", backend: "probe", updated: 0 };
  }
  const info: SessionInfo = { id: cur.id, title: cur.title, profile: cur.profile, provider: "Stage A probe", workspace: "" };
  const sessions: SidebarSession[] = list.map((s) => ({ id: s.id, title: s.title, place: s.backend, meta: `${s.backend} · ${s.profile}`, state: "idle" }));
  return { kernel, info, sessions };
}

/** A served page that could not open a session shows the cause as text in `root`, never the fake. */
export function openFailed(root: HTMLElement, e: unknown) {
  root.textContent = `The console could not open a session: ${e instanceof Error ? e.message : String(e)}`;
}
