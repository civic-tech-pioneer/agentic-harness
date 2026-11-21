// S23 outcome 6 (acceptance): a session paging in its history shows "Loading" for its turn, never "Idle". On the page, the long
// session's status line reads "Idle" from the first frame until its attach answers `behind` (about 30 ms here, 145 ms at 6x CPU
// throttling), then "Loading": App starts from `emptyView` (no turn, loading false), which StatusLine words as "Idle".
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/react";
import { longInfo, longKernel } from "../fake/page";
import { App } from "./App";

afterEach(cleanup);

describe("regress_S23", () => {
  it("regress_S23_idle_on_open", () => {
    render(<App kernel={longKernel(1200)} info={longInfo(1200)} />);
    expect(document.querySelector(".wa-status-word")!.textContent).not.toBe("Idle");
  });
});
