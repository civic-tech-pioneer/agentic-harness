// TECHNICAL 18.2: what the page opens until a real kernel exists: the scripted kernel with the whole story played,
// and the session facts (title, profile, provider, workspace) that the kernel stream does not carry yet.
import type { Kernel } from "../kit/kernel";
import { fakeKernel } from "./kernel";
import { story } from "./story";
import { secondAsk } from "./asks";
import { longStory } from "./long";
import type { SidebarSession } from "../src/Sidebar";

export const storyInfo = { id: "s1", title: "Fix the retry bug in sync.ts", profile: "Careful", provider: "Claude Code", workspace: "~/work/sync",
  place: "sync", model: "Claude Sonnet", mode: "Ask before edits" };

// The other sessions, until the console's `list` is wired (S10): the frame's mix.
export const storySessions: SidebarSession[] = [
  { id: "m1", title: "Draft the Sunday note", place: "notes", meta: "notes · asks to send", state: "needs" },
  { id: "d1", title: "Weekly review", place: "site", meta: "site · ChatGPT · 4 min", state: "running" },
  { id: "m2", title: "Sort the notes", place: "notes", meta: "notes · paused until 14:00", state: "paused" },
  { id: "d2", title: "Plan the Lisbon trip", place: "trip", meta: "trip · not running since restart", state: "unknown" },
  { id: "d3", title: "Release notes 0.4", place: "site", meta: "site · Claude Code · yesterday", state: "idle" },
];

/** `?second=1` (dev, the slow checks): a second open ask at the end of the story; `?refuse=busy`: the kernel refuses each answer as busy. */
export function scriptedKernel(): Kernel {
  const q = typeof location === "undefined" ? new URLSearchParams() : new URLSearchParams(location.search);
  const script = q.get("second") ? [...story, secondAsk] : story;
  const fake = fakeKernel(script, q.get("refuse") === "busy" ? { refuseAnswer: "busy" } : {});
  for (let n = 0; n < script.length; n++) fake.play(n);
  return fake;
}

/** `?long=N` (dev): a session of N finished entries with a fast stream running after them, to try the console on a long transcript. */
export function longKernel(n: number): Kernel {
  const script = longStory(n);
  const fake = fakeKernel(script);
  fake.play(0);
  let next = 1, timer: ReturnType<typeof setInterval> | undefined;
  const attach = fake.attach.bind(fake);
  fake.attach = async (...a) => {
    const r = await attach(...a);
    if ("ok" in r && !timer) timer = setInterval(() => { if (next < script.length) fake.play(next++); else clearInterval(timer); }, 20);
    return r;
  };
  return fake;
}

export const longInfo = (n: number) => ({ ...storyInfo, title: `A long session of ${n} entries` });
