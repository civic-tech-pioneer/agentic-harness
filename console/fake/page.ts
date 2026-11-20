// TECHNICAL 18.2: what the page opens until a real kernel exists: the scripted kernel with the whole story played,
// and the session facts (title, profile, provider, workspace) that the kernel stream does not carry yet.
import type { Kernel } from "../kit/kernel";
import { fakeKernel } from "./kernel";
import { story } from "./story";

export const storyInfo = { id: "s1", title: "Fix the retry bug in sync.ts", profile: "Careful", provider: "Claude Code", workspace: "~/work/sync" };

export function scriptedKernel(): Kernel {
  const fake = fakeKernel(story);
  for (let n = 0; n < story.length; n++) fake.play(n);
  return fake;
}
