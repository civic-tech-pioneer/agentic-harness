// TECHNICAL 18.3, 18.5, 18.6: component tests of the transcript and status line from scripted wire messages.
import { describe, expect, it, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { emptyView, type Actions, type SessionView } from "./model";
import { reduce } from "./reduce";
import { ThreadProvider, Transcript } from "./thread";
import { StatusLine } from "./status";
import { story } from "../fake/story";

const j = JSON.stringify;
const w = (m: object) => j({ _meta: { watane: m } });
const upd = (u: object, i?: number, at?: number) => j({ ...u, ...(i || at ? { _meta: { watane: { ...(i ? { i } : {}), ...(at ? { at } : {}) } } } : {}) });
const text = (t: string) => ({ type: "text", text: t });
const feed = (...msgs: string[]) => msgs.reduce(reduce, emptyView);
const mk = (): Actions => ({ say: vi.fn(), answer: vi.fn(), stop: vi.fn(), takeBack: vi.fn() });
const show = (view: SessionView, actions = mk()) =>
  render(<ThreadProvider view={view} actions={actions}><Transcript view={view} actions={actions} /></ThreadProvider>);
const call = (id: string, title: string, kind: string, extra: object = {}) =>
  upd({ sessionUpdate: "tool_call", toolCallId: id, title, kind, status: "pending", ...extra });
const finish = (id: string, i: number, extra: object = {}) =>
  upd({ sessionUpdate: "tool_call_update", toolCallId: id, status: "completed", ...extra }, i, 5000);
const diff = (path: string, o: string, n: string) => [{ type: "diff", path, oldText: o, newText: n }];
const entry = (i: number, kind: string, body: object) => w({ type: "entry", entry: { v: 1, i, kind, at: 1000, body } });

describe("rows", () => {
  it("ac1_user_row_is_a_panel_aligned_right", () => {
    show(feed(upd({ sessionUpdate: "user_message_chunk", content: text("Hello") }, 1, 1000)));
    expect(screen.getByText("Hello")).toHaveAttribute("data-align", "right");
  });

  it("ac2_thought_open_while_streaming_then_closed_disclosure", async () => {
    const live = feed(upd({ sessionUpdate: "agent_thought_chunk", content: text("pondering") }, undefined, 1000));
    const a = show(live);
    expect(screen.getByText("Thinking…")).toBeInTheDocument();
    expect(screen.getByText("pondering")).toBeInTheDocument();
    a.unmount();
    const done = reduce(live, upd({ sessionUpdate: "agent_message_chunk", content: text("ok") }, 2, 4000));
    show(done);
    const b = screen.getByRole("button", { name: "Thought for 3 s" });
    expect(b).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText("pondering")).toBeNull();
    const u = userEvent.setup();
    await u.click(b);
    expect(screen.getByText("pondering")).toBeInTheDocument();
    await u.click(b);
    b.focus();
    await u.keyboard("{Enter}");
    expect(screen.getByText("pondering")).toBeInTheDocument();
    await u.keyboard(" ");
    expect(screen.queryByText("pondering")).toBeNull();
  });

  it("ac3_explored_line_counts_and_lists", async () => {
    show(feed(
      call("r1", "Read a.ts", "read"), finish("r1", 1), call("r2", "Read b.ts", "read"), finish("r2", 2),
      call("r3", "Read c.ts", "read"), finish("r3", 3), call("s1", "Search x", "search"), finish("s1", 4)));
    const b = screen.getByRole("button", { name: "Explored · 3 reads, 1 search" });
    expect(b).toHaveAttribute("aria-expanded", "false");
    await userEvent.setup().click(b);
    for (const t of ["Read a.ts", "Read b.ts", "Read c.ts", "Search x"]) expect(screen.getByText(t)).toBeInTheDocument();
  });

  it("ac3_zero_part_left_out_and_singular", () => {
    show(feed(call("r1", "Read a.ts", "read"), finish("r1", 1)));
    expect(screen.getByRole("button", { name: "Explored · 1 read" })).toBeInTheDocument();
  });

  it("ac4_edit_card_with_diff_and_status", async () => {
    show(feed(call("e1", "Edit src/a.ts", "edit", { locations: [{ path: "src/a.ts" }] }),
      finish("e1", 1, { content: diff("src/a.ts", "old line\nkeep", "new line\nkeep") })));
    expect(screen.getByRole("region", { name: "Edit src/a.ts" })).toBeInTheDocument();
    await userEvent.setup().click(screen.getByRole("button", { name: /Edit src\/a.ts Done/ }));
    expect(screen.getByText("Done")).toBeInTheDocument();
    expect(screen.getByText("old line").parentElement).toHaveAttribute("data-diff", "del");
    expect(screen.getByText("new line").parentElement).toHaveAttribute("data-diff", "add");
    expect(screen.getByText("old line").parentElement).toHaveTextContent("-");
    expect(screen.getByText("new line").parentElement).toHaveTextContent("+");
    expect(screen.getByText("keep").parentElement).toHaveAttribute("data-diff", "same");
  });

  it("ac4_execute_output_is_escaped_text", () => {
    const hostile = "<img src=x onerror=alert(1)>\u001b[31mred\u001b[0m";
    const { container } = show(feed(call("x1", "npm test", "execute"),
      finish("x1", 1, { content: [{ type: "content", content: text(hostile) }] }).replace("completed", "failed")));
    expect(container.querySelector("img")).toBeNull();
    expect(container.textContent).toContain("<img src=x onerror=alert(1)>red");
    expect(container.textContent).not.toContain("\u001b");
  });

  it("ac4_failed_card_is_open_completed_is_closed", () => {
    show(feed(call("f1", "Edit f.ts", "edit", { content: diff("f.ts", "a", "b") }),
      finish("f1", 1).replace("completed", "failed"),
      call("g1", "Edit g.ts", "edit", { content: diff("g.ts", "c", "d") }), finish("g1", 2)));
    const f = screen.getByRole("region", { name: "Edit f.ts" });
    expect(f.textContent).not.toContain("Edit Edit");
    expect(within(f).getByRole("button")).toHaveAttribute("aria-expanded", "true");
    expect(within(f).getByText("Failed")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "Edit g.ts" })).getByRole("button")).toHaveAttribute("aria-expanded", "false");
  });
});

