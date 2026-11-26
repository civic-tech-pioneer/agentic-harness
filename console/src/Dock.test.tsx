// TECHNICAL 18.4-18.6: the dock's Needs you pill, plan fold, composer, Stop and footer.
import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { emptyView, type Actions, type SessionView } from "../kit";
import { Dock } from "./Dock";

const mk = (): Actions => ({ say: vi.fn(), answer: vi.fn(), stop: vi.fn(), takeBack: vi.fn() });
const V = (p: Partial<SessionView>): SessionView => ({ ...emptyView, ...p });
const running = { state: "running", since: 0 } as SessionView["turn"];
const dock = (view: SessionView, extra: Partial<Parameters<typeof Dock>[0]> = {}, actions = mk()) => {
  render(<Dock view={view} actions={actions} onNeedsYou={() => {}} status={<p>status</p>} {...extra} />);
  return actions;
};
const box = () => screen.getByRole("textbox", { name: "Message" });

describe("Dock", () => {
  it("ac3_needs_you_pill_has_an_arrow_and_keeps_its_name", async () => {
    const onNeedsYou = vi.fn();
    dock(V({ openAsks: ["a", "b"] }), { onNeedsYou });
    const b = screen.getByRole("button", { name: "Needs you (2)" });
    expect(b.querySelector("svg")).toHaveAttribute("aria-hidden", "true");
    await userEvent.setup().click(b);
    expect(onNeedsYou).toHaveBeenCalled();
  });

  it("ac4_placeholders", () => {
    const a = render(<Dock view={V({})} actions={mk()} onNeedsYou={() => {}} status={null} />);
    expect(box()).toHaveAttribute("placeholder", "Message the agent  ·  Enter to send, Shift+Enter for a new line");
    a.unmount();
    dock(V({ turn: running }));
    expect(box()).toHaveAttribute("placeholder", "Add a message: it is queued and sent when this turn ends");
  });

  it("ac4_ac6_full_queue_turns_send_off_and_says_why", () => {
    dock(V({ turn: running, queue: Array.from({ length: 32 }, (_, n) => ({ n, input: `q${n}` })) }));
    expect(box()).toHaveAttribute("placeholder", "The queue is full: 32 messages wait. Remove one, or stop the turn.");
    const b = screen.getByRole("button", { name: "Queue message" });
    expect(b).toBeDisabled();
    expect(b).toHaveAccessibleDescription(/queue is full/);
    expect(screen.getByText(/^The queue is full:/)).toHaveAttribute("role", "status");
  });

  it("ac5_stop_button_only_while_a_turn_runs_or_waits", async () => {
    const a = render(<Dock view={V({})} actions={mk()} onNeedsYou={() => {}} status={null} />);
    expect(screen.queryByRole("button", { name: /Stop/ })).toBeNull();
    a.unmount();
    const idle = V({ turn: { state: "idle", stop: { kind: "completed" } } as SessionView["turn"] });
    const b = render(<Dock view={idle} actions={mk()} onNeedsYou={() => {}} status={null} />);
    expect(screen.queryByRole("button", { name: /Stop/ })).toBeNull();
    b.unmount();
    const actions = dock(V({ turn: { state: "waiting", asks: [], expiresAt: 1 } as SessionView["turn"] }));
    await userEvent.setup().click(screen.getByRole("button", { name: "Stop" }));
    expect(actions.stop).toHaveBeenCalled();
  });

  it("ac5_stopping_label", () => {
    dock(V({ turn: running }), { stopping: true });
    expect(screen.getByRole("button", { name: "Stopping…" })).toBeInTheDocument();
  });

  it("ac6_send_button_names_and_disabled_when_empty", async () => {
    dock(V({}));
    const s = screen.getByRole("button", { name: "Send message" });
    expect(s).toBeDisabled();
    expect(s).toHaveAccessibleDescription("Enter a message first.");
    expect(screen.getByText("Enter a message first.")).toBeVisible();
    expect(s.querySelector("svg")).toHaveAttribute("aria-hidden", "true");
    await userEvent.setup().type(box(), "hi");
    expect(s).toBeEnabled();
  });

  it("ac7_footer_tags", () => {
    const a = render(<Dock view={V({})} actions={mk()} onNeedsYou={() => {}} status={null} session={{ model: "Claude Sonnet", mode: "Ask before edits" }} />);
    expect(document.querySelector(".wa-irow")).toHaveTextContent("Claude Sonnet· Ask before edits");
    a.unmount();
    render(<Dock view={V({})} actions={mk()} onNeedsYou={() => {}} status={null} session={{ mode: "Plan" }} />);
    expect(document.querySelector(".wa-irow .wa-tag")).toHaveTextContent(/^Plan$/);
    expect(document.querySelectorAll(".wa-tag").length).toBe(1);
  });

  it("ac7_no_session_no_tags", () => {
    dock(V({}));
    expect(document.querySelectorAll(".wa-tag").length).toBe(0);
  });

  it("ac8_plan_fold_has_a_chevron_and_toggles", async () => {
    dock(V({ openAsks: ["a"], plan: [{ content: "x", status: "completed" }, { content: "y", status: "pending" }] }));
    const b = screen.getByRole("button", { name: "Plan · 1 of 2 done" });
    expect(b.querySelector("svg")).toHaveAttribute("aria-hidden", "true");
    expect(b).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText("y")).toBeNull();
    const u = userEvent.setup();
    await u.click(b);
    expect(b).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("y")).toBeInTheDocument();
    await u.click(b);
    expect(b).toHaveAttribute("aria-expanded", "false");
  });

  it("ac8_open_plan_keeps_its_line", () => {
    dock(V({ plan: [{ content: "x", status: "completed" }] }));
    expect(screen.getByRole("button", { name: "Plan · 1 of 1 done" })).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("x")).toBeInTheDocument();
  });
});
