// S156-2 AC4: a queued row offers Stop and send only while a turn runs; Not running (`unknown`, `resume-lost`) offers Edit and ×.
// TECHNICAL 18.6 "Queue"; frame Main-2 (`.qitem`).
import { afterEach, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { storyInfo as INFO } from "../fake/page";
import { App } from "./App";

afterEach(cleanup);
const w = (m: object) => JSON.stringify({ _meta: { watane: m } });

it.each(["unknown", "resume-lost"])("ac4_a_queued_row_offers_no_stop_and_send_when_the_turn_is_%s", async (state) => {
  const f = fakeKernel([...story, [w({ type: "queue", inputs: [{ n: 7, input: "first one" }] }), w({ type: "turn", state: { state } })]]);
  for (let n = 0; n <= story.length; n++) f.play(n);
  render(<App kernel={f} info={INFO} />);
  await screen.findByRole("button", { name: "Edit queued message 1" });
  expect(screen.getByRole("button", { name: "Remove from queue" })).toBeEnabled();
  expect(screen.queryByRole("button", { name: /Stop and send/ })).toBeNull();
});

it("ac4_a_running_turn_still_offers_it", async () => {
  const f = fakeKernel([...story, [w({ type: "queue", inputs: [{ n: 7, input: "first one" }] }), w({ type: "turn", state: { state: "running", since: 1 } })]]);
  for (let n = 0; n <= story.length; n++) f.play(n);
  render(<App kernel={f} info={INFO} />);
  expect(await screen.findByRole("button", { name: "Stop and send queued message 1" })).toBeEnabled();
});
