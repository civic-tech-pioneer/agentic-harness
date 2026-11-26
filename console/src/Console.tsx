// TECHNICAL 18.2: the page's top: an open session, or the new-session view; index.html's static frame shows until the kernel has answered (S35 AC4c).
import { useEffect, useState } from "react";
import type { Kernel, ProfileInfo, Refusal } from "../kit";
import { App } from "./App";
import { Composer } from "./Dock";
import { FOLDER, type SessionInfo } from "./Header";
import { Sidebar, type SidebarSession } from "./Sidebar";
import type { Served } from "./served";
import { startKey } from "./served";

const NEW_KEY = "wa-draft:new";

const BACKEND: Record<string, string> = { claude: "Claude Code", openai: "ChatGPT" };
const backendName = (b: string) => BACKEND[b] ?? b;

/** 18.2, 18.6 "New session": the profiles the console may use (`profiles`, read once on opening; none listed, or no answer, offers none
 * and Start stays off), the selected one's backend, the kernel's own refusals for it (`check` on each selection, field by field), and the
 * first message. A refusal is the kernel's, never a rule here. */
export function NewSession({ kernel, sessions, onNew, start }: { kernel: Kernel; sessions: SidebarSession[]; onNew: () => void; start: (input: string, folder: string | undefined, profile: ProfileInfo) => Promise<void> }) {
  const [folder, setFolder] = useState("");
  const [profiles, setProfiles] = useState<ProfileInfo[] | null>(null);
  const [picked, setPicked] = useState(0);
  const [refusals, setRefusals] = useState<Refusal[]>([]);
  // `profiles` once on opening: the list is the kernel's, an empty one too; a kernel that cannot answer offers none
  useEffect(() => {
    let live = true;
    Promise.resolve().then(() => kernel.profiles()).then(
      (p) => { if (live) { setProfiles(Array.isArray(p) ? p : []); setPicked(0); } },
      () => { if (live) setProfiles([]); });
    return () => { live = false; };
  }, [kernel]);
  const profile: ProfileInfo | undefined = profiles?.[picked] ?? profiles?.[0];
  // `check` on each selection: an answer for a profile no longer selected is dropped, and a new selection shows none until the kernel answers
  useEffect(() => {
    let live = true;
    setRefusals([]);
    if (!profile) return;
    Promise.resolve().then(() => kernel.check(profile.name, "{}")).then((r) => { if (live && Array.isArray(r)) setRefusals(r); }, () => {});
    return () => { live = false; };
  }, [kernel, profile?.name]);
  const on = profile ? backendName(profile.backend) : "";
  const blocked = profiles === null ? "Start is off until the kernel lists your profiles."
    : !profile ? "Start is off: no profile is open to you."
    : refusals.length ? `Start is off: ${profile.name} cannot run on ${on} (${refusals.length} ${refusals.length === 1 ? "refusal" : "refusals"} above).` : undefined;
  return (
    <div className="wa-app" data-view="new-session">
      <Sidebar sessions={sessions} onNew={onNew} />
      <main className="wa-main">
        <header className="wa-header">
          <h1 className="wa-title">New session</h1>
          <div className="wa-chips">
            <span className="wa-chip" title="Profile">Profile <b>{profile?.name ?? "none"}</b></span>
            <label className="wa-chip wa-folder-path">
              {FOLDER} Folder path
              <input value={folder} onChange={(e) => setFolder(e.target.value)}
                aria-describedby="wa-folder-default" />
            </label>
            <span id="wa-folder-default" className="wa-folder-default">A new scratch folder if left blank</span>
          </div>
        </header>
        <div className="wa-scroll">
          <section className="wa-newbody">
            <div role="radiogroup" aria-label="Profile" className="wa-profiles">
              {(profiles ?? []).map((p, n) => (
                <label key={p.name} className="wa-pcard" data-on={n === picked ? "true" : undefined}>
                  <input type="radio" name="wa-profile" checked={n === picked} onChange={() => setPicked(n)} />
                  <b>{p.name}</b>
                  <span>{backendName(p.backend)}{p.model ? ` · ${p.model}` : ""}</span>
                </label>
              ))}
            </div>
            <div className="wa-runson" role="group" aria-label="Runs on">
              <span className="wa-runson-label">Runs on</span>
              {Object.keys(BACKEND).map((b) => (
                <button key={b} type="button" className="wa-seg" aria-pressed={b === profile?.backend} disabled={b !== profile?.backend} title={b === profile?.backend ? undefined : "Set in the profile"}>{BACKEND[b]}</button>
              ))}
            </div>
            <div className="wa-refusals" role="status" aria-live="polite" data-on={refusals.length ? "true" : undefined}>
              {refusals.length ? (
                <>
                  <b>Can't run on {on}</b>
                  {refusals.map((r, n) => <div key={n} className="wa-ref"><code>{r.field}</code><span>{r.reason}</span></div>)}
                </>
              ) : null}
            </div>
          </section>
        </div>
        <div className="wa-dock">
          <Composer queued={0} draftKey={NEW_KEY} placeholder={profile?.backend === "openai" ? "Message ChatGPT…" : "Message Claude…"} sendLabel="Start session" blocked={blocked}
            actions={{ say: (input) => profile ? start(input, folder.trim() ? folder : undefined, profile) : Promise.resolve(), answer: () => {}, stop: () => {}, takeBack: () => {} }} />
        </div>
      </main>
    </div>
  );
}

/** The newest session open, or the new-session view; Start opens the new session in place, and New session goes back to the view. */
export function Console({ served }: { served: Served }) {
  const { kernel } = served;
  const [info, setInfo] = useState<SessionInfo | null>(served.info);
  const [sessions, setSessions] = useState(served.sessions);
  const start = async (input: string, folder: string | undefined, profile: ProfileInfo) => {
    const id = folder === undefined
      ? await kernel.start(startKey(), profile.name, "{}", input)
      : await kernel.start(startKey(), profile.name, "{}", input, folder);
    // the new session's own title and the sidebar come from `list`; a failed read leaves the session open under its first words
    let title = input.length > 60 ? `${input.slice(0, 60)}…` : input;
    const claude = profile.backend === "claude";
    let workspace = folder ?? (claude && profile.name === "scratch" ? "Scratch folder" : "");
    try {
      const l = await served.list();
      setSessions(l.sessions);
      const session = l.info(id);
      title = session?.title ?? title;
      workspace = session?.workspace || workspace;
    } catch { /* the session is started: the sidebar catches up at the next list */ }
    setInfo({ id, title, profile: profile.name, provider: claude ? "Claude Code (unchecked)" : backendName(profile.backend), workspace });
  };
  /** 18.3 `child`, the sidebar: the session is read once first (2.3 `read`), so a refusal (`unknown-session`) rejects with the open session left as it is. */
  const open = async (id: string) => {
    await kernel.read(id, 0, 1);
    let next: SessionInfo | undefined;
    try { const l = await served.list(); setSessions(l.sessions); next = l.info(id); } catch { /* the session is read: it opens under its id */ }
    setInfo(next ?? { id, title: "", profile: "", provider: "", workspace: "" });
  };
  return info
    ? <App kernel={kernel} info={info} sessions={sessions} onNew={() => setInfo(null)} onOpen={open} />
    : <NewSession kernel={kernel} sessions={sessions} onNew={() => setInfo(null)} start={start} />;
}
