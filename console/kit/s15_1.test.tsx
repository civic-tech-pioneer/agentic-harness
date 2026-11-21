// S15-1: the console reads the S10 wire (TECHNICAL 2.3, 9.3, 9.4) and draws an ask from the ask entry only (9.8).
import { describe, expect, it, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { emptyView, type Actions, type SessionView } from "./model";
import { reduce } from "./reduce";
import { ThreadProvider, Transcript } from "./thread";
import { StatusLine } from "./status";
import { story } from "../fake/story";

const J = JSON.stringify;
const meta = (m: object) => J({ _meta: { watane: m } });
const entry = (i: number, kind: string, body: object) => meta({ type: "entry", entry: { v: 1, i, kind, at: i, body } });
const upd = (u: object, m?: object) => J({ ...u, ...(m ? { _meta: { watane: m } } : {}) });
const feed = (msgs: string[], v: SessionView = emptyView) => msgs.reduce(reduce, v);
const call = (id: string, kind: string, extra: object = {}) => upd({ sessionUpdate: "tool_call", toolCallId: id, title: `Streamed ${id}`, kind, status: "pending", ...extra });
const patch = (id: string, extra: object) => upd({ sessionUpdate: "tool_call_update", toolCallId: id, ...extra });
const diff = (p: string, n: string) => [{ type: "diff", path: p, oldText: "", newText: n }];
const askBody = (id: string, tc: string, extra: object = {}) => ({ id, kind: "ask", toolCallId: tc, input: {}, "expires-at": 9, ...extra });
const ask = (i: number, id: string, tc: string, extra: object = {}) => entry(i, "ask", askBody(id, tc, extra));
const decide = (i: number, id: string, tc: string, decision: object, extra: object = {}) => entry(i, "ask", askBody(id, tc, { decision, ...extra }));
const mk = (): Actions => ({ say: vi.fn(), answer: vi.fn(), stop: vi.fn(), takeBack: vi.fn() });
const show = (view: SessionView, actions = mk()) => render(<ThreadProvider view={view} actions={actions}><Transcript view={view} actions={actions} /></ThreadProvider>);
const tool = (v: SessionView, k = 0) => v.rows[k] as Extract<SessionView["rows"][number], { kind: "tool" }>;

describe("S15-1 wire", () => {
  it("ac1_an_ask_body_needs_id_kind_toolcallid_and_expires_at", () => {
    const bad = [
      { kind: "ask", toolCallId: "t", "expires-at": 1 },
      { id: "a", toolCallId: "t", "expires-at": 1 },
      { id: "a", kind: "other", toolCallId: "t", "expires-at": 1 },
      { id: "a", kind: "ask", "expires-at": 1 },
      { id: "a", kind: "ask", toolCallId: "t", expiresAt: 1 },
      { id: "a", kind: "ask", toolCallId: "t", "expires-at": "1" },
    ];
    for (const b of bad) {
      const v = feed([entry(3, "ask", { input: {}, ...b })]);
      expect(v.rows).toMatchObject([{ kind: "unknown", i: 3 }]);
      expect(v.openAsks).toEqual([]);
    }
  });
  it("ac1_reach_is_ignored_on_a_check", () => {
    const v = feed([ask(1, "c1", "t", { kind: "check", reach: "Edit(**)" })]);
    expect(tool(v).ask).not.toHaveProperty("reach");
    expect(tool(feed([ask(1, "a1", "t", { reach: "Edit(**)" })])).ask).toMatchObject({ reach: "Edit(**)" });
  });
  it("ac2_waiting_reads_asks_and_expires_at_and_paused_may_lack_until", () => {
    expect(feed([meta({ type: "turn", state: { state: "waiting", asks: ["a1"], "expires-at": 5 } })]).turn).toEqual({ state: "waiting", asks: ["a1"], expiresAt: 5 });
    expect(feed([meta({ type: "turn", state: { state: "waiting", asks: ["a1"], expiresAt: 5 } })]).rows[0]).toMatchObject({ kind: "unknown" });
    expect(feed([meta({ type: "turn", state: { state: "paused" } })]).turn).toEqual({ state: "paused" });
    expect(feed([entry(1, "turn", { state: "paused" })]).turn).toEqual({ state: "paused" });
    expect(feed([entry(1, "turn", { state: "paused", until: 7 })]).turn).toEqual({ state: "paused", until: 7 });
    expect(feed([entry(1, "turn", { state: "waiting", asks: ["x"], "expires-at": 8 })]).turn).toMatchObject({ state: "waiting", expiresAt: 8 });
  });
  it("ac2_status_line_reads_paused_with_no_time", () => {
    const view = { ...emptyView, turn: { state: "paused" } as SessionView["turn"] };
    render(<StatusLine view={view} now={0} stopping={false} actions={mk()} onNeedsYou={() => {}} />);
    expect(screen.getByRole("status").textContent).toBe("Paused");
    expect(screen.getByRole("img", { name: "Paused" })).toHaveAttribute("data-shape", "bars");
    expect(screen.getByRole("button", { name: "Resume now" })).toBeInTheDocument();
  });
  it("ac8_verdict_on_the_input_is_a_decision_line", () => {
    const v = feed([entry(1, "verdict", { on: "input", outcome: "deny", reasons: ["too long"], by: "judge" })]);
    expect(v.rows[0]).toMatchObject({ kind: "decision", who: "judge", what: "deny: the input", reason: "too long" });
    expect(feed([entry(1, "verdict", { outcome: "deny", by: "judge" })]).rows[0]).toMatchObject({ kind: "unknown" });
    expect(feed([entry(1, "verdict", { on: "input", by: "judge" })]).rows[0]).toMatchObject({ kind: "unknown" });
    const w = feed([call("t", "edit"), entry(2, "verdict", { toolCallId: "t", outcome: "allow", by: "judge" })]);
    expect(w.rows[1]).toMatchObject({ what: "allow: Streamed t" });
  });
});

describe("S15-1 asks and decisions", () => {
  it("ac3_open_asks_are_ask_ids_in_arrival_order_and_not_checks", () => {
    const v = feed([ask(1, "a2", "t2"), ask(2, "c1", "t1", { kind: "check" }), ask(3, "a1", "t1")]);
    expect(v.openAsks).toEqual(["a2", "a1"]);
  });
  it("ac3_the_card_carries_the_ask_id_and_the_buttons_answer_it", async () => {
    const actions = mk();
    const v = feed([call("t", "execute"), ask(1, "a9", "t", { input: { command: "ls" }, reach: "Bash(ls)", toolCall: { toolCallId: "t", title: "ls", kind: "execute" } })]);
    const { container } = show(v, actions);
    expect(container.querySelector("[data-ask='a9']")).not.toBeNull();
    await userEvent.setup().click(screen.getByRole("button", { name: "Allow once" }));
    expect(actions.answer).toHaveBeenCalledWith("a9", { outcome: "allow" });
  });
  it("ac4_a_decision_clears_the_ask_of_its_own_id_only", () => {
    let v = feed([call("t", "edit"), ask(1, "c1", "t", { kind: "check" }), ask(2, "a1", "t")]);
    expect(v.openAsks).toEqual(["a1"]);
    v = feed([decide(3, "c1", "t", { outcome: "none", by: "checked" }, { kind: "check" })], v);
    expect(v.openAsks).toEqual(["a1"]);
    expect(tool(v).ask).toMatchObject({ id: "a1" });
    expect(tool(v).ask).not.toHaveProperty("decision");
    show(v);
    expect(screen.getByRole("button", { name: "Allow once" })).toBeInTheDocument();
    expect(v.rows[1]).toMatchObject({ kind: "decision", who: "checked", what: "No objection: t" });
  });
  it("ac4_two_asks_on_one_call_are_answered_one_at_a_time", () => {
    let v = feed([ask(1, "a1", "t"), ask(2, "a2", "t")]);
    expect(v.openAsks).toEqual(["a1", "a2"]);
    v = feed([decide(3, "a1", "t", { outcome: "allow", by: "owner" })], v);
    expect(v.openAsks).toEqual(["a2"]);
    expect(tool(v).ask).toMatchObject({ id: "a2" });
  });
  it("ac4_any_by_is_text_and_an_unknown_outcome_is_an_unknown_row", () => {
    const v = feed([ask(1, "a1", "t", { toolCall: { toolCallId: "t", title: "Edit x", kind: "edit" } }), decide(2, "a1", "t", { outcome: "deny", by: "someone-new", reason: "no" })]);
    expect(v.rows[1]).toMatchObject({ kind: "decision", who: "someone-new", what: "Denied: Edit x", reason: "no" });
    const bad = feed([decide(1, "a1", "t", { outcome: "maybe", by: "owner" })]);
    expect(bad.rows).toMatchObject([{ kind: "unknown", i: 1 }]);
  });
  it("ac4_the_decision_line_names_the_call_by_the_ask_not_the_stream", () => {
    const v = feed([call("t", "execute"), ask(1, "a1", "t", { input: { command: "rm -rf x" } }), decide(2, "a1", "t", { outcome: "allow", by: "owner" }, { input: { command: "rm -rf x" } })]);
    expect(v.rows[1]).toMatchObject({ what: "Allowed: rm -rf x" });
  });
});

describe("S15-1 the card is the ask's", () => {
  const streamed = [call("t", "edit", { locations: [{ path: "/streamed" }], rawInput: { file_path: "/streamed" }, content: diff("/streamed", "STREAMED") }),
    patch("t", { title: "Edit /streamed2", locations: [{ path: "/streamed2" }], content: diff("/streamed2", "LATE") })];
  it("ac5_an_ask_without_a_toolcall_shows_no_streamed_title_path_or_diff", () => {
    const v = feed([...streamed, ask(1, "a1", "t", { input: { command: "ls" } }), patch("t", { title: "Edit /after", kind: "edit", locations: [{ path: "/after" }], content: diff("/after", "AFTER"), rawInput: { x: 1 } })]);
    expect(tool(v).call).toEqual({ id: "t", title: "other", kind: "other", status: "pending", content: [], locations: [] });
    const { container } = show(v);
    expect(container.textContent).not.toMatch(/streamed|STREAMED|LATE|after|AFTER/i);
    expect(container.querySelector("code")!.textContent).toBe("ls");
  });
  it("ac5_the_argument_line_is_the_input_not_the_title_or_raw_input", () => {
    const v = feed([ask(1, "a1", "t", { input: { url: "https://a.example", pattern: "p" }, toolCall: { toolCallId: "t", title: "Fetch https://evil.example", kind: "fetch", rawInput: { path: "/evil" } } })]);
    const { container } = show(v);
    expect(container.querySelector("code")!.textContent).toBe("https://a.example");
    expect(container.querySelector(".wa-tool-head")!.textContent).not.toContain("evil");
  });
  it("ac5_after_the_decision_the_call_and_input_stay_and_only_output_and_status_can_follow", () => {
    const own = { toolCallId: "t", title: "Edit /real", kind: "edit", status: "pending", locations: [{ path: "/real" }], content: diff("/real", "REAL") };
    let v = feed([ask(1, "a1", "t", { input: { file_path: "/real", note: "n" }, toolCall: own }), decide(2, "a1", "t", { outcome: "allow", by: "owner" }, { input: { file_path: "/real", note: "n" }, toolCall: own })]);
    v = feed([patch("t", { title: "Edit /evil", kind: "read", locations: [{ path: "/evil" }], rawInput: { z: 1 }, status: "completed",
      content: [...diff("/evil", "EVIL"), { type: "content", content: { type: "text", text: "OUT" } }, { type: "terminal", terminalId: "x" }], rawOutput: "raw" }),
      entry(5, "tool", { toolCallId: "t", title: "Edit /evil2", kind: "execute", status: "completed" })], v);
    expect(tool(v).call).toMatchObject({ title: "Edit /real", kind: "edit", status: "completed", locations: [{ path: "/real" }], rawOutput: "raw" });
    expect(tool(v).call).not.toHaveProperty("rawInput");
    expect(tool(v).call.content).toEqual([...own.content, { type: "content", content: { type: "text", text: "OUT" } }, { type: "terminal", terminalId: "x" }]);
    const { container } = show(v);
    expect(container.querySelector(".wa-input-label")!.textContent).toBe("Input");
    expect(container.textContent).toContain("OUT");
    expect(container.querySelector(".wa-askbar")).toBeNull();
    expect(container.querySelector("[data-asking]")).toBeNull();
    expect(screen.queryByText("Waiting for you")).toBeNull();
  });
  it("ac6_a_call_with_an_ask_is_never_folded_into_explored", () => {
    let v = feed([call("r1", "read"), call("r2", "read"), ask(1, "a1", "r2", { toolCall: { toolCallId: "r2", title: "Read x", kind: "read" } }), call("r3", "read")]);
    expect(v.rows.map((r) => r.kind)).toEqual(["explored", "tool", "explored"]);
    v = feed([patch("r2", { status: "completed", kind: "read" }), decide(2, "a1", "r2", { outcome: "allow", by: "owner" }), patch("r2", { status: "completed" })], v);
    expect(v.rows.filter((r) => r.kind === "explored" && r.calls.some((c) => c.id === "r2"))).toEqual([]);
    expect(v.rows.some((r) => r.kind === "tool" && r.call.id === "r2")).toBe(true);
  });
});

describe("S15-1 checks", () => {
  it("ac7_a_check_is_a_pending_check_with_no_answer", () => {
    const v = feed([ask(1, "c1", "t", { kind: "check", input: { command: "ls" }, toolCall: { toolCallId: "t", title: "ls", kind: "execute" } })]);
    expect(v.openAsks).toEqual([]);
    const { container } = show(v);
    expect(screen.getByText("Checking")).toBeInTheDocument();
    expect(container.querySelector(".wa-input")).not.toBeNull();
    expect(container.querySelector(".wa-askbar")).toBeNull();
    expect(screen.queryByRole("button", { name: /Allow|Deny/ })).toBeNull();
    expect(container.querySelector("[data-asking]")).toBeNull();
  });
  it("ac7_a_check_is_not_the_status_lines_subject_and_its_decision_is_a_line", () => {
    const v = feed([call("t", "edit"), ask(1, "c1", "t", { kind: "check" }), ask(2, "a1", "u", { toolCall: { toolCallId: "u", title: "Run it", kind: "execute" } })]);
    const view = { ...v, turn: { state: "waiting", asks: ["a1"], expiresAt: 5000 } as SessionView["turn"] };
    render(<StatusLine view={view} now={0} stopping={false} actions={mk()} onNeedsYou={() => {}} />);
    expect(screen.getByRole("status").textContent).toBe("Waiting for you · Execute Run it");
    const d = feed([decide(3, "c1", "t", { outcome: "deny", by: "checked", reason: "r" }, { kind: "check" })], v);
    expect(d.rows[d.rows.length - 1]).toMatchObject({ kind: "decision", what: "Denied: t", who: "checked" });
    expect(d.openAsks).toEqual(["a1"]);
  });
});

describe("S15-1 the fake", () => {
  it("ac9_the_story_writes_the_s10_wire_with_times_relative_to_now", () => {
    const wire = story.flat().map((m) => JSON.parse(m));
    const a = wire.map((m) => m._meta?.watane?.entry).find((e) => e?.kind === "ask");
    expect(a.body).toMatchObject({ id: "a8", kind: "ask", toolCallId: "e2", reach: expect.any(String), input: expect.any(Object), toolCall: expect.any(Object) });
    expect(typeof a.body["expires-at"]).toBe("number");
    expect(a.body).not.toHaveProperty("expiresAt");
    const w = wire.map((m) => m._meta?.watane?.state).find((s) => s?.state === "waiting");
    expect(w).toMatchObject({ asks: ["a8"], "expires-at": a.body["expires-at"] });
    const run = wire.map((m) => m._meta?.watane?.state).find((s) => s?.state === "running");
    expect(Math.abs(Date.now() - run.since)).toBeLessThan(60_000);
    const first = feed(story[0]!);
    expect(first.turn).toMatchObject({ state: "running" });
    render(<StatusLine view={first} now={Date.now()} stopping={false} actions={mk()} onNeedsYou={() => {}} />);
    expect(within(screen.getByRole("status").parentElement!).getByText(/^· \d+ s$/)).toBeInTheDocument();
  });
});