const asking = () => feed(
  call("e2", "Edit src/sync.ts", "edit", { locations: [{ path: "src/sync.ts" }], content: diff("src/sync.ts", "a", "b") }),
  entry(8, "ask", { id: "a-e2", kind: "ask", toolCallId: "e2", reach: "edits in src", input: { file_path: "src/sync.ts", note: "hi" }, "expires-at": 9,
    toolCall: { toolCallId: "e2", title: "Edit src/sync.ts", kind: "edit", status: "pending", locations: [{ path: "src/sync.ts" }], content: diff("src/sync.ts", "a", "b") } }));

describe("ask", () => {
  it("ac5_open_ask_shows_input_diff_and_bar_and_answers", async () => {
    const actions = mk();
    show(asking(), actions);
    expect(screen.getByRole("region").textContent).toContain("file_path");
    expect(screen.getByRole("region").textContent).toContain("note");
    expect(screen.getByText("b").parentElement).toHaveAttribute("data-diff", "add");
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Allow once" }));
    expect(actions.answer).toHaveBeenLastCalledWith("a-e2", { outcome: "allow" });
    await u.click(screen.getByRole("button", { name: "Allow edits in src this session" }));
    expect(actions.answer).toHaveBeenLastCalledWith("a-e2", { outcome: "allow", for: "session" });
    await u.click(screen.getByRole("button", { name: "Deny…" }));
    await u.type(screen.getByRole("textbox", { name: "Tell the agent why" }), "no thanks");
    await u.click(screen.getByRole("button", { name: "Deny" }));
    expect(actions.answer).toHaveBeenLastCalledWith("a-e2", { outcome: "deny", reason: "no thanks" });
  });

  it("ac5_non_edit_ask_without_diff_still_shows_its_input", () => {
    show(feed(call("b1", "Run ls", "execute"), entry(9, "ask", { id: "a-b1", kind: "ask", toolCallId: "b1", input: { command: "ls -la" }, "expires-at": 9 })));
    expect(screen.getByText("command")).toBeInTheDocument();
    expect(within(document.querySelector(".wa-input")! as HTMLElement).getByText("ls -la")).toBeInTheDocument();
  });

  it("ac5_no_reach_no_session_allow_button", () => {
    show(feed(call("e3", "Edit notes", "edit"), entry(9, "ask", { id: "a-e3", kind: "ask", toolCallId: "e3", input: {}, "expires-at": 9 })));
    expect(screen.getByRole("button", { name: "Allow once" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /this session/ })).toBeNull();
  });

  it("ac5_esc_and_cancel_close_the_reason_field_without_a_call", async () => {
    const actions = mk();
    show(asking(), actions);
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Deny…" }));
    await u.keyboard("{Escape}");
    expect(screen.queryByRole("textbox")).toBeNull();
    await u.click(screen.getByRole("button", { name: "Deny…" }));
    await u.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("textbox")).toBeNull();
    expect(actions.answer).not.toHaveBeenCalled();
  });
});

