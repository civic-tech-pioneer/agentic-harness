// S23-fix6 AC2 (validator): a view from the old session's openSession that lands after its close never shows under the new id,
// and never takes the place of the new session's view.
import { describe, expect, it, vi } from "vitest";
import { act, render, screen } from "@testing-library/react";
import type { Kernel, SessionView } from "../kit";
import { storyInfo as INFO } from "../fake/page";

const views = new Map<string, (v: SessionView) => void>(); // each session's onView, kept past its close
vi.mock("../kit", async (orig) => {
  const k = await orig<typeof import("../kit")>();
  const R = await import("react");
  return { ...k,
    openSession: (_k: Kernel, id: string, onView: (v: SessionView) => void) => { views.set(id, onView); return { close() {} }; },
    Transcript: (p: { view: SessionView }) => R.createElement("div", { "data-testid": "mark" }, String(p.view.lastIndex)) };
});
const { App } = await import("./App");
const { emptyView } = await import("../kit");

describe("S23-fix6 late view", () => {
  it("ac2_late_view_of_closed_session_never_shows_under_the_new_id", async () => {
    const { rerender } = render(<App kernel={{} as Kernel} info={INFO} />);
    act(() => views.get(INFO.id)!({ ...emptyView, lastIndex: 11 }));
    expect(screen.getByTestId("mark").textContent).toBe("11");
    rerender(<App kernel={{} as Kernel} info={{ ...INFO, id: "s2" }} />);
    expect(screen.getByTestId("mark").textContent, "the old view leaves at once").toBe("0");
    act(() => views.get("s2")!({ ...emptyView, lastIndex: 22 }));
    act(() => views.get(INFO.id)!({ ...emptyView, lastIndex: 12 })); // late: after its close
    expect(screen.getByTestId("mark").textContent).toBe("22");
  });
});

describe("S23-fix6 session switch state", () => {
  it("ac2_new_session_does_not_inherit_stopping", async () => {
    const { fireEvent } = await import("@testing-library/react");
    const running = { ...emptyView, turn: { state: "running" as const, since: 0 } };
    const k = { interrupt: () => Promise.resolve() } as unknown as Kernel;
    const { rerender } = render(<App kernel={k} info={INFO} />);
    act(() => views.get(INFO.id)!(running));
    fireEvent.click(screen.getByRole("button", { name: /Stop/ }));
    expect(screen.getByRole("button", { name: /Stop/ }).textContent).toContain("Stopping");
    rerender(<App kernel={k} info={{ ...INFO, id: "s3" }} />);
    act(() => views.get("s3")!(running));
    expect(screen.getByRole("button", { name: /Stop/ }).textContent).not.toContain("Stopping");
  });
});
