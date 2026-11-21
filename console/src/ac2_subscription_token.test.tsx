// S23-fix7 AC2: the held view belongs to one subscription; a new kernel prop with the same id shows nothing of the old one.
import { describe, expect, it } from "vitest";
import { useLayoutEffect } from "react";
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
    // the first commit with k2 runs before the openSession effect sets the new token: it must not show the old card either
    const seen: boolean[] = [];
    const Probe = ({ kernel }: { kernel: Kernel }) => {
      useLayoutEffect(() => { if (kernel === k2) seen.push(!!document.querySelector("[data-ask][data-asking]")); });
      return <App kernel={kernel} info={INFO} />;
    };
    const { rerender } = render(<Probe kernel={f1} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    const old = document.querySelector<HTMLElement>("[data-ask][data-asking] [data-ask-first]");
    rerender(<Probe kernel={k2} />);
    await act(async () => { await Promise.resolve(); });
    expect(seen[0], "the first commit on the new kernel showed the old card").toBe(false);
    expect(document.querySelector("[data-ask][data-asking]")).toBeNull();
    if (old) await act(async () => { fireEvent.click(old); await Promise.resolve(); });
    expect(f1.calls.filter((c) => c.name === "answer")).toEqual([]);
    expect(f2.calls.filter((c) => c.name === "answer")).toEqual([]);
  });
});
