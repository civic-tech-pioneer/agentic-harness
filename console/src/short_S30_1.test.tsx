// S30-1 AC3: a short decision ends "Answered, waiting" exactly as a full decision does (TECHNICAL 9.4 rule 4, "The short decision").
import { describe, expect, it } from "vitest";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { decided, shortDecided } from "../fake/asks";
import { story } from "../fake/story";
import { App } from "./App";
import { storyInfo as INFO } from "../fake/page";

const card = () => document.querySelector<HTMLElement>(`[data-call="e2"]`)!;
async function answered(decision: string[]) {
  const f = fakeKernel([...story, decision]);
  for (let n = 0; n < story.length; n++) f.play(n);
  f.answer = (() => new Promise(() => {})) as typeof f.answer; // no refusal, no reply: the decision entry is what ends the wait
  render(<App kernel={f} info={INFO} />);
  await screen.findByRole("button", { name: "Needs you (1)" });
  const u = userEvent.setup();
  await u.click(within(card()).getByRole("button", { name: "Allow once" }));
  expect(within(card()).getByRole("status")).toHaveTextContent("Answered, waiting");
  await act(async () => { f.play(story.length); });
  await waitFor(() => expect(within(card()).queryByRole("status")).toBeNull());
  return card();
}

describe("S30-1 AC3 a pending answer", () => {
  it("ac3_a_short_decision_ends_answered_waiting_like_a_full_one", async () => {
    const c = await answered(shortDecided(10, 8, "a8"));
    expect(c).not.toHaveAttribute("data-asking");
    expect(within(c).queryByRole("button", { name: /^(Allow once|Deny)/ })).toBeNull();
    expect(screen.queryByRole("button", { name: /Needs you/ })).toBeNull();
    expect(c.querySelector("[data-diff]")).not.toBeNull(); // the card keeps the ask's call
    expect(document.body.textContent).toMatch(/alice · Allowed: Edit src\/sync\.ts/);
  });
  it("ac3_the_full_decision_gives_the_same_card_state", async () => {
    const full = await answered(decided(10, "a8", "e2"));
    expect(full).not.toHaveAttribute("data-asking");
    expect(document.body.textContent).toMatch(/alice · Allowed: Edit src\/e2\.ts/);
  });
});
