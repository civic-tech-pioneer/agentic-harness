// S23-fix10 AC3: after a busy refusal, focus is on the refused line (never a button); focus outside the card stays.
import { describe, expect, it, vi } from "vitest";
import { render, screen, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { emptyView, type Actions } from "./model";
import { reduce } from "./reduce";
import { ThreadProvider, Transcript } from "./thread";

const J = JSON.stringify;
const entry = (i: number, kind: string, body: object) => J({ _meta: { watane: { type: "entry", entry: { v: 1, i, kind, at: i, body } } } });
const ask = (i: number, id: string) =>
  entry(i, "ask", { id, kind: "ask", toolCallId: "t" + id, input: { command: "ls" }, reach: "ls", "expires-at": 9, toolCall: { toolCallId: "t" + id, title: "mcp__x__run", rawInput: { command: "ls" } } });

async function setup(outside: boolean) {
  const v = [ask(1, "a1")].reduce(reduce, emptyView);
  const calls: unknown[] = [];
  let refuse: () => void = () => {};
  const a = { say: vi.fn(), stop: vi.fn(), takeBack: vi.fn(),
    answer: vi.fn((_id: string, d: unknown) => { calls.push(d); return new Promise<void>((_, rej) => { refuse = () => rej(Object.assign(new Error("busy"), { kind: "busy" })); }); }) } as Actions;
  render(<><ThreadProvider view={v} actions={a}><Transcript view={v} actions={a} /></ThreadProvider><input aria-label="Message" /></>);
  const u = userEvent.setup();
  return { u, calls, outside, refuse: async () => { await act(async () => { refuse(); await Promise.resolve(); }); } };
}

const sends: [string, (u: ReturnType<typeof userEvent.setup>) => Promise<void>][] = [
  ["allow_once", async (u) => { await u.click(screen.getByRole("button", { name: "Allow once" })); }],
  ["session_allow", async (u) => { await u.click(screen.getByRole("button", { name: /this session$/ })); }],
  ["deny", async (u) => { await u.click(screen.getByRole("button", { name: "Deny…" })); await u.click(screen.getByRole("button", { name: /^Deny$/ })); }],
];

describe("S23 fix10", () => {
  for (const [name, go] of sends) {
    it(`ac3_${name}_refused_focus_on_the_line_and_enter_sends_nothing`, async () => {
      const s = await setup(false);
      await go(s.u);
      expect(s.calls).toHaveLength(1);
      await s.refuse();
      const line = await screen.findByRole("alert");
      expect(line).toHaveFocus();
      expect(screen.getByRole("button", { name: "Allow once" })).toBeInTheDocument();
      await s.u.keyboard("{Enter}");
      expect(s.calls).toHaveLength(1);
    });
    it(`ac3_${name}_refused_leaves_focus_outside_the_card`, async () => {
      const s = await setup(true);
      await go(s.u);
      await s.u.click(screen.getByRole("textbox", { name: "Message" }));
      await s.refuse();
      await screen.findByRole("alert");
      expect(screen.getByRole("textbox", { name: "Message" })).toHaveFocus();
    });
  }
});
