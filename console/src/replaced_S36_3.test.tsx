// S36-3: a tab whose stream another tab took says so and offers "Use here" (TECHNICAL 18.6 "Another tab", 9.4 `replaced`).
import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { storyInfo as INFO } from "../fake/page";
import { App } from "./App";

afterEach(cleanup);
const S = story.length;
const NOTICE = "This session is open in another tab.";

/** A fake whose attach streams can be ended `replaced` by hand (the fake's own replaced goes to the old stream of the same page). */
async function open() {
  const extra: string[][] = [...story];
  const f = fakeKernel(extra);
  for (let n = 0; n < S; n++) f.play(n);
  const ends: Array<(r: string) => void> = [];
  const raw = f.attach.bind(f);
  f.attach = (s, a, m, e) => { ends.push(e); return raw(s, a, m, e); };
  render(<App kernel={f} info={INFO} />);
  await screen.findByRole("button", { name: "Needs you (1)" });
  const attaches = () => f.calls.filter((c) => c.name === "attach");
  return { f, ends, attaches, extra };
}
const settle = () => act(async () => { await new Promise((r) => setTimeout(r, 50)); });

const notice = () => document.querySelector<HTMLElement>(".wa-replaced");
const bar = () => ["Allow once", "Deny…", /Allow .* this session/].map((n) => screen.queryByRole("button", { name: n }));
const send = () => screen.getByRole("button", { name: /^(Send|Queue) message$/ });

describe("S36-3", () => {
  it("ac1_replaced_shows_the_notice_and_does_not_reattach", async () => {
    const { ends, attaches } = await open();
    expect(notice()).toBeNull();
    act(() => ends[0]!("replaced"));
    await settle();
    expect(notice()?.getAttribute("role")).toBe("status");
    expect(notice()?.textContent).toContain(NOTICE);
    expect(screen.getByRole("button", { name: "Use here" })).toBeInTheDocument();
    await act(async () => { await new Promise((r) => setTimeout(r, 1500)); });
    expect(attaches()).toHaveLength(1);
  });
  it("ac2_use_here_attaches_after_the_last_index_and_the_notice_goes_once_synced", async () => {
    const { f, ends, attaches, extra } = await open();
    act(() => ends[0]!("replaced"));
    await settle();
    await userEvent.setup().click(screen.getByRole("button", { name: "Use here" }));
    await settle();
    expect(attaches()).toHaveLength(2);
    const after = attaches()[1]!.args[1] as number;
    expect(after).toBeGreaterThan(0);
    expect(document.body.textContent).not.toContain("pushed after use here");
    const entry = JSON.stringify({ _meta: { watane: { type: "entry", entry: { v: 1, i: after + 1, kind: "message", at: 1, body: { role: "agent", content: [{ type: "text", text: "pushed after use here" }] } } } } });
    extra.push([entry]);
    act(() => f.play(extra.length - 1));
    await settle();
    expect(document.body.textContent!.split("pushed after use here")).toHaveLength(2);
  });
  it("ac3_send_stop_and_the_ask_buttons_are_off_while_the_notice_shows", async () => {
    const { ends } = await open();
    expect(bar().every((b) => b && !b.hasAttribute("disabled"))).toBe(true);
    act(() => ends[0]!("replaced"));
    await settle();
    expect(bar().every((b) => b && b.hasAttribute("disabled"))).toBe(true);
    const stop = screen.queryByRole("button", { name: /Stop/ });
    if (stop) expect(stop).toBeDisabled();
    expect(send()).toBeDisabled();
    await userEvent.setup().click(screen.getByRole("button", { name: "Use here" }));
    await settle();
    expect(bar().every((b) => b && !b.hasAttribute("disabled"))).toBe(true);
  });
  it("ac4_a_new_attach_for_another_session_has_no_notice", async () => {
    const { ends, f } = await open();
    act(() => ends[0]!("replaced"));
    await settle();
    expect(notice()).not.toBeNull();
    cleanup();
    render(<App kernel={f} info={{ ...INFO, id: "other" }} />);
    await settle();
    expect(notice()).toBeNull();
  });
  it("ac5_a_dropped_stream_still_reattaches_without_the_notice", async () => {
    const { f, attaches } = await open();
    act(() => f.drop());
    await act(async () => { await new Promise((r) => setTimeout(r, 100)); });
    expect(notice()).toBeNull();
    expect(attaches().length).toBeGreaterThan(1);
  });
});
