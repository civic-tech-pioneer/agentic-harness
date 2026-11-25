// S40-6 AC4, AC5: a ChatGPT session in the console: header and sidebar words, the stream, the pause that lifts on its own, the context meter.
import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { Console } from "./Console";
import { served } from "./served";
import type { Served } from "./served";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { fakeWatane } from "../kit/watane_fake";
import { pausedUntil } from "../fake/chatgpt";

afterEach(cleanup);
const tick = () => act(async () => { await new Promise((r) => setTimeout(r, 80)); });
const box = () => screen.getByRole("textbox", { name: "Message" }) as HTMLTextAreaElement;

describe("S40-6 AC4 the header and the sidebar name ChatGPT", () => {
  it("ac4_an_openai_session_shows_ChatGPT_dot_profile_and_a_claude_scratch_keeps_its_words", async () => {
    const f = fakeWatane({
      "sessions.list": () => ({ ok: [
        { id: "a", title: "Sorting", profile: "Sort", backend: "openai", turn: "idle", updated: 9 },
        { id: "b", title: "Old", profile: "scratch", backend: "claude", workspace: null, turn: "idle", updated: 1 }] }),
      "sessions.attach": () => ({ ok: "st1" }), "sessions.read": () => ({ ok: { entries: [], last: 0 } }),
    });
    const s = await served(f.w);
    expect(s.sessions.map((r) => r.meta)).toEqual(["ChatGPT · Sort", "Scratch folder · Claude Code (unchecked)"]);
    render(<Console served={s} />);
    expect(document.querySelector(".wa-header")!.textContent).toContain("ChatGPT · Sort");
    expect(document.querySelector(".wa-sidebar")!.textContent).toContain("ChatGPT · Sort");
  });
});

describe("S40-6 AC5 a ChatGPT session streams", () => {
  it("ac5_text_a_tool_call_the_footer_a_pause_that_lifts_and_the_meter", async () => {
    const kernel = fakeKernel(story);
    const info = { id: "new1", title: "Tag the untagged notes", profile: "Sort", provider: "ChatGPT", workspace: "" };
    const s: Served = { kernel, info: null, sessions: [], list: async () => ({ sessions: [], info: () => info }) };
    render(<Console served={s} />);
    await tick();
    fireEvent.click(screen.getByRole("radio", { name: /Sort/ }));
    await tick();
    fireEvent.change(box(), { target: { value: "Tag the untagged notes" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await tick();
    expect(kernel.calls.find((c) => c.name === "start")!.args[1]).toBe("Sort");
    await waitFor(() => expect(document.body.textContent).toContain("I will list the notes first."));
    act(() => kernel.play(1));
    await waitFor(() => expect(document.body.textContent).toContain("Three notes need tags."));
    expect(document.body.textContent).toContain("notes: list");
    expect(document.body.textContent).toMatch(/1 tool/);
    const meter = screen.getByRole("meter", { name: "Context used" });
    expect(meter).toHaveAttribute("aria-valuenow", "12000");
    act(() => kernel.play(2));
    await waitFor(() => expect(document.querySelector(".wa-status-word")!.textContent).toMatch(/^Paused/));
    const hh = new Date(pausedUntil);
    const hhmm = `${String(hh.getHours()).padStart(2, "0")}:${String(hh.getMinutes()).padStart(2, "0")}`;
    expect(document.querySelector(".wa-status-word")!.textContent).toBe(`Paused until ${hhmm}`);
    expect(document.querySelector(".wa-shape")).toHaveAttribute("data-shape", "bars");
    act(() => kernel.play(3)); // the kernel lifts the pause: no click
    await waitFor(() => expect(document.querySelector(".wa-status-word")!.textContent).not.toMatch(/Paused/));
    expect(document.querySelector(".wa-shape")).toHaveAttribute("data-shape", "dot"); // Running again, with the status word of a running turn
  });
});
