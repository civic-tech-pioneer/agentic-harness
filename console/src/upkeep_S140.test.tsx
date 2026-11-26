// S140 (upkeep): S139 review r1 polish P1.1 and P1.6 (PR #62). A taken-out message's "Stop and send" retry is
// unavailable while the session cannot be interrupted, as the queue row's is; the queue preview keeps 240 characters.
// TECHNICAL 18.5, 18.6 "Queue".
import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel, type FakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { storyInfo as INFO } from "../fake/page";
import { App } from "./App";

afterEach(cleanup);
const w = (m: object) => JSON.stringify({ _meta: { watane: m } });
const queue = (...xs: string[]) => [w({ type: "queue", inputs: xs.map((input, k) => ({ n: k + 7, input })) })];
const settle = () => act(async () => { await new Promise((r) => setTimeout(r, 20)); });
const refuseInterrupt = (f: FakeKernel) => {
  f.interrupt = async (...args: unknown[]) => { f.calls.push({ name: "interrupt", args }); throw Object.assign(new Error("quota"), { kind: "quota" }); };
};
const box = () => screen.getByRole("textbox", { name: "Message" }) as HTMLTextAreaElement;
const items = () => [...document.querySelectorAll(".wa-queue li .wa-queue-text")].map((e) => e.textContent);

describe("S140", () => {
  it.each(["unknown", "resume-lost"])(
    "p1_1_taken_row_stop_and_send_retry_is_unavailable_when_the_turn_is_%s", async (state) => {
      const f = fakeKernel([...story, queue("first one"), [w({ type: "turn", state: { state } })]]);
      for (let n = 0; n <= story.length; n++) f.play(n);
      render(<App kernel={f} info={INFO} />);
      await screen.findByRole("button", { name: "Needs you (1)" });
      refuseInterrupt(f);
      await userEvent.setup().click(screen.getByRole("button", { name: "Stop and send queued message 1" }));
      await settle();
      // the refused interrupt put her text in the composer, which holds it; cleared, the row offers the retry itself
      const u = userEvent.setup();
      expect(box()).toHaveValue("first one");
      await u.clear(box());
      await settle();
      const note = () => document.querySelector(".wa-queue-note")!;
      expect(note()).toHaveTextContent("first one");
      const retry = () => [...note().querySelectorAll("button")].find((b) => b.textContent === "Stop and send")!;
      expect(retry()).toBeEnabled();
      act(() => f.play(story.length + 1));
      await settle();
      if (state === "unknown") expect(retry()).toBeUndefined();
      else {
        expect(retry()).toBeDisabled();
        expect(retry()).toHaveAccessibleDescription("Claude no longer has this session.");
      }
      expect([...note().querySelectorAll("button")].find((b) => b.textContent === "Discard")).toBeEnabled();
    });

  it.each([
    ["exactly 240 characters", "a".repeat(240), "a".repeat(240)],
    ["241 characters", "b".repeat(241), `${"b".repeat(240)}…`],
    ["a pair of code units at the cut", `${"c".repeat(239)}😀tail`, `${"c".repeat(239)}😀…`],
    // the limit counts code points, not UTF-16 units (S140 r1 F1.1): supplementary characters before the cut
    ["exactly 240 emoji", "😀".repeat(240), "😀".repeat(240)],
    ["241 emoji", "😀".repeat(241), `${"😀".repeat(240)}…`],
    ["emoji then letters past the limit", `${"😀".repeat(200)}${"d".repeat(41)}`, `${"😀".repeat(200)}${"d".repeat(40)}…`],
  ])("p1_6_queue_preview_of_%s", async (_name, input, shown) => {
    const f = fakeKernel([...story, queue(input)]);
    for (let n = 0; n <= story.length; n++) f.play(n);
    render(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Edit queued message 1" });
    expect(items()).toEqual([shown]);
  });
});
