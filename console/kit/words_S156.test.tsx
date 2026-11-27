// S156-1: the words after Stop (S154 r1 P1.2) and "Thinking" (S144 deputy compare), against Main-2 and States-2.
// TECHNICAL 18.3 (decision line, footer), 18.5 (status words).
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { emptyView, type Row, type SessionView } from "./model";
import { reduce } from "./reduce";
import { RowView } from "./parts/rows";
import { StatusLine } from "./status";

afterEach(cleanup);
const J = JSON.stringify;
const entry = (i: number, kind: string, body: object) => J({ v: 1, i, kind, at: i, body });
const w = (m: object) => J({ _meta: { watane: m } });
const ask = (i: number, id: string, file: string) => entry(i, "ask", { id, kind: "ask", toolCallId: `t-${id}`, input: { file_path: file }, "expires-at": 99,
  toolCall: { toolCallId: `t-${id}`, title: `Edit ${file}`, kind: "edit", status: "pending", locations: [{ path: file }] } });
const short = (i: number, askI: number, id: string, d: object) => entry(i, "ask", { id, kind: "ask", "ask-i": askI, decision: d });
const running = w({ type: "turn", state: { state: "running", since: 1 } });
const interrupted = (i: number) => entry(i, "turn", { state: "idle", stop: { kind: "interrupted" }, seconds: 3 });
const run = (...ms: string[]) => ms.reduce(reduce, emptyView);
const rows = <K extends Row["kind"]>(v: SessionView, kind: K) => v.rows.filter((r): r is Extract<Row, { kind: K }> => r.kind === kind);
const show = (v: SessionView) => render(<>{v.rows.map((r) => <RowView key={r.key} row={r} actions={{} as never} />)}</>);

/** Stop during a launch: two asks the kernel closed `cancel`, one she allowed that Claude never got. */
const cancelledLaunch = () => run(entry(1, "message", { role: "user", content: [{ type: "text", text: "Merge" }] }), running,
  ask(2, "a1", "reading-list.md"), ask(3, "a2", "books.md"), ask(4, "a3", "to-read.md"),
  short(5, 4, "a3", { outcome: "allow", by: "alice" }),
  short(6, 2, "a1", { outcome: "deny", by: "kernel", reason: "cancel" }),
  short(7, 3, "a2", { outcome: "deny", by: "kernel", reason: "cancel" }),
  interrupted(8));

describe("S156-1 words after Stop", () => {
  it("ac1_the_footer_counts_no_ask_that_never_ran", () => {
    const v = cancelledLaunch();
    expect(rows(v, "footer")[0]).toMatchObject({ tools: 0, interrupted: true });
    show(v);
    expect(screen.getByText("Stopped after 3 s · 0 tools")).toBeInTheDocument();
  });

  it("ac1_a_call_that_ran_still_counts", () => {
    const v = run(running, w({ type: "x" }), J({ sessionUpdate: "tool_call", toolCallId: "t9", title: "Read a.md", kind: "other", status: "completed" }), interrupted(2));
    expect(rows(v, "footer")[0]!.tools).toBe(1);
  });

  it("ac2_a_kernel_cancel_reads_ask_cancelled_the_turn_was_stopped", () => {
    show(cancelledLaunch());
    const lines = [...document.querySelectorAll(".wa-decision")].map((e) => e.textContent);
    expect(lines).toContain("Ask cancelled · Edit reading-list.md — the turn was stopped");
    expect(lines.join("\n")).not.toMatch(/kernel|— cancel/);
    const card = document.querySelector('[data-ask="a1"] .wa-tool-status')!;
    expect(card.textContent).toBe("Cancelled");
  });

  it.each([["restart", "watane restarted"], ["ended", "the connection to Claude Code ended"], ["busy", "too many open asks"]])(
    "ac2_kernel_reason_%s_in_words", (reason, words) => {
      show(run(running, ask(2, "a1", "x.md"), short(3, 2, "a1", { outcome: "deny", by: "kernel", reason })));
      expect(document.querySelector(".wa-decision")!.textContent).toBe(`Ask cancelled · Edit x.md — ${words}`);
    });

  it("ac2_another_kernel_reason_and_her_own_deny_read_as_before", () => {
    show(run(running, ask(2, "a1", "x.md"), short(3, 2, "a1", { outcome: "deny", by: "kernel", reason: "odd" }),
      ask(4, "a2", "y.md"), short(5, 4, "a2", { outcome: "deny", by: "alice", reason: "cancel" })));
    const lines = [...document.querySelectorAll(".wa-decision")].map((e) => e.textContent);
    expect(lines).toEqual(["kernel · Denied: Edit x.md — odd", "alice · Denied: Edit y.md — cancel"]);
  });

  it("ac3_an_allowed_call_with_no_result_reads_no_result_after_the_turn", () => {
    const before = run(running, ask(2, "a3", "to-read.md"), short(3, 2, "a3", { outcome: "allow", by: "alice" }));
    show(before);
    expect(document.querySelector('[data-ask="a3"] .wa-tool-status')!.textContent).toBe("Allowed");
    cleanup();
    show(cancelledLaunch());
    expect(document.querySelector('[data-ask="a3"] .wa-tool-status')!.textContent).toBe("No result");
  });
});

