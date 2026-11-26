// S134-5: the `child` link (AC1-AC4), the stopped limit (AC5) and the usage clear (AC6) on the scripted fake kernel.
import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { fakeKernel } from "../fake/kernel";
import { storyInfo as INFO } from "../fake/page";
import { emptyView, type Kernel } from "../kit";
import { reduce } from "../kit/reduce";
import { App } from "./App";
import { Console } from "./Console";

afterEach(cleanup);
const tool = (i: number, id: string, title: string) => JSON.stringify({ v: 1, i, kind: "tool", at: i, body: { toolCallId: id, title, kind: "other", status: "completed" } });
const child = (i: number, session: string, toolCallId: string) => JSON.stringify({ v: 1, i, kind: "child", at: i, body: { session, toolCallId } });
const meta = (m: object) => JSON.stringify({ _meta: { watane: m } });
const link = /Open child session/;

describe("S134-5 AC1 the link beside its call", () => {
  it("ac1_two_children_of_two_calls_render_one_link_each_beside_their_own_call", async () => {
    const k = fakeKernel([], { preload: [tool(1, "c1", "delegate one"), tool(2, "c2", "delegate two"), child(3, "s9", "c1"), child(4, "s10", "c2")] });
    render(<App kernel={k} info={INFO} />);
    const c1 = (await screen.findByLabelText("delegate one")).closest("section")!;
    const c2 = screen.getByLabelText("delegate two").closest("section")!;
    expect(within(c1).getAllByRole("button", { name: link })).toHaveLength(1);
    expect(within(c1).getByRole("button", { name: link })).toHaveTextContent("s9");
    expect(within(c2).getByRole("button", { name: link })).toHaveTextContent("s10");
    expect(screen.getAllByRole("button", { name: link })).toHaveLength(2);
  });
});

describe("S134-5 AC3 replay and live, no doubled link", () => {
  it("ac3_link_from_replay_and_from_a_pushed_entry_is_one_link", async () => {
    const k = fakeKernel([[meta({ type: "entry", entry: JSON.parse(child(4, "s10", "c2")) })]],
      { preload: [tool(1, "c1", "delegate one"), tool(2, "c2", "delegate two"), child(3, "s9", "c1")] });
    render(<App kernel={k} info={INFO} />);
    await screen.findByLabelText("delegate one");
    expect(screen.getAllByRole("button", { name: link })).toHaveLength(1);
    await act(async () => { k.play(0); });
    await waitFor(() => expect(screen.getAllByRole("button", { name: link })).toHaveLength(2));
  });
  it("ac3_the_reducer_keeps_one_link_when_the_entry_is_read_twice", () => {
    let v = reduce(emptyView, tool(1, "c1", "d"));
    v = reduce(v, child(2, "s9", "c1"));
    v = reduce(v, child(2, "s9", "c1"));
    expect(v.rows.filter((r) => r.kind === "tool" && r.child === "s9")).toHaveLength(1);
    expect(v.rows).toHaveLength(1);
  });
});

describe("S134-5 AC4 no matching call", () => {
  it("ac4_a_child_with_no_tool_row_is_its_own_row_with_the_link", async () => {
    const k = fakeKernel([], { preload: [child(1, "s9", "gone")] });
    render(<App kernel={k} info={INFO} />);
    expect(await screen.findByRole("button", { name: link })).toHaveTextContent("s9");
  });
});

describe("S134-5 AC2 activating the link", () => {
  function served(refuse: string[] = []) {
    const base = fakeKernel([], { preload: [tool(1, "c1", "delegate one"), child(2, "s9", "c1"), child(3, "bad", "c1")] });
    const kernel: Kernel = Object.assign(Object.create(base), {
      read: async (s: string, a: number, m: number) => {
        if (refuse.includes(s)) throw Object.assign(new Error("unknown-session"), { kind: "unknown-session" });
        return base.read(s, a, m);
      },
    });
    return { base, served: { kernel, info: INFO, sessions: [], list: async () => ({ sessions: [], info: (id: string) => (id === "s9" ? { ...INFO, id: "s9", title: "The child" } : undefined) }) } };
  }
  it("ac2_the_link_opens_the_child_through_attach_and_shows_it_in_the_header", async () => {
    const { base, served: s } = served();
    render(<Console served={s} />);
    fireEvent.click(await screen.findByRole("button", { name: /Open child session s9/ }));
    await waitFor(() => expect(base.calls.some((c) => c.name === "attach" && c.args[0] === "s9")).toBe(true));
    expect(await screen.findByRole("heading", { level: 1, name: "The child" })).toBeInTheDocument();
  });
  it("ac2_a_refused_child_shows_the_ordinary_error_and_keeps_the_open_session", async () => {
    const { base, served: s } = served(["bad"]);
    render(<Console served={s} />);
    fireEvent.click(await screen.findByRole("button", { name: /Open child session bad/ }));
    expect(await screen.findByRole("alert")).toHaveTextContent("This console has no such session.");
    expect(base.calls.some((c) => c.name === "attach" && c.args[0] === "bad")).toBe(false);
    expect(screen.getByRole("heading", { level: 1, name: INFO.title })).toBeInTheDocument();
  });
});

describe("S134-5 AC5 stopped at a limit", () => {
  for (const limit of ["max-turns", "max-tool-errors", "max-tokens"]) {
    it(`ac5_${limit.replace(/-/g, "_")}_names_the_limit_in_the_status_line_and_the_turn_end_line`, async () => {
      const run = meta({ type: "turn", state: { state: "running", since: 1 } });
      const end = meta({ type: "turn", state: { state: "idle", stop: { kind: "stopped", limit }, seconds: 4, tools: 1 } });
      const k = fakeKernel([[run, end]], { preload: [tool(1, "c1", "t")] });
      render(<App kernel={k} info={INFO} />);
      await screen.findByLabelText("t");
      await act(async () => { k.play(0); });
      await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent(`Stopped at the ${limit}`));
      expect(screen.getByText(new RegExp(`^Stopped at the ${limit} after 4 s`))).toBeInTheDocument();
    });
  }
});

describe("S134-5 AC6 the usage clear", () => {
  it("ac6_a_null_usage_clears_the_meter_and_a_later_sample_shows_again", async () => {
    const sample = meta({ type: "usage", "context-used": 1200, "context-window": 272000 });
    const clear = meta({ type: "usage", "context-used": null, "context-window": null });
    const k = fakeKernel([[sample], [clear], [sample]], { preload: [tool(1, "c1", "t")] });
    render(<App kernel={k} info={INFO} />);
    await screen.findByLabelText("t");
    await act(async () => { k.play(0); });
    expect(await screen.findByRole("meter")).toBeInTheDocument();
    await act(async () => { k.play(1); });
    await waitFor(() => expect(screen.queryByRole("meter")).toBeNull());
    expect(screen.queryByText("An event this console does not know")).toBeNull();
    await act(async () => { k.play(2); });
    expect(await screen.findByRole("meter")).toBeInTheDocument();
  });
});
