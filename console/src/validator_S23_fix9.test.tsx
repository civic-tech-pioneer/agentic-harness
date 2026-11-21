// S23-fix9 validator: pins what the regress tests leave open. AC2: a `waiting` turn with no open ask goes under
// "Running" with the run glyph (not "Recent"); with `needs` unset the turn decides. AC3: the first frame says "Loading".
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/react";
import { longInfo, longKernel, storyInfo } from "../fake/page";
import { App } from "./App";
import { Sidebar } from "./Sidebar";

afterEach(cleanup);
const cur = () => document.querySelector("[aria-current]")!;
const group = () => cur().closest(".wa-group")!.querySelector(".wa-glabel")!.textContent;

describe("validator_S23_fix9", () => {
  it("ac2_waiting_with_no_open_ask_is_running", () => {
    render(<Sidebar info={storyInfo} turn="waiting" needs={false} sessions={[]} />);
    expect(group()).toBe("Running");
    expect(cur().querySelector(".glyph")!.classList.contains("g-run")).toBe(true);
  });
  it("ac2_waiting_with_needs_unset_follows_the_turn", () => {
    render(<Sidebar info={storyInfo} turn="waiting" sessions={[]} />);
    expect(group()).toBe("Needs you");
  });
  it("ac3_first_frame_says_loading", () => {
    render(<App kernel={longKernel(1200)} info={longInfo(1200)} />);
    expect(document.querySelector(".wa-status-word")!.textContent).toBe("Loading");
  });
});
