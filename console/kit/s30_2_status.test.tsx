// S30-2 AC5: a turn `waiting` with no ask to answer (a check) reads as before; only an answered ask changes the line.
import { afterEach, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { emptyView } from "./model";
import { StatusLine } from "./status";

afterEach(cleanup);

it("ac5_a_waiting_turn_with_only_a_check_is_unchanged", () => {
  const view = { ...emptyView, turn: { state: "waiting" as const, asks: [], expiresAt: 61_000 }, openAsks: [] };
  render(<StatusLine view={view} now={1_000} stopping={false} actions={{} as never} onNeedsYou={() => {}} answered={0} />);
  expect(screen.getByRole("status").textContent).toBe("Waiting for you");
  expect(screen.getByText("· denied in 1:00 if unanswered")).toBeInTheDocument();
});
