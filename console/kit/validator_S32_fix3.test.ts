// S32-fix3 validator: the footer rule moves only the first `turn` after an attach (no turn seen
// yet). A turn the console saw paused (9.3: a usage limit; the turn is still active) and then
// ended keeps its footer, as before fix3 (AC3: nothing else changes).
import { expect, it } from "vitest";
import { emptyView, type SessionView } from "./model";
import { reduce } from "./reduce";

const J = JSON.stringify;
const turn = (state: object) => J({ _meta: { watane: { type: "turn", state } } });
const synced = J({ _meta: { watane: { type: "synced", through: 0 } } });
const idle = turn({ state: "idle", stop: { kind: "interrupted" }, seconds: 4, tools: 1 });
const feed = (msgs: string[]) => msgs.reduce<SessionView>(reduce, emptyView);
const footers = (v: SessionView) => v.rows.filter((r) => r.kind === "footer");

it("validator_S32_fix3_paused_then_idle_keeps_its_footer", () => {
  expect(footers(feed([synced, turn({ state: "paused", until: 9 }), idle]))).toMatchObject([{ seconds: 4, tools: 1 }]);
  expect(footers(feed([synced, turn({ state: "paused" }), idle]))).toHaveLength(1);
  expect(footers(feed([synced, idle]))).toEqual([]);
});
