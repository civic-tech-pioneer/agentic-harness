// S23 outcome 6 (acceptance): the sidebar's group for the open session agrees with the dock's Needs you. After Alice answered
// the only open ask, the dock shows no "Needs you" (App passes needs={false}), but the turn is still `waiting` until the
// decision entry arrives, and `shapeOf("waiting")` keeps the open session under "Needs you" with the ask glyph: the two disagree.
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/react";
import { storyInfo } from "../fake/page";
import { Sidebar } from "./Sidebar";

afterEach(cleanup);
const group = () => [...document.querySelectorAll(".wa-group")].find((g) => g.querySelector("[aria-current]"))!.querySelector(".wa-glabel")!.textContent;

describe("regress_S23", () => {
  it("regress_S23_sidebar_after_answer", () => {
    render(<Sidebar info={storyInfo} turn="waiting" needs={false} sessions={[]} />);
    expect(group()).not.toBe("Needs you");
  });
});
