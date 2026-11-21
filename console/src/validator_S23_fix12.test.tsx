// S23-fix12 validator: pins what the regress test leaves open. Moves between the list's own children (mouseover/mouseout
// bubbling with relatedTarget inside the nav) keep the pointer "over"; a hold set before the leave ends with the leave; the
// native listeners go with the nav on unmount.
import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, render } from "@testing-library/react";
import { flushSync } from "react-dom";
import { storyInfo } from "../fake/page";
import { Sidebar } from "./Sidebar";

afterEach(() => { cleanup(); vi.restoreAllMocks(); });
const group = () => [...document.querySelectorAll(".wa-group")].find((g) => g.querySelector("[aria-current]"))!.querySelector(".wa-glabel")!.textContent;
const move = (from: Element, to: Element) => {
  from.dispatchEvent(new MouseEvent("mouseout", { bubbles: true, relatedTarget: to }));
  to.dispatchEvent(new MouseEvent("mouseover", { bubbles: true, relatedTarget: from }));
};

describe("validator_S23_fix12", () => {
  it("a_move_between_children_of_the_list_keeps_the_hold", () => {
    const { rerender } = render(<Sidebar info={storyInfo} sessions={[]} />);
    const nav = document.querySelector("nav")!;
    const row = nav.querySelector("[aria-current]")!;
    const brand = nav.querySelector(".wa-brand")!;
    act(() => { move(document.body, row); });
    act(() => { move(row, brand); });
    rerender(<Sidebar info={storyInfo} turn="waiting" needs sessions={[]} />);
    rerender(<Sidebar info={storyInfo} turn="waiting" needs={false} sessions={[]} />);
    expect(group()).toBe("Needs you");
  });
  it("a_hold_set_before_the_leave_ends_with_the_leave", async () => {
    const { rerender } = render(<Sidebar info={storyInfo} turn="waiting" needs sessions={[]} />);
    const nav = document.querySelector("nav")!;
    act(() => { move(document.body, nav); });
    await act(async () => {
      move(nav, document.body);
      flushSync(() => rerender(<Sidebar info={storyInfo} turn="waiting" needs={false} sessions={[]} />));
    });
    expect(group()).toBe("Running");
  });
  it("the_native_listeners_are_removed_on_unmount", () => {
    const add = vi.spyOn(EventTarget.prototype, "addEventListener");
    const remove = vi.spyOn(EventTarget.prototype, "removeEventListener");
    const { unmount } = render(<Sidebar info={storyInfo} sessions={[]} />);
    const nav = document.querySelector("nav")!;
    const on = (m: typeof add) => m.mock.calls.filter((_, i) => m.mock.contexts[i] === nav).map((c) => [c[0], c[1]]);
    const added = on(add);
    expect(added.map((c) => c[0]).sort()).toEqual(["mouseout", "mouseover"]);
    unmount();
    expect(on(remove)).toEqual(expect.arrayContaining(added));
  });
});
