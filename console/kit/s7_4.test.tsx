// S7-4: status subject, decided-never-ran tag, check beside an ask, ended after restart.
import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { emptyView, type Actions, type SessionView } from "./model";
import { reduce } from "./reduce";
import { ThreadProvider, Transcript } from "./thread";
import { StatusLine } from "./status";

const J = JSON.stringify;
const meta = (m: object) => J({ _meta: { watane: m } });
const entry = (i: number, kind: string, body: object) => meta({ type: "entry", entry: { v: 1, i, kind, at: i, body } });
const feed = (msgs: string[], v: SessionView = emptyView) => msgs.reduce(reduce, v);
const call = (id: string, kind: string, extra: object = {}) => J({ sessionUpdate: "tool_call", toolCallId: id, title: `Streamed ${id}`, kind, status: "pending", ...extra });
const patch = (id: string, extra: object) => J({ sessionUpdate: "tool_call_update", toolCallId: id, ...extra });
const askBody = (id: string, tc: string, extra: object = {}) => ({ id, kind: "ask", toolCallId: tc, input: {}, "expires-at": 9, ...extra });
const ask = (i: number, id: string, tc: string, extra: object = {}) => entry(i, "ask", askBody(id, tc, extra));
const turn = (state: object) => meta({ type: "turn", state });
const mk = (): Actions => ({ say: vi.fn(), answer: vi.fn(), stop: vi.fn(), takeBack: vi.fn() });
const show = (view: SessionView) => { const a = mk(); return render(<ThreadProvider view={view} actions={a}><Transcript view={view} actions={a} /></ThreadProvider>); };
const status = (view: SessionView) => { const a = mk(); return render(<StatusLine view={view} now={0} stopping={false} actions={a} onNeedsYou={() => {}} />); };
const tool = (v: SessionView, k = 0) => v.rows[k] as Extract<SessionView["rows"][number], { kind: "tool" }>;
const waiting = (a: string) => turn({ state: "waiting", asks: [a], "expires-at": 5 });

describe("S7-4", () => {
  it("ac1_status_names_the_ask_call_once", () => {
    const v = feed([call("t", "edit"), ask(1, "a1", "t", { input: {}, toolCall: { toolCallId: "t", title: "Edit s.ts", kind: "edit" } }), waiting("a1")]);
    status(v);
    expect(screen.getByRole("status").textContent).toBe("Waiting for you · Edit s.ts");
  });
  it("ac1_an_other_ask_without_argument_shows_its_title_alone", () => {
    const v = feed([ask(1, "a1", "t", { toolCall: { toolCallId: "t", title: "other" } }), waiting("a1")]);
    status(v);
    expect(screen.getByRole("status").textContent).toBe("Waiting for you · other");
  });
  it("ac1_the_argument_wins", () => {
    const v = feed([ask(1, "a1", "t", { input: { file_path: "src/s.ts" }, toolCall: { toolCallId: "t", kind: "edit", title: "Edit s.ts" } }), waiting("a1")]);
    status(v);
    expect(screen.getByRole("status").textContent).toBe("Waiting for you · Edit src/s.ts");
  });
  it.each([["allow", "Allowed", "b-ok"], ["deny", "Denied", "b-bad"], ["none", "No objection", "b-off"]])("ac2_decided_never_ran_%s", (o, word, cls) => {
    const v = feed([ask(1, "a1", "t", { toolCall: { toolCallId: "t", kind: "edit", title: "Edit s.ts" } }), ask(2, "a1", "t", { toolCall: { toolCallId: "t", kind: "edit", title: "Edit s.ts" }, decision: { outcome: o, by: "you" } })]);
    const { container } = show(v);
    const tag = container.querySelector(".wa-tool-status")!;
    expect(tag.textContent).toBe(word);
    expect(tag.className).toContain(cls);
  });
  it("ac2_the_decision_still_explains_a_call_that_never_ran_after_the_turn_ends", () => {
    const v = feed([ask(1, "a1", "t", { toolCall: { toolCallId: "t", kind: "edit", title: "Edit s.ts" } }), ask(2, "a1", "t", { toolCall: { toolCallId: "t", kind: "edit", title: "Edit s.ts" }, decision: { outcome: "deny", by: "you" } }), turn({ state: "unknown" })]);
    expect(tool(v)).toMatchObject({ ended: true, call: { status: "pending" } });
    expect(show(v).container.querySelector(".wa-tool-status")!.textContent).toBe("Denied");
  });
  it("ac2_a_later_status_shows_as_today", () => {
    const v = feed([ask(1, "a1", "t", { toolCall: { toolCallId: "t", kind: "edit", title: "Edit s.ts" } }), ask(2, "a1", "t", { toolCall: { toolCallId: "t", kind: "edit", title: "Edit s.ts" }, decision: { outcome: "allow", by: "you" } }), patch("t", { status: "completed" })]);
    const { container } = show(v);
    expect(container.querySelector(".wa-tool-status")!.textContent).toBe("Done");
  });
  it("ac3_a_check_beside_an_open_ask_shows_checking_and_goes_when_decided", () => {
    let v = feed([call("t", "edit"), ask(1, "a1", "t"), ask(2, "c1", "t", { kind: "check" })]);
    expect(tool(v)).toMatchObject({ ask: { id: "a1" }, checking: true });
    const { container } = show(v);
    expect(container.querySelector(".wa-tool-status")!.textContent).toBe("Waiting for you");
    expect(container.querySelector(".wa-tool-checking")!.textContent).toBe("Checking");
    v = feed([ask(3, "c1", "t", { kind: "check", decision: { outcome: "none", by: "judge" } })], v);
    expect(tool(v).checking).toBeUndefined();
    expect(tool(v).ask?.id).toBe("a1");
  });
  it("ac3_a_lone_check_has_no_second_tag", () => {
    const v = feed([call("t", "edit"), ask(1, "c1", "t", { kind: "check" })]);
    expect(tool(v).checking).toBeUndefined();
    expect(show(v).container.querySelector(".wa-tool-checking")).toBeNull();
  });
  it.each([
    [turn({ state: "unknown" })], [turn({ state: "resume-lost" })], [turn({ state: "idle", stop: { kind: "completed" } })],
  ])("ac4_a_call_nothing_drives_ends_with_the_turn", (t) => {
    let v = feed([turn({ state: "running", since: 0 }), call("t", "execute", { status: "in_progress" }), t]);
    expect(tool(v)).toMatchObject({ ended: true, call: { status: "in_progress" } });
    const { container } = show(v);
    expect(container.querySelector(".wa-tool-status")!.textContent).toBe("Ended");
    v = feed([patch("t", { status: "completed" })], v);
    expect(tool(v).ended).toBeUndefined();
  });
  it("ac4_a_call_with_an_open_ask_is_not_marked", () => {
    const v = feed([call("t", "edit"), ask(1, "a1", "t"), turn({ state: "unknown" })]);
    expect(tool(v).ended).toBeUndefined();
  });
  it("ac4_the_running_word_never_names_an_ended_call", () => {
    const v = feed([call("t", "read", { title: "x", locations: [{ path: "a.ts" }] }), call("u", "edit", { locations: [{ path: "b.ts" }] }), turn({ state: "unknown" })]);
    status({ ...v, turn: { state: "running", since: 0 } });
    expect(screen.getByRole("status").textContent).toBe("Thinking");
  });
});
