// S127-4: the console shows the checked provider's verdicts, its breaker and a turn the judge failed (TECHNICAL 9.4, 10, 18.3, 18.5).
import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { emptyView, type Actions, type SessionView } from "./model";
import { reduce } from "./reduce";
import { StatusLine } from "./status";

const J = JSON.stringify;
const meta = (m: object) => J({ _meta: { watane: m } });
const entry = (i: number, kind: string, body: object) => meta({ type: "entry", entry: { v: 1, i, kind, at: i, body } });
const feed = (msgs: string[], v: SessionView = emptyView) => msgs.reduce(reduce, v);
const mk = (): Actions => ({ say: vi.fn(), answer: vi.fn(), stop: vi.fn(), takeBack: vi.fn() });
const word = (turn: object) => {
  const view = { ...emptyView, turn: turn as SessionView["turn"] };
  render(<StatusLine view={view} now={1000} stopping={false} actions={mk()} onNeedsYou={() => {}} />);
  return screen.getByRole("status").textContent;
};

describe("S127-4 verdicts", () => {
  it("ac1_the_breaker_verdict_is_a_decision_line_with_its_rule", () => {
    const v = feed([entry(1, "verdict", { on: "breaker", outcome: "unsafe", reasons: ["3 denials in a row: the session was stopped"], by: "local/claude-code-checked", "denials-in-a-row": 3 })]);
    expect(v.rows[0]).toMatchObject({ kind: "decision", who: "local/claude-code-checked", what: "Stopped by your judge's breaker", reason: "3 denials in a row: the session was stopped" });
  });
  it("ac2_a_verdict_on_the_input_names_the_input_and_each_score", () => {
    const v = feed([entry(1, "verdict", { on: "input", outcome: "unsure", reasons: ["unsafe: 0.50", "off_task: 0.10"], by: "checked" })]);
    expect(v.rows[0]).toMatchObject({ kind: "decision", what: "unsure: the input", reason: "unsafe: 0.50; off_task: 0.10" });
  });
  it("ac2_a_breaker_verdict_with_another_on_value_is_unknown", () => {
    expect(feed([entry(1, "verdict", { on: "other", outcome: "unsafe", by: "checked" })]).rows[0]).toMatchObject({ kind: "unknown" });
  });
});

describe("S127-4 status line", () => {
  it("ac3_a_turn_the_judge_failed_says_so", () => {
    expect(word({ state: "idle", stop: { kind: "failed", error: "judge" } })).toBe("Failed: your judge denied it");
  });
  it("ac4_running_with_checking_reads_checking_with_your_judge", () => {
    expect(word({ state: "running", since: 0, activity: { kind: "checking", on: "input" } })).toBe("Checking with your judge");
  });
});
