// S23-fix7 AC2: the held view belongs to one subscription; a new kernel prop with the same id shows nothing of the old one.
import { describe, expect, it } from "vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { App } from "./App";
import { storyInfo as INFO } from "../fake/page";
import type { Kernel } from "../kit";

describe("S23-fix7 AC2", () => {
  it("ac2_new_kernel_same_id_old_card_gone_click_sends_nothing", async () => {
    const f1 = fakeKernel(story);
    for (let n = 0; n < story.length; n++) f1.play(n);
    const f2 = fakeKernel(story);
    const k2: Kernel = { ...f2, attach: () => new Promise(() => {}) };
    const { rerender } = render(<App kernel={f1} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    const old = document.querySelector<HTMLElement>("[data-ask][data-asking] [data-ask-first]");
    rerender(<App kernel={k2} info={INFO} />);
    await act(async () => { await Promise.resolve(); });
    expect(document.querySelector("[data-ask][data-asking]")).toBeNull();
    if (old) await act(async () => { fireEvent.click(old); await Promise.resolve(); });
    expect(f1.calls.filter((c) => c.name === "answer")).toEqual([]);
    expect(f2.calls.filter((c) => c.name === "answer")).toEqual([]);
  });
});
