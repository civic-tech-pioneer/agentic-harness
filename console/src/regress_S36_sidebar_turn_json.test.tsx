// S36 user-sim (Alice on the real host): the kernel's `list` gives each session's `turn` as the turn JSON of 9.3
// (`{"state":"running","since":...}`, kernel/src/sessions.rs SessionInfo.turn), not a bare word. The served page
// reads it as a word, so every other session in the sidebar shows Idle under Recent: a running one, one that waits
// for her answer (Needs you), and one left Not running by a restart.
import { describe, expect, it } from "vitest";
import { served } from "./served";
import { fakeWatane } from "../kit/watane_fake";

const LIST = [
  { id: "r", title: "Running one", profile: "scratch", backend: "claude-code", turn: JSON.stringify({ state: "running", since: 1 }), updated: 3 },
  { id: "w", title: "Waiting one", profile: "scratch", backend: "claude-code", turn: JSON.stringify({ state: "waiting", asks: ["a1"], "expires-at": 9 }), updated: 2 },
  { id: "u", title: "Restarted one", profile: "scratch", backend: "claude-code", turn: JSON.stringify({ state: "unknown" }), updated: 1 },
];

describe("S36 user-sim", () => {
  it("regress_S36_sidebar_reads_the_kernels_turn_json", async () => {
    const f = fakeWatane({ "sessions.list": () => ({ ok: LIST }) });
    const x = await served(f.w);
    expect(x.sessions.map((s) => [s.id, s.state])).toEqual([["r", "running"], ["w", "needs"], ["u", "unknown"]]);
  });
});
