// S23 outcome 1 / S23-1 AC3 (acceptance): once Alice answered the only open ask, the answered ask is not the status line's
// subject. With `openAsks` empty, `askSubject` looks for `r.ask?.id === undefined` and so names the first tool row with no
// ask at all: the page read "Waiting for you · Edit src/retry.ts", a call that finished long ago and never asked anything.
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { openSession } from "./kernel";
import type { SessionView } from "./model";
import { StatusLine } from "./status";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";

afterEach(cleanup);

describe("regress_S23", () => {
  it("regress_S23_status_subject_after_answer", async () => {
    const k = fakeKernel(story);
    for (let n = 0; n < story.length; n++) k.play(n);
    let view: SessionView | undefined;
    const s = openSession(k, "s1", (v) => { view = v; });
    await vi.waitFor(() => expect(view?.turn?.state === "waiting" && view.openAsks.length === 1).toBe(true));
    s.close();
    // what App shows once this console answered a8 and no decision entry has come yet: the ask is not open any more
    render(<StatusLine view={{ ...view!, openAsks: [] }} now={Date.now()} stopping={false} actions={{} as never} onNeedsYou={() => {}} />);
    expect(screen.getByRole("status").textContent).not.toContain("retry.ts");
  });
});
