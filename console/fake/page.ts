// TECHNICAL 18.2: what the page opens until a real kernel exists: the scripted kernel with the whole story played,
// and the session facts (title, profile, provider, workspace) that the kernel stream does not carry yet.
import type { Kernel } from "../kit/kernel";
import { fakeKernel } from "./kernel";
import { story } from "./story";
import type { SidebarSession } from "../src/Sidebar";

export const storyInfo = { id: "s1", title: "Fix the retry bug in sync.ts", profile: "Careful", provider: "Claude Code", workspace: "~/work/sync",
  place: "sync", model: "Claude Sonnet", mode: "Ask before edits" };

// The other sessions, until the console's `list` is wired (S10): the frame's mix.
export const storySessions: SidebarSession[] = [
  { id: "m1", title: "Reply to Ivy about Sunday", place: "Mail", meta: "Mail · asks to send", state: "needs" },
  { id: "d1", title: "Weekly review", place: "Desk", meta: "Desk · ChatGPT · 4 min", state: "running" },
  { id: "m2", title: "Sort the inbox", place: "Mail", meta: "Mail · paused until 14:00", state: "paused" },
  { id: "d2", title: "Plan the Lisbon trip", place: "Desk", meta: "Desk · not running since restart", state: "unknown" },
  { id: "d3", title: "Release notes 0.4", place: "Desk", meta: "Desk · yesterday", state: "idle" },
];

export function scriptedKernel(): Kernel {
  const fake = fakeKernel(story);
  for (let n = 0; n < story.length; n++) fake.play(n);
  return fake;
}
