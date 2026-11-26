// S36-3: a tab whose stream another tab took says so and offers "Use here" (TECHNICAL 18.6 "Another tab", 9.4 `replaced`).
import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { storyInfo as INFO } from "../fake/page";
import { entry } from "../kit/watane_fake";
import { App } from "./App";

afterEach(cleanup);
const S = story.length;
const LAST_SHOWN = 8; // The open ask is the last indexed entry in story.
const AFTER_TAKEOVER = "pushed after use here";
const NOTICE = "This session is open in another tab.";

/** A fake whose attach streams can be ended `replaced` by hand (the fake's own replaced goes to the old stream of the same page). */
async function open({ delayTakeoverSync = false } = {}) {
  const extra: string[][] = [...story];
  const f = fakeKernel(extra);
  for (let n = 0; n < S; n++) f.play(n);
  const ends: Array<(r: string) => void> = [];
  const raw = f.attach.bind(f);
  let pendingSync: (() => void) | undefined;
  f.attach = (session, after, message, end) => {
    const holdSync = delayTakeoverSync && ends.length === 1;
    ends.push(end);
    return raw(session, after, (json) => {
      if (holdSync && JSON.parse(json)._meta?.watane?.type === "synced") {
        pendingSync = () => message(json);
      } else {
        message(json);
      }
    }, end);
  };
  const { rerender } = render(<App kernel={f} info={INFO} />);
  await screen.findByRole("button", { name: "Needs you (1)" });
  const attaches = () => f.calls.filter((c) => c.name === "attach");
  return {
    f, ends, attaches, extra, rerender,
    sync() {
      expect(pendingSync).toBeTypeOf("function");
      act(() => pendingSync?.());
    },
  };
}
const settle = () => act(async () => { await new Promise((r) => setTimeout(r, 50)); });

const notice = () => document.querySelector<HTMLElement>(".wa-replaced");
const bar = () => ["Allow once", "Deny…", /Allow .* this session/].map((name) => screen.getByRole("button", { name }));
const stop = () => screen.getByRole("button", { name: /^Stop(ping…)?$/ });
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
    const { f, ends, attaches, extra, sync } = await open({ delayTakeoverSync: true });
    act(() => ends[0]!("replaced"));
    await settle();
    await userEvent.setup().click(screen.getByRole("button", { name: "Use here" }));
    expect(attaches().map(({ args }) => args)).toEqual([[INFO.id, 0], [INFO.id, LAST_SHOWN]]);
    expect(screen.queryByText(AFTER_TAKEOVER)).toBeNull();
    extra.push([entry(LAST_SHOWN + 1, AFTER_TAKEOVER)]);
    act(() => f.play(extra.length - 1));
    await screen.findByText(AFTER_TAKEOVER);
    expect(notice()).toHaveTextContent(NOTICE);
    sync();
    expect(notice()).toBeNull();
    expect(screen.getAllByText(AFTER_TAKEOVER)).toHaveLength(1);
  });
  it("ac3_send_stop_and_the_ask_buttons_are_off_while_the_notice_shows", async () => {
    const { f, ends, sync } = await open({ delayTakeoverSync: true });
    const user = userEvent.setup();
    const draft = screen.getByRole("textbox", { name: "Message" });
    const status = () => document.querySelector(".wa-status-word")?.textContent;
    const controls = () => [send(), stop(), ...bar()];
    await user.type(draft, "before takeover");
    controls().forEach((control) => expect(control).toBeEnabled());
    await user.click(send());
    await user.click(stop());
    const statusBefore = status();
    await user.type(draft, "after takeover");
    act(() => ends[0]!("replaced"));
    expect(notice()).toHaveTextContent(NOTICE);
    controls().forEach((control) => expect(control).toBeDisabled());
    await user.click(screen.getByRole("button", { name: "Use here" }));
    expect(notice()).toHaveTextContent(NOTICE);
    controls().forEach((control) => expect(control).toBeDisabled());
    expect(status()).toBe(statusBefore);
    sync();
    expect(notice()).toBeNull();
    controls().forEach((control) => expect(control).toBeEnabled());
    await user.click(send());
    await user.click(stop());
    await user.click(screen.getByRole("button", { name: "Allow once" }));
    expect(f.calls.filter(({ name }) => name === "say").map(({ args }) => [args[0], args[2]])).toEqual([
      [INFO.id, "before takeover"], [INFO.id, "after takeover"],
    ]);
    expect(f.calls.filter(({ name }) => name === "interrupt").map(({ args }) => args[0])).toEqual([INFO.id, INFO.id]);
    const answers = f.calls.filter(({ name }) => name === "answer");
    expect(answers).toHaveLength(1);
    expect(answers[0]!.args).toEqual([INFO.id, expect.any(String), "a8", JSON.stringify({ allow: true })]);
  });
  it("ac4_a_new_attach_for_another_session_has_no_notice", async () => {
    const { ends, f, attaches, rerender } = await open();
    act(() => ends[0]!("replaced"));
    expect(notice()).toHaveTextContent(NOTICE);
    rerender(<App kernel={f} info={{ ...INFO, id: "other" }} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    expect(notice()).toBeNull();
    rerender(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    expect(notice()).toBeNull();
    expect(attaches().map(({ args }) => args)).toEqual([[INFO.id, 0], ["other", 0], [INFO.id, 0]]);
  });
  it("ac5_a_dropped_stream_still_reattaches_without_the_notice", async () => {
    const { f, attaches } = await open();
    act(() => f.drop());
    await act(async () => { await new Promise((r) => setTimeout(r, 100)); });
    expect(notice()).toBeNull();
    expect(attaches().length).toBeGreaterThan(1);
  });
});
