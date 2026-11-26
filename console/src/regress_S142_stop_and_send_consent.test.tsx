// S142 outcome (1), user-sim r1: a queued row's "Stop and send" while a folder consent waits (before `initialized`) is a Stop too.
// It must not send `sessions.interrupt` (watane S103 r1: an interrupt before `initialized` lets a later yes answer into the session);
// either the control is not offered there, or it cancels the launch and sends the text by another path.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { fakeWatane, items, synced } from "../kit/watane_fake";
import { Console } from "./Console";
import { served } from "./served";

afterEach(cleanup);

const folder = "/home/alice/Notes";
const meta = (o: object) => JSON.stringify({ _meta: { watane: o } });

it("regress_S142_stop_and_send_in_folder_consent_sends_no_interrupt", async () => {
  const host = fakeWatane({
    "sessions.list": () => ({ ok: [{ id: "n1", title: "Notes", profile: "scratch", backend: "claude", workspace: folder }] }),
    "sessions.attach": () => ({ ok: "st1" }),
    "sessions.cancel": () => null,
    "sessions.interrupt": () => ({ ok: null }),
    "sessions.unqueue": () => ({ ok: null }),
    "sessions.say": () => ({ ok: 2 }),
  });
  render(<Console served={await served(host.w)} />);
  await waitFor(() => expect(host.calls.some((call) => call.fn === "sessions.attach")).toBe(true));
  act(() => host.push({ ...items(
    [1, synced(0)],
    [2, meta({ type: "turn", state: { state: "running", since: 1, activity: { kind: "consent", what: `workspace:local/claude-code:${folder}` } } })],
    [3, meta({ type: "queue", inputs: [{ n: 1, input: "also look at todo.md" }] })],
  ), stream: "st1" }));
  await screen.findByText(`Waiting for your answer in watane for ${folder}`);
  const control = screen.queryByRole("button", { name: "Stop and send queued message 1" });
  if (control) {
    fireEvent.click(control);
    await act(async () => { await new Promise((r) => setTimeout(r, 100)); });
  }
  expect(host.calls.filter(({ fn }) => fn === "sessions.interrupt")).toEqual([]);
});
