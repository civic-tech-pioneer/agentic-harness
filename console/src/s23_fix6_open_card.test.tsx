// S23-fix6 AC3: Needs you and its measure find the open card of an id, never a decided card that keeps the same data-ask.
import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { storyInfo as INFO } from "../fake/page";

vi.mock("../kit", async (orig) => {
  const k = await orig<typeof import("../kit")>();
  const R = await import("react");
  return { ...k, Transcript: (p: { view: { openAsks: string[] } }) => R.createElement("div", null,
    R.createElement("div", { "data-ask": p.view.openAsks[0], id: "decided" }, R.createElement("button", { "data-ask-first": "" }, "Allow once")),
    R.createElement("div", { "data-ask": p.view.openAsks[0], "data-asking": "", id: "open" }, R.createElement("button", { "data-ask-first": "" }, "Allow once"))) };
});
const { App } = await import("./App");

describe("S23-fix6 open card", () => {
  it("ac3_needs_you_focuses_the_open_card_not_a_decided_one_with_the_same_id", async () => {
    const f = fakeKernel([...story]);
    for (let n = 0; n < story.length; n++) f.play(n);
    render(<App kernel={f} info={INFO} />);
    fireEvent.click(await screen.findByRole("button", { name: "Needs you (1)" }));
    expect(document.activeElement?.closest("[data-ask]")?.id).toBe("open");
  });
});
