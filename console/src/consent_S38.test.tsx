import { afterEach, expect, it } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { fakeWatane, items, synced } from "../kit/watane_fake";
import { Console } from "./Console";
import { served } from "./served";

afterEach(cleanup);

const folder = "/home/alice/Notes";
const what = `workspace:${folder}`;
const tick = () => act(async () => { await new Promise((r) => setTimeout(r, 60)); }); // the view lists the kernel's profiles first (S40 r1)
const turn = (state: object) => JSON.stringify({ _meta: { watane: { type: "turn", state } } });

// the consent wait in host 19.7's form (with the provider identity) and in the older one (the folder alone, S140 P1.3)
const current = `workspace:local/claude-code:${folder}`;
it.each([
  ["gone", "Connection to Claude Code ended", current],
  [`consent-refused(${current})`, `Workspace access was refused for ${folder}`, current],
  [`consent-refused(${what})`, `Workspace access was refused for ${folder}`, what],
])("ac1_ac2_shows_the_folder_wait_and_outcome_%s", async (cause, words, waiting) => {
  let started = false;
  const row = { id: "n1", title: "Notes session", profile: "scratch", backend: "claude", workspace: folder, updated: 1 };
  const host = fakeWatane({
    "sessions.profiles": () => ({ ok: [{ name: "scratch", backend: "claude", model: null }] }),
    "sessions.check": () => ({ ok: [] }),
    "sessions.list": () => ({ ok: started ? [row] : [] }),
    "sessions.start-in-folder": () => { started = true; return { ok: "n1" }; },
    "sessions.attach": () => ({ ok: "st1" }),
  });
  const page = render(<Console served={await served(host.w)} />);
  await tick();
  fireEvent.change(screen.getByRole("textbox", { name: "Folder path" }), { target: { value: folder } });
  fireEvent.change(screen.getByRole("textbox", { name: "Message" }), { target: { value: "Read my notes" } });
  fireEvent.click(screen.getByRole("button", { name: "Start session" }));
  await waitFor(() => expect(host.calls.some((call) => call.fn === "sessions.attach")).toBe(true));
  expect(host.calls.find((call) => call.fn === "sessions.start-in-folder")?.args).toEqual({
    key: expect.any(String), profile: "scratch", overlay: "{}", input: "Read my notes", folder,
  });
  expect(screen.getByTitle("Workspace")).toHaveTextContent(folder);
  const push = (...messages: [number, string][]) => act(() => host.push({ ...items(...messages), stream: "st1" }));
  push([1, synced(0)], [2, turn({ state: "running", since: 1, activity: { kind: "consent", what: waiting } })]);
  expect(await screen.findByText(`Waiting for your answer in watane for ${folder}`)).toBeInTheDocument();
  push([3, turn({ state: "running", since: 1 })]);
  await waitFor(() => expect(screen.queryByText(`Waiting for your answer in watane for ${folder}`)).toBeNull());
  push([4, turn({ state: "idle", stop: { kind: "failed", error: "provider-ended", cause } })]);
  expect(await screen.findByText(`Failed: ${words}`)).toBeInTheDocument();
  page.unmount();
  render(<Console served={await served(host.w)} />);
  await tick();
  expect(screen.getByTitle("Workspace")).toHaveTextContent(folder);
});

it("ac1_keeps_the_folder_and_message_when_the_host_rejects_the_path", async () => {
  const host = fakeWatane({
    "sessions.profiles": () => ({ ok: [{ name: "scratch", backend: "claude", model: null }] }),
    "sessions.check": () => ({ ok: [] }),
    "sessions.list": () => ({ ok: [] }),
    "sessions.start-in-folder": () => ({ err: { backend: "bad-launch: workspace: protected folder" } }),
  });
  render(<Console served={await served(host.w)} />);
  await tick();
  const path = screen.getByRole("textbox", { name: "Folder path" });
  const message = screen.getByRole("textbox", { name: "Message" });
  fireEvent.change(path, { target: { value: "/home/alice/.ssh" } });
  fireEvent.change(message, { target: { value: "Read" } });
  fireEvent.click(screen.getByRole("button", { name: "Start session" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("protected folder");
  expect(path).toHaveValue("/home/alice/.ssh");
  expect(message).toHaveValue("Read");
  expect(host.calls.some((call) => call.fn === "sessions.attach")).toBe(false);
});

it.each([
  ["canonical path", { ok: [{ id: "n1", title: "Read", profile: "scratch", backend: "claude", workspace: folder, updated: 1 }] }, folder],
  ["typed path when list fails", { err: "busy" }, "/home/alice/link-to-notes"],
])("shows the %s after Start", async (_case, list, expected) => {
  const typed = "/home/alice/link-to-notes";
  let started = false;
  const host = fakeWatane({
    "sessions.profiles": () => ({ ok: [{ name: "scratch", backend: "claude", model: null }] }),
    "sessions.check": () => ({ ok: [] }),
    "sessions.list": () => started ? list : { ok: [] },
    "sessions.start-in-folder": () => { started = true; return { ok: "n1" }; },
    "sessions.attach": () => ({ ok: "st1" }),
    "sessions.read": () => ({ ok: { entries: [], last: 0 } }),
  });
  render(<Console served={await served(host.w)} />);
  await tick();
  fireEvent.change(screen.getByRole("textbox", { name: "Folder path" }), { target: { value: typed } });
  fireEvent.change(screen.getByRole("textbox", { name: "Message" }), { target: { value: "Read" } });
  fireEvent.click(screen.getByRole("button", { name: "Start session" }));
  expect(await screen.findByTitle("Workspace")).toHaveTextContent(expected);
});

async function openConsent(waiting: string, backend = "claude") {
  const host = fakeWatane({
    "sessions.list": () => ({ ok: [{ id: "n1", title: "Notes", profile: "scratch", backend, workspace: backend === "claude" ? folder : null }] }),
    "sessions.attach": () => ({ ok: "st1" }),
    "sessions.cancel": () => null,
    "sessions.interrupt": () => ({ ok: null }),
  });
  render(<Console served={await served(host.w)} />);
  await waitFor(() => expect(host.calls.some((call) => call.fn === "sessions.attach")).toBe(true));
  host.push({ ...items([1, synced(0)], [2, turn({ state: "running", since: 1, activity: { kind: "consent", what: waiting } })]), stream: "st1" });
  return () => host.calls.filter(({ fn }) => fn === "sessions.cancel" || fn === "sessions.interrupt");
}

it.each(["button", "Escape"])("ac1_stops_a_folder_consent_by_cancelling_the_provider_via_%s", async (control) => {
  const stops = await openConsent(current);
  await screen.findByText(`Waiting for your answer in watane for ${folder}`);
  if (control === "button") fireEvent.click(screen.getByRole("button", { name: "Stop" }));
  else fireEvent.keyDown(screen.getByRole("textbox", { name: "Message" }), { key: "Escape" });
  expect(stops()).toEqual([{ fn: "sessions.cancel", args: { session: "n1" } }]);
});

it("leaves_other_consent_stops_on_the_existing_interrupt_path", async () => {
  const stops = await openConsent("toolset:notes", "openai");
  await screen.findByText("Waiting for your answer in watane");
  fireEvent.click(screen.getByRole("button", { name: "Stop" }));
  expect(stops()).toEqual([{ fn: "sessions.interrupt", args: { session: "n1", replacement: null } }]);
});
