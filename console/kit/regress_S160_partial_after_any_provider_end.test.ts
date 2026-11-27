// S160 item 3, user-sim: a half-streamed message is never stored, so a reload drops it. The live view drops it only when the
// provider stream ended `gone`; any other ending of the provider stream (TECHNICAL 2.1: the outcome's name is the `cause`,
// e.g. `failed`, `lagging`) leaves the half message on screen until a reload, the same stale view outcome 3 removes for `gone`.
import { expect, it } from "vitest";
import { emptyView } from "./model";
import { reduce } from "./reduce";

const J = JSON.stringify;
const w = (m: object) => J({ _meta: { watane: m } });
const chunk = (text: string, i?: number) => J({ sessionUpdate: "agent_message_chunk", content: { type: "text", text }, _meta: { watane: { at: 1000, i } } });

it.each(["gone", "failed", "lagging"])("regress_S160_half_streamed_message_dropped_when_provider_ends_%s", (cause) => {
  const v = [
    w({ type: "synced", through: 0 }),
    w({ type: "turn", state: { state: "running", since: 1 } }),
    chunk("Finished message", 1),
    chunk("Half streamed message"),
    w({ type: "turn", state: { state: "idle", stop: { kind: "failed", error: "provider-ended", cause } } }),
  ].reduce(reduce, emptyView);
  const agent = v.rows.filter((r) => r.kind === "agent").map((r) => (r as { text: string }).text);
  expect(agent).toContain("Finished message");
  expect(agent).not.toContain("Half streamed message");
});
