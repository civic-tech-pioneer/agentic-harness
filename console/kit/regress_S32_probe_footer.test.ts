// S32 user-sim: Alice's probe session ("Numbered items", turn idle, 9.9) shows a footer
// "Completed in 0 s · 0 tools" that no turn ended (18.3: a footer only at a turn's end), and it
// sits wherever the attach happened, so a reload draws the same transcript differently.
// The bodies below are the exact item bodies the real logic.wasm pushed on its owner stream.
import { describe, expect, it } from "vitest";
import { emptyView, type SessionView } from "./model";
import { reduce } from "./reduce";

const J = JSON.stringify;
const item = (n: number) =>
  J({ _meta: { watane: { type: "entry", entry: { v: 1, i: n, kind: "message", at: 1795255995237 + n, body: { role: "agent", content: [{ type: "text", text: `Item ${n} of 1000` }] } } } } });
const synced = (through: number) => J({ _meta: { watane: { type: "synced", through } } });
const turnIdle = J({ _meta: { watane: { type: "turn", state: { state: "idle", stop: { kind: "completed" } } } } });
const queue = J({ _meta: { watane: { type: "queue", inputs: [] } } });
const feed = (msgs: string[]) => msgs.reduce<SessionView>(reduce, emptyView);

describe("regress_S32 probe footer", () => {
  it("regress_S32_probe_session_shows_no_turn_footer_and_a_reload_draws_the_same_rows", () => {
    // live: the page started the probe and attached before item 1 (after: 0, nothing stored yet)
    const live = feed([synced(0), turnIdle, queue, item(1), item(2), item(3)]);
    // reload: attach(after: 0) once items 1..3 are stored
    const reload = feed([item(1), item(2), item(3), synced(3), turnIdle, queue]);
    expect(live.rows.filter((r) => r.kind === "footer")).toEqual([]);
    expect(reload.rows.map((r) => r.kind)).toEqual(live.rows.map((r) => r.kind));
  });
});
