// Adversary S23 pass 9: a refused deny remounts the deny row; its autoFocus pulls focus out of the composer.
import { describe, expect, it } from "vitest";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { secondAsk } from "../fake/asks";
import { App } from "./App";
import { storyInfo as INFO } from "../fake/page";

describe("adv S23 fix10", () => {
  it("adv_S23_fix10_refused_deny_pulls_focus_from_the_composer", async () => {
    const f = fakeKernel([...story]);
    for (let n = 0; n < story.length; n++) f.play(n);
    render(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    const card = document.querySelector<HTMLElement>("[data-asking]")!;
    let refuse: () => void = () => {};
    f.answer = ((...a: unknown[]) => { f.calls.push({ name: "answer", args: a });
      return new Promise<void>((_, rej) => { refuse = () => rej(Object.assign(new Error("busy"), { kind: "busy" })); }); }) as typeof f.answer;
    const u = userEvent.setup();
    await u.click(within(card).getByRole("button", { name: "Deny…" }));
    await u.keyboard("not this one{Enter}");
    expect(within(card).getByRole("status")).toHaveTextContent("Answered, waiting");
    // the participant moves on to the composer; the kernel's busy refusal lands while they type
    const msg = screen.getByRole("textbox", { name: "Message" });
    await u.click(msg);
    await u.keyboard("keep");
    await act(async () => { refuse(); await new Promise((r) => setTimeout(r, 0)); });
    await screen.findByRole("alert");
    expect(msg).toHaveFocus(); // fix10 AC3: focus outside the card stays where it was
  });

  it("adv_S23_fix10_refused_deny_pulls_focus_from_another_card", async () => {
    const f = fakeKernel([...story, secondAsk]);
    for (let n = 0; n <= story.length; n++) f.play(n);
    render(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (2)" });
    const a = document.querySelector<HTMLElement>('[data-call="e2"]')!, b = document.querySelector<HTMLElement>('[data-call="e3"]')!;
    let refuse: () => void = () => {};
    f.answer = ((...x: unknown[]) => { f.calls.push({ name: "answer", args: x });
      return new Promise<void>((_, rej) => { refuse = () => rej(Object.assign(new Error("busy"), { kind: "busy" })); }); }) as typeof f.answer;
    const u = userEvent.setup();
    await u.click(within(a).getByRole("button", { name: "Deny…" }));
    await u.keyboard("no{Enter}");
    // the participant goes on to the other card's Deny… row and is typing its reason
    await u.click(within(b).getByRole("button", { name: "Deny…" }));
    const why = within(b).getByRole("textbox");
    await u.keyboard("wrong file");
    await act(async () => { refuse(); await new Promise((r) => setTimeout(r, 0)); });
    await within(a).findByRole("alert");
    expect(why).toHaveFocus(); // fix10 AC3: focus in another card is never moved
  });
});
