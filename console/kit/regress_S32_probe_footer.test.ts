// S32 user-sim, kept for a Claude turn (S35-4): a footer is drawn only where a turn ended (18.3), and a reload draws the same transcript as the live stream, footer included.
// The probe session this once covered is gone (9.9, removed); the items are what the kernel pushes (live) and replays for the fake claude's default turn.
import { describe, expect, it } from "vitest";
import { emptyView, type SessionView } from "./model";
import { reduce } from "./reduce";
import fixture from "./fixtures/fake_claude_turn.json";

const { live: turn, replay } = fixture as { live: string[]; replay: string[] };
const feed = (msgs: string[]) => msgs.reduce<SessionView>(reduce, emptyView);
const kinds = (v: SessionView) => v.rows.map((r) => r.kind);

describe("regress_S32 footer", () => {
  it("regress_S32_a_turn_shows_one_footer_at_its_end_and_a_reload_draws_the_same_rows", () => {
    const live = feed(turn), reload = feed(replay);
    expect(kinds(live).filter((k) => k === "footer")).toHaveLength(1);
    expect(kinds(reload)).toEqual(kinds(live));
    expect(reload.rows.map((r) => r.kind === "agent" ? r.text : null)).toEqual(live.rows.map((r) => r.kind === "agent" ? r.text : null));
  });
  it("regress_S32_no_footer_before_a_turn_ends", () => {
    expect(kinds(feed(turn.slice(0, 5))).filter((k) => k === "footer")).toEqual([]);
    expect(kinds(feed(replay.slice(0, 3))).filter((k) => k === "footer")).toEqual([]);
  });
});