describe("other rows", () => {
  it("ac6_decision_line_names_who_what_and_reason", () => {
    show(feed(entry(3, "ask", { id: "a-t", kind: "ask", toolCallId: "t", input: {}, "expires-at": 1, decision: { outcome: "deny", reason: "too broad", by: "alice" } })));
    const el = document.querySelector(".wa-decision")!;
    expect(el.textContent).toContain("alice");
    expect(el.textContent).toContain("Denied");
    expect(el.textContent).toContain("too broad");
  });

  it("ac7_footer_line", () => {
    show(feed(w({ type: "turn", state: { state: "running", since: 1 } }),
      w({ type: "turn", state: { state: "idle", stop: { kind: "completed" }, seconds: 9, tools: 2, tokens: 18000 } })));
    expect(screen.getByText("Completed in 9 s · 2 tools · 18k tokens")).toBeInTheDocument();
  });

  it("ac7_footer_without_tokens", () => {
    show(feed(w({ type: "turn", state: { state: "running", since: 1 } }),
      w({ type: "turn", state: { state: "idle", stop: { kind: "completed" }, seconds: 9, tools: 1 } })));
    expect(screen.getByText("Completed in 9 s · 1 tool")).toBeInTheDocument();
  });

  it("ac8_unknown_row_raw_on_demand_escaped", async () => {
    const { container } = show(feed(entry(5, "unknown", { raw: { x: "<img src=x onerror=alert(1)>" } })));
    expect(screen.getByText("An event this console does not know")).toBeInTheDocument();
    expect(container.querySelector("pre")).toBeNull();
    await userEvent.setup().click(screen.getByRole("button", { name: "Show raw" }));
    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector("pre")!.textContent).toContain("<img src=x onerror=alert(1)>");
  });

  it("ac10_busy_only_while_writing", () => {
    const writing = feed(upd({ sessionUpdate: "agent_message_chunk", content: text("Hel") }, undefined, 1000));
    const a = show(writing);
    expect(screen.getByText("Hel")).toHaveAttribute("aria-busy", "true");
    a.unmount();
    const b = show(reduce(writing, upd({ sessionUpdate: "agent_message_chunk", content: text("lo") }, 2, 2000)));
    expect(screen.getByText("Hello")).not.toHaveAttribute("aria-busy");
    b.unmount();
    show(feed(w({ type: "synced", through: 1, writing: true })));
    expect(screen.getByText("Writing…")).toHaveAttribute("aria-busy", "true");
  });

  it("ac11_story_end_view_renders_each_row_once", () => {
    const view = story.flat().reduce(reduce, emptyView);
    const { container } = show(view);
    const kinds = [...container.querySelectorAll(".wa-row")].map((e) => e.getAttribute("data-kind"));
    expect(kinds.length).toBe(view.rows.length);
    for (const k of ["user", "thought", "explored", "tool", "agent"]) expect(kinds).toContain(k);
    expect(container.querySelectorAll("[data-kind='user']").length).toBe(1);
    expect(container.querySelectorAll("[data-kind='explored']").length).toBe(1);
    expect(screen.getAllByRole("region").length).toBe(2);
  });
});

