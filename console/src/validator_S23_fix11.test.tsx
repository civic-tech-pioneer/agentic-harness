// S23-fix11 validator: body counts as "in the card" only when what held focus there went away; a participant who left focus on body keeps it.
import { describe, expect, it } from "vitest";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { App } from "./App";
import { storyInfo as INFO } from "../fake/page";

async function go(leave: "none" | "blur" | "bodyclick") {
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
  expect(within(card).getByRole("textbox")).toHaveFocus();
  await u.keyboard("x{Enter}");
  expect(within(card).getByRole("status")).toHaveFocus();
  if (leave === "blur") (document.activeElement as HTMLElement).blur();
  if (leave === "bodyclick") await u.click(document.body);
  await act(async () => { refuse(); await new Promise((r) => setTimeout(r, 0)); });
  const al = await within(card).findByRole("alert");
  return { al, card };
}
describe("validator S23 fix11", () => {
  it("refused_focus_in_the_line_moves_to_the_refused_line", async () => { const { al } = await go("none"); expect(al).toHaveFocus(); });
  it("refused_after_the_participant_blurred_to_body_leaves_body", async () => { const { al } = await go("blur"); expect(al).not.toHaveFocus(); expect(document.activeElement).toBe(document.body); });
  it("refused_after_a_click_on_the_page_leaves_focus_alone", async () => { const { al } = await go("bodyclick"); expect(al).not.toHaveFocus(); });
});
