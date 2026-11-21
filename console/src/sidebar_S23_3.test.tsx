// S23-3 AC3: the open session sits under "Needs you" whenever an ask is open, whatever the turn says.
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { storyInfo } from "../fake/page";
import { App } from "./App";
import { Sidebar } from "./Sidebar";

afterEach(cleanup);
const group = () => [...document.querySelectorAll(".wa-group")].find((g) => g.querySelector("[aria-current]"))!.querySelector(".wa-glabel")!.textContent;

describe("S23-3 sidebar", () => {
  it("ac3_a_running_turn_with_an_open_ask_is_under_Needs_you", () => {
    render(<Sidebar info={storyInfo} turn="running" needs sessions={[]} />);
    expect(group()).toBe("Needs you");
    expect(document.querySelector("[aria-current] .g-ask")).not.toBeNull();
  });
  it("ac3_without_needs_it_is_as_before", () => {
    render(<Sidebar info={storyInfo} turn="running" sessions={[]} />);
    expect(group()).toBe("Running");
  });
  it("ac3_app_puts_a_running_turn_with_an_open_ask_under_Needs_you", async () => {
    const running = JSON.stringify({ _meta: { watane: { type: "turn", state: { state: "running", since: 0 } } } });
    const f = fakeKernel([...story, [running]]);
    for (let n = 0; n <= story.length; n++) f.play(n);
    render(<App kernel={f} info={storyInfo} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    await waitFor(() => expect(within(screen.getByRole("button", { current: true })).getByRole("img", { name: "Needs you" })).toBeInTheDocument());
    expect(group()).toBe("Needs you");
  });
  it("ac3_the_hold_under_the_pointer_still_holds", () => {
    const { rerender } = render(<Sidebar info={storyInfo} turn="running" needs sessions={[]} />);
    fireEvent.mouseEnter(screen.getByRole("navigation"));
    rerender(<Sidebar info={storyInfo} turn="running" needs={false} sessions={[]} />);
    expect(group()).toBe("Needs you");
  });
});
