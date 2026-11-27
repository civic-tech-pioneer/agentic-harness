// Adversary S145: Esc in the composer (TECHNICAL 18.6 "Keys") stops the turn once, and only when asked.
import { describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { fakeKernel } from "../fake/kernel";
import { storyInfo as INFO } from "../fake/page";
import { App } from "./App";
import { Dock } from "./Dock";

vi.mock("./Dock", async (importOriginal) => {
  const { Dock } = await importOriginal<typeof import("./Dock")>();
  return { Dock: vi.fn(Dock) };
});

const running = JSON.stringify({ _meta: { watane: { type: "turn", state: { state: "running", since: 1 } } } });
const stops = (fake: ReturnType<typeof fakeKernel>) => fake.calls.filter(({ name }) => name === "interrupt" || name === "cancel");

async function runningApp() {
  const fake = fakeKernel([[running]]);
  fake.play(0);
  render(<App kernel={fake} info={INFO} />);
  await screen.findByRole("button", { name: /Stop/ });
  return { fake, box: screen.getByRole("textbox", { name: "Message" }) };
}

describe("adversary S145 Esc", () => {
  it("S152_Escape_while_loading_makes_no_Stop_call", async () => {
    const fake = fakeKernel([]);
    fake.attach = () => new Promise(() => {}); // the kernel has sent no turn state yet
    render(<App kernel={fake} info={INFO} />);
    expect(screen.getByText("Loading", { exact: true })).toBeVisible();
    fireEvent.keyDown(screen.getByRole("textbox", { name: "Message" }), { key: "Escape" });
    expect(stops(fake)).toEqual([]);
  });

  it("ac1_a_direct_stop_action_during_loading_sends_no_kernel_call", () => {
    const fake = fakeKernel([]);
    fake.attach = () => new Promise(() => {});
    render(<App kernel={fake} info={INFO} />);
    expect(screen.getByText("Loading", { exact: true })).toBeVisible();
    const dockProps = vi.mocked(Dock).mock.lastCall![0];
    act(() => dockProps.actions.stop());
    expect(stops(fake)).toEqual([]);
    expect(screen.getByText("Loading", { exact: true })).toBeVisible();
  });

  it("adv_S145_esc_that_cancels_an_ime_composition_does_not_stop_the_turn", async () => {
    const { fake, box } = await runningApp();
    // Escape pressed to dismiss an IME candidate window: the browser reports it as composing.
    fireEvent.keyDown(box, { key: "Escape", isComposing: true });
    expect(stops(fake)).toEqual([]);
  });

  it("adv_S145_held_esc_key_repeat_sends_one_interrupt", async () => {
    const { fake, box } = await runningApp();
    fireEvent.keyDown(box, { key: "Escape" });
    // auto-repeat of the same held key while the view still reads running ("Stopping…")
    fireEvent.keyDown(box, { key: "Escape", repeat: true });
    fireEvent.keyDown(box, { key: "Escape", repeat: true });
    expect(stops(fake)).toHaveLength(1);
  });
});
