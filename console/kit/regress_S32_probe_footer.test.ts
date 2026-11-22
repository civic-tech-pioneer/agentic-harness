// S32 user-sim, kept for a Claude turn (S35-4): a footer is drawn only where a turn ended (18.3), and a reload draws the same entry rows as the live stream.
// The probe session this once covered is gone (9.9, removed); the items below are what the kernel pushes for the fake claude's default turn.
import { describe, expect, it } from "vitest";
import { emptyView, type SessionView } from "./model";
import { reduce } from "./reduce";
import turn from "./fixtures/fake_claude_turn.json";

const [synced0, userEntry, running, chunk, agentEntry, idle] = turn as string[];
const synced = (through: number) => JSON.stringify({ _meta: { watane: { type: "synced", through } } });
const queue = JSON.stringify({ _meta: { watane: { type: "queue", inputs: [] } } });
const feed = (msgs: string[]) => msgs.reduce<SessionView>(reduce, emptyView);

describe("regress_S32 footer", () => {
  it("regress_S32_a_turn_shows_one_footer_at_its_end_and_a_reload_draws_the_same_rows", () => {
    const live = feed([synced0!, userEntry!, running!, chunk!, agentEntry!, idle!, queue]);
    const reload = feed([userEntry!, agentEntry!, synced(2), idle!, queue]);
    expect(live.rows.filter((r) => r.kind === "footer")).toHaveLength(1);
    // the footer is a live moment (the turn ran, then ended on this stream): a reload shows the entries alone, the same ones, once each
    expect(reload.rows.map((r) => r.kind)).toEqual(live.rows.filter((r) => r.kind !== "footer").map((r) => r.kind));
  });
  it("regress_S32_no_footer_before_a_turn_ends", () => {
    expect(feed([synced0!, userEntry!, running!, chunk!, agentEntry!]).rows.filter((r) => r.kind === "footer")).toEqual([]);
    expect(feed([synced0!, userEntry!, agentEntry!, synced(2), idle!]).rows.filter((r) => r.kind === "footer")).toEqual([]);
  });
});
