// S142 r2, user-sim outcome (1): during the folder-consent wait the queued row's "Stop and send" is disabled, but nothing says why:
// no visible line, no title, no accessible description. "Open the question" beside it carries "This console cannot do this yet.";
// the disabled Stop and send must carry its reason too (the folder question in watane must be answered or the turn stopped first).
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { fakeWatane, items, synced } from "../kit/watane_fake";
import { Console } from "./Console";
import { served } from "./served";

afterEach(cleanup);

const folder = "/home/alice/Notes";
const meta = (o: object) => JSON.stringify({ _meta: { watane: o } });

it("regress_S142_r2_disabled_stop_and_send_says_why", async () => {
  const host = fakeWatane({
    "sessions.list": () => ({ ok: [{ id: "n1", title: "Notes", profile: "scratch", backend: "claude", workspace: folder }] }),
    "sessions.attach": () => ({ ok: "st1" }),
  });
  render(<Console served={await served(host.w)} />);
  await waitFor(() => expect(host.calls.some((call) => call.fn === "sessions.attach")).toBe(true));
  act(() => host.push({ ...items(
    [1, synced(0)],
    [2, meta({ type: "turn", state: { state: "running", since: 1, activity: { kind: "consent", what: `workspace:local/claude-code:${folder}` } } })],
    [3, meta({ type: "queue", inputs: [{ n: 1, input: "also look at todo.md" }] })],
  ), stream: "st1" }));
  await screen.findByText(`Waiting for your answer in watane for ${folder}`);
  const control = screen.getByRole("button", { name: "Stop and send queued message 1" });
  expect(control).toBeDisabled();
  expect(control).toHaveAccessibleDescription(/consent|question|answer/i);
});
