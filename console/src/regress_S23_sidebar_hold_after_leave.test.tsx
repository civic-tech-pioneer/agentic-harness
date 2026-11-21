// S23 outcome 6 (acceptance run 2): the sidebar's group for the open session agrees with the dock's Needs you. On a touch screen
// (or any pointer that leaves the list and presses Allow once in the same frame), the list's mouseleave (a continuous-priority
// event in React) is still pending when the answer's sync render runs: the hold effect sees the pointer still over the list and
// holds the row under "Running" (the answered ask is out of `needs`). The pointer has left, so nothing clears that hold: after a
// `busy` refusal the dock says "Needs you (1)", the row keeps the ask glyph, and its group stays "Running" until the next press.
import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, render } from "@testing-library/react";
import { flushSync } from "react-dom";
import { storyInfo } from "../fake/page";
import { Sidebar } from "./Sidebar";

afterEach(cleanup);
const group = () => [...document.querySelectorAll(".wa-group")].find((g) => g.querySelector("[aria-current]"))!.querySelector(".wa-glabel")!.textContent;

describe("regress_S23", () => {
  it("regress_S23_sidebar_hold_after_leave", async () => {
    const { rerender } = render(<Sidebar info={storyInfo} turn="waiting" needs sessions={[]} />);
    const nav = document.querySelector("nav")!;
    // the pointer was over the list (a tap on a row)
    act(() => { nav.dispatchEvent(new MouseEvent("mouseover", { bubbles: true, relatedTarget: document.body })); });
    expect(group()).toBe("Needs you");
    // the next tap, on Allow once: the pointer leaves the list, then the answer renders synchronously in the same task
    await act(async () => {
      document.body.dispatchEvent(new MouseEvent("mouseover", { bubbles: true, relatedTarget: nav }));
      nav.dispatchEvent(new MouseEvent("mouseout", { bubbles: true, relatedTarget: document.body }));
      flushSync(() => rerender(<Sidebar info={storyInfo} turn="waiting" needs={false} sessions={[]} />));
    });
    // the kernel refuses the answer (busy): the ask is open again and the dock shows Needs you (1)
    await act(async () => { rerender(<Sidebar info={storyInfo} turn="waiting" needs sessions={[]} />); });
    expect(group()).toBe("Needs you");
  });
});
