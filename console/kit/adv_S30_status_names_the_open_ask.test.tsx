// Adversary S30 (TECHNICAL 18.5 the `waiting` row, 14 "An ask shows the tool call that will run"; S30-2: the status line "names the next one
// (as Needs you would go to) and its clock counts down to that ask's own `expires-at`").
// An ask id is used again after it closed (9.8: only an id still open is refused; S30-1 AC6 tests the reuse). The status line's clock follows
// the open ask (S30-2), but its subject is the first row whose `ask.id` matches, which is the old, decided card of the same id on another call.
// So the line reads "Waiting for you · Edit src/old.ts · denied in 1:00" while the open ask, its card and Needs you are about src/new.ts.
import { afterEach, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { emptyView } from "./model";
import { reduce } from "./reduce";
import { StatusLine } from "./status";

afterEach(cleanup);
const J = JSON.stringify;
const entry = (i: number, kind: string, body: object) => J({ v: 1, i, kind, at: i, body });
const ask = (i: number, id: string, call: string, extra: object = {}) => entry(i, "ask", { id, kind: "ask", toolCallId: call, input: { file_path: `src/${call}.ts` }, "expires-at": 61_000,
  toolCall: { toolCallId: call, title: `Edit src/${call}.ts`, kind: "edit", status: "pending", locations: [{ path: `src/${call}.ts` }] }, ...extra });

it("adv_S30_status_subject_names_the_open_asks_call_not_an_old_card_of_its_id", () => {
  const v0 = [ask(2, "a1", "old"), ask(3, "a1", "old", { decision: { outcome: "deny", by: "alice" } }), ask(4, "a1", "new")].reduce(reduce, emptyView);
  const view = { ...v0, turn: { state: "waiting" as const, asks: ["a1"], expiresAt: 61_000 } };
  expect(view.openAsks).toEqual(["a1"]); // the open ask is the one on src/new.ts
  render(<StatusLine view={view} now={1_000} stopping={false} actions={{} as never} onNeedsYou={() => {}} answered={0} />);
  expect(screen.getByRole("status").textContent).toContain("new.ts");
});