/** OpenAI's cleanup of 9.5: after the kernel denies the open ask, the call gets the `aborted` output, a `tool` entry with status `failed`. */
const aborted = (i: number, id: string, file: string) => entry(i, "tool", { toolCallId: `t-${id}`, title: "edit", rawInput: { file_path: file },
  status: "failed", content: [{ type: "content", content: { type: "text", text: "aborted" } }] });
const badge = (id: string) => document.querySelector(`[data-ask="${id}"] .wa-tool-status`)!.textContent;

describe("S156 r1 F1.1 a kernel cleanup result after a cancelled ask", () => {
  it.each(["cancel", "restart"])("f1_1_%s_then_failed_abort_keeps_cancelled_and_counts_0_tools", (reason) => {
    const v = run(entry(1, "message", { role: "user", content: [{ type: "text", text: "Merge" }] }), running,
      ask(2, "a1", "reading-list.md"), short(3, 2, "a1", { outcome: "deny", by: "kernel", reason }),
      aborted(4, "a1", "reading-list.md"), interrupted(5));
    expect(rows(v, "footer")[0]).toMatchObject({ tools: 0, interrupted: true });
    show(v);
    expect(badge("a1")).toBe("Cancelled");
    expect(screen.getByText("Stopped after 3 s · 0 tools")).toBeInTheDocument();
  });

  it("f1_1_her_own_deny_then_a_failed_result_still_reads_failed", () => {
    show(run(running, ask(2, "a1", "x.md"), short(3, 2, "a1", { outcome: "deny", by: "alice" }), aborted(4, "a1", "x.md")));
    expect(badge("a1")).toBe("Failed");
  });
});

describe("S156 r1 F1.2 Claude's result after Stop", () => {
  const done = J({ sessionUpdate: "tool_call_update", toolCallId: "t-a1", status: "completed" });
  const stopped = () => run(entry(1, "message", { role: "user", content: [{ type: "text", text: "Edit" }] }), running,
    ask(2, "a1", "notes.md"), short(3, 2, "a1", { outcome: "allow", by: "alice" }),
    J({ sessionUpdate: "tool_call_update", toolCallId: "t-a1", status: "pending" }), interrupted(4));

  it("f1_2_allow_tool_use_stop_reads_no_result_and_0_tools", () => {
    const v = stopped();
    expect(rows(v, "footer")[0]!.tools).toBe(0);
    show(v);
    expect(badge("a1")).toBe("No result");
    expect(document.body.textContent).not.toMatch(/Did not run/);
  });

  it("f1_2_the_late_completed_result_updates_the_card_and_the_footer", () => {
    const v = reduce(stopped(), done);
    expect(rows(v, "footer")[0]).toMatchObject({ tools: 1, interrupted: true });
    show(v);
    expect(badge("a1")).toBe("Done");
    expect(screen.getByText("Stopped after 3 s · 1 tool")).toBeInTheDocument();
  });
});

describe("S156-1 Thinking", () => {
  const status = (v: SessionView) => { render(<StatusLine view={v} now={2_000} stopping={false} actions={{} as never} onNeedsYou={() => {}} answered={0} />); return screen.getByRole("status").textContent; };
  const thought = J({ sessionUpdate: "agent_thought_chunk", content: { type: "text", text: "hm" } });
  it("ac4_a_streaming_thought_says_thinking", () => expect(status(run(running, thought))).toBe("Thinking"));
  it("ac4_no_row_yet_says_working", () => expect(status(run(entry(1, "message", { role: "user", content: [{ type: "text", text: "hi" }] }), running))).toBe("Working"));
});
