// Adversary S23 pass 5: App takes a new session (info.id) in place, but the old session's view (its open ask cards) stays on screen
// until the new session's first frame; an answer on such a card goes out to the NEW session under the old ask's id.
import { describe, expect, it } from "vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { App } from "./App";
import { storyInfo as INFO } from "../fake/page";
import type { Kernel } from "../kit";

describe("adv S23 fix5 session switch", () => {
  it("adv_S23_fix5_answer_on_old_session_card_goes_to_new_session", async () => {
    const f = fakeKernel(story);
    for (let n = 0; n < story.length; n++) f.play(n);
    // session s2: the kernel has not answered its attach yet (a slow link), so no frame of s2 has come
    const k: Kernel = { ...f, attach: (s, a, m, e) => (s === "s2" ? new Promise(() => {}) : f.attach(s, a, m, e)) };
    const { rerender } = render(<App kernel={k} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    const card = document.querySelector<HTMLElement>("[data-asking]")!;
    const askId = card.getAttribute("data-ask");
    rerender(<App kernel={k} info={{ ...INFO, id: "s2", title: "another session" }} />);
    // the card drawn for s1's ask is still on screen, with its bar; the reader clicks it
    const allow = card.isConnected ? card.querySelector<HTMLElement>("[data-ask-first]") : null;
    if (allow) await act(async () => { fireEvent.click(allow); await Promise.resolve(); });
    const sent = f.calls.filter((c) => c.name === "answer");
    // s2's ask with the same id (ids repeat across sessions) is another tool call: no answer may go to s2 for a card s2 never drew
    expect(sent.filter((c) => c.args[0] === "s2"), `answered ${askId} in s2 from s1's card`).toEqual([]);
  });
});