describe("StatusLine", () => {
  const T = 1_700_000_000_000;
  const askE = (i: number, id: string, title: string, kind: string, path?: string) =>
    entry(i, "ask", { id: `a-${id}`, kind: "ask", toolCallId: id, input: {}, "expires-at": 9, toolCall: { toolCallId: id, title, kind, status: "pending", ...(path ? { locations: [{ path }] } : {}) } });
  const status = (turn: object | null, o: { stopping?: boolean; rows?: string[]; asks?: string[] } = {}) => {
    const view = { ...feed(...(o.rows ?? [])), turn: turn as SessionView["turn"], openAsks: o.asks ?? [] };
    const actions = mk(); const onNeedsYou = vi.fn();
    const r = render(<StatusLine view={view} now={T + 12_000} stopping={!!o.stopping} actions={actions} onNeedsYou={onNeedsYou} />);
    return { actions, onNeedsYou, ...r };
  };
  const word = () => screen.getByRole("status").textContent;
  const shape = () => document.querySelector("[data-shape]")!.getAttribute("data-shape");
  /** An act with no kernel call yet: a disabled, described button that sends nothing to the agent. */
  const noKernel = (s: { actions: Actions }, label: string) => {
    const b = screen.getByRole("button", { name: label });
    expect(b).toBeDisabled();
    expect(b).toHaveAccessibleDescription("This console cannot do this yet.");
    b.click();
    expect(s.actions.say).not.toHaveBeenCalled();
    expect(s.actions.stop).not.toHaveBeenCalled();
  };

  it("ac9_running_thinking_with_clock_and_stop", async () => {
    const s = status({ state: "running", since: T });
    expect(word()).toBe("Thinking");
    expect(shape()).toBe("dot");
    expect(screen.getByRole("img", { name: "Running" })).toBeInTheDocument();
    expect(screen.getByText("· 12 s")).toHaveAttribute("aria-hidden", "true");
    expect(screen.getByRole("status")).not.toHaveTextContent("12");
    expect(screen.queryByRole("button")).toBeNull();
    expect(screen.getByText("Esc").tagName).toBe("KBD");
    expect(document.querySelector(".wa-status-esc")).toHaveTextContent("Esc to stop");
    expect(s.actions.stop).not.toHaveBeenCalled();
  });

  it("ac9_running_reading_names_the_file", () => {
    status({ state: "running", since: T }, { rows: [call("r1", "Read sync.ts", "read", { locations: [{ path: "src/sync.ts" }], status: "in_progress" })] });
    expect(word()).toBe("Reading src/sync.ts");
  });

  it("ac9_running_editing_and_writing", () => {
    status({ state: "running", since: T }, { rows: [call("e1", "Edit", "edit", { locations: [{ path: "a.ts" }] })] });
    expect(word()).toBe("Editing a.ts");
  });

  it("ac9_running_writing", () => {
    status({ state: "running", since: T }, { rows: [upd({ sessionUpdate: "agent_message_chunk", content: text("x") }, undefined, 1)] });
    expect(word()).toBe("Writing");
  });

  it("ac9_checking_square", () => {
    status({ state: "running", since: T, activity: { kind: "checking" } });
    expect(word()).toBe("Checking with your judge");
    expect(shape()).toBe("square");
  });

  it("ac9_compacting_dot", () => {
    status({ state: "running", since: T, activity: { kind: "compacting" } });
    expect(word()).toBe("Making room");
    expect(shape()).toBe("dot");
  });

  it("ac9_consent_diamond_opens_the_question", async () => {
    const s = status({ state: "running", since: T, activity: { kind: "consent", what: "x" } });
    expect(word()).toBe("Waiting for your answer in watane");
    expect(shape()).toBe("diamond");
    await userEvent.setup().click(screen.getByRole("button", { name: "Open the question" }));
    expect(s.onNeedsYou).toHaveBeenCalled();
  });

  it("ac9_waiting_diamond_time_left_needs_you", async () => {
    const rows = [call("a", "Edit file", "edit"), askE(1, "a", "Edit file", "edit", "src/sync.ts"), call("b", "Edit other", "edit"), askE(2, "b", "Edit other", "edit", "x.ts")];
    status({ state: "waiting", asks: ["a-a", "a-b"], expiresAt: T + 72_000 }, { rows, asks: ["a-a", "a-b"] });
    expect(word()).toBe("Waiting for you · Edit src/sync.ts");
    expect(shape()).toBe("diamond");
    expect(screen.getByText("· denied in 1:00 if unanswered")).toHaveAttribute("aria-hidden", "true");
    expect(screen.queryByRole("button")).toBeNull();
  });

  it("ac9_waiting_subject_falls_back_to_the_title_and_the_clock_stops_at_zero", () => {
    status({ state: "waiting", asks: ["a-t"], expiresAt: T + 5_000 }, { rows: [call("t", "Run npm test", "execute"), askE(1, "t", "Run npm test", "execute")], asks: ["a-t"] });
    expect(word()).toBe("Waiting for you · Execute Run npm test");
    expect(screen.getByText("· denied in 0:00 if unanswered")).toBeInTheDocument();
  });

  it("ac9_running_clock_is_m_ss_from_a_minute", () => {
    status({ state: "running", since: T - 63_000 });
    expect(screen.getByText("· 1:15")).toHaveAttribute("aria-hidden", "true");
  });

  it("ac9_paused_bars_resume_now", () => {
    const until = new Date(2026, 0, 2, 7, 5).getTime();
    const s = status({ state: "paused", until });
    expect(word()).toBe("Paused until 07:05");
    expect(shape()).toBe("bars");
    noKernel(s, "Resume now");
  });

  it("ac9_idle_completed_ring", () => {
    status({ state: "idle", stop: { kind: "completed" } });
    expect(word()).toBe("Idle");
    expect(shape()).toBe("ring");
    expect(screen.queryByRole("button")).toBeNull();
  });

  it("ac9_idle_stopped_ring_names_limit", () => {
    status({ state: "idle", stop: { kind: "stopped", limit: "turn limit" } });
    expect(word()).toBe("Stopped at the turn limit");
    expect(shape()).toBe("ring");
  });

  it("ac9_idle_failed_triangle_try_again", () => {
    const s = status({ state: "idle", stop: { kind: "failed", error: "rate limit" } });
    expect(word()).toBe("Failed: rate limit");
    expect(shape()).toBe("triangle");
    noKernel(s, "Try again");
  });

  it("ac9_unknown_dashed_ring_continue", () => {
    const s = status({ state: "unknown" });
    expect(word()).toBe("Not running: watane restarted");
    expect(shape()).toBe("dashed-ring");
    noKernel(s, "Continue");
  });

  it("ac9_resume_lost_dashed_ring_continue_from_summary", () => {
    const s = status({ state: "resume-lost" });
    expect(word()).toBe("Claude no longer has this session");
    expect(shape()).toBe("dashed-ring");
    noKernel(s, "Continue from a summary");
  });

  it("ac9_stopping_shows_until_idle", () => {
    const a = status({ state: "running", since: T }, { stopping: true });
    expect(word()).toBe("Stopping…");
    expect(screen.queryByRole("button", { name: /Stop/ })).toBeNull();
    a.unmount();
    status({ state: "idle", stop: { kind: "interrupted" } }, { stopping: true });
    expect(word()).toBe("Idle");
  });
});
