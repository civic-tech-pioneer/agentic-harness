// S9-1: the transcript rows look and read like the frame (Main-2): card header, diff with folds, ask bar, chevrons.
import { describe, expect, it, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { emptyView, type Actions, type SessionView } from "./model";
import { reduce } from "./reduce";
import { ThreadProvider, Transcript } from "./thread";
import { DiffView } from "./parts/diff";
import { Chevron, Pencil, Search } from "./parts/icons";
import { story } from "../fake/story";

const j = JSON.stringify;
const w = (m: object) => j({ _meta: { watane: m } });
const upd = (u: object, i?: number, at?: number) => j({ ...u, ...(i || at ? { _meta: { watane: { ...(i ? { i } : {}), ...(at ? { at } : {}) } } } : {}) });
const feed = (...msgs: string[]) => msgs.reduce(reduce, emptyView);
const mk = (): Actions => ({ say: vi.fn(), answer: vi.fn(), stop: vi.fn(), takeBack: vi.fn() });
const show = (view: SessionView, actions = mk()) =>
  render(<ThreadProvider view={view} actions={actions}><Transcript view={view} actions={actions} /></ThreadProvider>);
const call = (id: string, title: string, kind: string, extra: object = {}) =>
  upd({ sessionUpdate: "tool_call", toolCallId: id, title, kind, status: "pending", ...extra });
const diff = (path: string, o: string | null, n: string) => [{ type: "diff", path, oldText: o, newText: n }];
const entry = (i: number, kind: string, body: object) => w({ type: "entry", entry: { v: 1, i, kind, at: 1000, body } });
const askOf = (id: string, kind: string, content: object[], input: object, reach?: string) => feed(
  call(id, kind === "edit" ? "Edit a.ts" : "Run it", kind, { locations: [{ path: "a.ts" }], content }),
  entry(8, "ask", { toolCallId: id, ...(reach ? { reach } : {}), input, expiresAt: 9,
    toolCall: { toolCallId: id, title: "x", kind, status: "pending", locations: [{ path: "a.ts" }], content } }));
const lines = (n: number, from = 1) => Array.from({ length: n }, (_, k) => `line ${from + k}`).join("\n");

describe("S9-1", () => {
  it("ac1_edit_ask_header_has_pencil_name_path_and_waiting_tag", () => {
    show(askOf("e", "edit", diff("a.ts", "x", "y"), { file_path: "a.ts" }));
    const card = screen.getByRole("region", { name: "Edit a.ts" });
    const head = card.querySelector(".chead")!;
    expect(head.querySelector("svg")).toHaveAttribute("aria-hidden", "true");
    expect(head.textContent).toContain("Edit");
    expect(head.querySelector("code")!.textContent).toBe("a.ts");
    expect(within(card).getByText("Waiting for you")).toHaveClass("badge", "b-wait");
    expect(card.textContent).not.toContain("Pending");
  });

  it("ac1_tag_per_status", () => {
    const cases: [string, string, string][] = [["in_progress", "Running", "b-off"], ["completed", "Done", "b-ok"], ["failed", "Failed", "b-bad"], ["pending", "Pending", "b-off"]];
    for (const [status, word, cls] of cases) {
      const view = feed(call("t", "Edit a.ts", "edit", { locations: [{ path: "a.ts" }], status }));
      const { unmount } = show(view);
      expect(screen.getByText(word)).toHaveClass("badge", cls);
      unmount();
    }
  });

  it("ac2_diff_ask_shows_diff_and_input_and_command_ask_keeps_it", () => {
    const a = show(askOf("e", "edit", diff("a.ts", "x", "y"), { file_path: "a.ts" }));
    expect(screen.getByRole("region").textContent).toContain("file_path");
    expect(screen.getByText("y").closest(".dl")).toHaveClass("add");
    a.unmount();
    show(askOf("b", "execute", [], { command: "git status" }));
    expect(screen.getByText("command")).toBeInTheDocument();
    expect(screen.getByText("git status")).toBeInTheDocument();
  });

  it("ac2_input_block_is_labelled_and_below_the_diff", () => {
    show(askOf("e", "edit", diff("a.ts", "x", "y"), { file_path: "a.ts", note: "n" }));
    const label = screen.getByText("Input");
    expect(label).toHaveClass("wa-input-label");
    const block = label.closest(".wa-input")!;
    expect(block.textContent).toContain("file_path");
    expect(block.textContent).toContain("note");
    expect(screen.getByText("y").compareDocumentPosition(block) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("ac4_one_diff_shows_its_path_only_when_it_differs_from_the_header", () => {
    const a = show(askOf("e", "edit", diff("b.ts", "x", "y"), { file_path: "a.ts" }));
    expect(a.container.querySelector(".wa-diff-path")!.textContent).toBe("b.ts");
    a.unmount();
    const b = show(askOf("e", "edit", diff("a.ts", "x", "y"), { file_path: "a.ts" }));
    expect(b.container.querySelector(".wa-diff-path")).toBeNull();
  });

  it("ac3_diff_classes_and_prefixes", () => {
    const { container } = render(<DiffView path="a" oldText="a\nb" newText="a\nc" />);
    const add = container.querySelector(".dl.add")!;
    const del = container.querySelector(".dl.del")!;
    expect(add.textContent).toContain("Added: ");
    expect(del.textContent).toContain("Removed: ");
  });

  it("ac3_long_unchanged_runs_fold_with_three_lines_of_context", () => {
    const old = `${lines(8, 1)}\nOLD1\n${lines(20, 9)}\nOLD2\n${lines(8, 29)}`;
    const nu = `${lines(8, 1)}\nNEW1\n${lines(20, 9)}\nNEW2\n${lines(8, 29)}`;
    const { container } = render(<DiffView path="a" oldText={old} newText={nu} />);
    const gaps = [...container.querySelectorAll(".dl.gap")].map((g) => g.textContent);
    expect(gaps).toEqual(["⋯ 5 unchanged lines", "⋯ 14 unchanged lines", "⋯ 5 unchanged lines"]);
    expect(container.querySelectorAll(".dl:not(.gap):not(.add):not(.del)").length).toBe(3 + 3 + 3 + 3);
  });

  it("ac3_run_of_one_and_new_file", () => {
    const a = render(<DiffView path="a" oldText={`${lines(4)}\nX`} newText={`${lines(4)}\nZ`} />);
    expect(a.container.querySelector(".dl.gap")!.textContent).toBe("⋯ 1 unchanged line");
    const b = render(<DiffView path="a" oldText={null} newText={"p\nq\nr"} />);
    expect(b.container.querySelectorAll(".dl.add").length).toBe(3);
    expect(b.container.querySelectorAll(".dl.gap").length).toBe(0);
  });

  it("ac4_ask_bar_order_and_labels", () => {
    show(askOf("e", "edit", diff("a.ts", "x", "y"), { file_path: "a.ts" }, "edits in src"));
    const bar = screen.getByRole("group", { name: "Your answer" });
    expect(bar.firstElementChild).toHaveTextContent("Allow this edit?");
    const names = within(bar).getAllByRole("button").map((b) => b.textContent);
    expect(names).toEqual(["Deny…", "Allow edits in src this session", "Allow once"]);
    expect(within(bar).getByRole("button", { name: "Allow once" })).toHaveAttribute("data-ask-first");
    expect(within(bar).getByRole("button", { name: "Allow once" })).toHaveClass("primary");
    expect(within(bar).getByRole("button", { name: "Deny…" })).toHaveClass("quiet");
  });

  it("ac4_question_per_kind", () => {
    const a = show(askOf("b", "execute", [], { command: "ls" }));
    expect(screen.getByText("Allow this command?")).toBeInTheDocument();
    a.unmount();
    show(askOf("o", "fetch", [], { url: "u" }));
    expect(screen.getByText("Allow this call?")).toBeInTheDocument();
  });

  it("ac4_deny_row_and_focus_return", async () => {
    const actions = mk();
    show(askOf("e", "edit", diff("a.ts", "x", "y"), {}), actions);
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Deny…" }));
    expect(screen.getByRole("textbox", { name: "Tell the agent why" })).toHaveFocus();
    await u.keyboard("{Escape}");
    expect(screen.queryByRole("textbox")).toBeNull();
    await Promise.resolve();
    expect(screen.getByRole("button", { name: "Deny…" })).toHaveFocus();
    await u.click(screen.getByRole("button", { name: "Deny…" }));
    await u.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.getByRole("button", { name: "Deny…" })).toHaveFocus();
    await u.click(screen.getByRole("button", { name: "Deny…" }));
    await u.type(screen.getByRole("textbox", { name: "Tell the agent why" }), "no");
    await u.click(screen.getByRole("button", { name: "Deny" }));
    expect(actions.answer).toHaveBeenCalledWith("e", { outcome: "deny", reason: "no" });
  });

  it("ac5_fold_buttons_have_chevrons_and_names_unchanged", async () => {
    const t = feed(upd({ sessionUpdate: "agent_thought_chunk", content: { type: "text", text: "hm" } }, undefined, 1000),
      upd({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: "ok" } }, 2, 4000));
    show(t);
    const b = screen.getByRole("button", { name: "Thought for 3 s" });
    expect(b.querySelector("svg")).toHaveAttribute("aria-hidden", "true");
    expect(b.querySelector(".chev")).not.toBeNull();
    await userEvent.setup().click(b);
    expect(b).toHaveAttribute("aria-expanded", "true");
    expect(b.className).toMatch(/\bopen\b/);
  });

  it("ac5_explored_has_chevron_and_search_icon", () => {
    const fin = (id: string, i: number) => upd({ sessionUpdate: "tool_call_update", toolCallId: id, status: "completed" }, i, 5000);
    show(feed(call("r1", "Read a", "read"), fin("r1", 1), call("s1", "Search x", "search"), fin("s1", 2)));
    const b = screen.getByRole("button", { name: "Explored · 1 read, 1 search" });
    expect(b.querySelectorAll("svg").length).toBe(2);
    expect(b.querySelector(".chev")).not.toBeNull();
  });

  it("ac5_icons_are_aria_hidden_svgs", () => {
    for (const I of [Chevron, Pencil, Search]) {
      const { container, unmount } = render(<I />);
      const s = container.querySelector("svg")!;
      expect(s).toHaveAttribute("aria-hidden", "true");
      expect(s.outerHTML).toContain("currentColor");
      unmount();
    }
  });

  it("ac6_story_e2_diff_folds", () => {
    const view = story.flat().reduce(reduce, emptyView);
    expect(view.openAsks).toEqual(["e2"]);
    show(view);
    const card = screen.getByRole("region", { name: "Edit src/sync.ts" });
    // At least 10 unchanged lines between two changes: 3 of context on each side, so a fold of 4 or more.
    const folded = [...card.querySelectorAll(".dl.gap")].map((g) => Number(/^⋯ (\d+) unchanged lines?$/.exec(g.textContent!)![1]));
    expect(Math.max(...folded)).toBeGreaterThanOrEqual(4);
  });
});
