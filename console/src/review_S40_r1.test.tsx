// S40 review r1, F1.1 (TECHNICAL 18.6 "New session", 2.3): the view honours the kernel's profile list, an empty one too, and checks only listed profiles.
import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import { Console } from "./Console";
import { served } from "./served";
import { fakeWatane } from "../kit/watane_fake";
import type { Kernel, ProfileInfo, Refusal } from "../kit";

afterEach(cleanup);
const tick = () => act(async () => { await new Promise((r) => setTimeout(r, 60)); });

function setup(profiles: unknown[]) {
  return fakeWatane({
    "sessions.list": () => ({ ok: [] }),
    "sessions.profiles": () => ({ ok: profiles }),
    "sessions.check": () => ({ ok: [] }),
    "sessions.start": () => ({ ok: "n1" }),
    "sessions.attach": () => ({ ok: "st1" }),
    "sessions.read": () => ({ ok: { entries: [], last: 0 } }),
  });
}

describe("S40 r1 F1.1 the profile list is the kernel's", () => {
  it("r1_f1_1_an_empty_list_shows_no_profile_and_keeps_start_off_saying_why", async () => {
    const f = setup([]);
    render(<Console served={await served(f.w)} />);
    await tick();
    expect(screen.queryAllByRole("radio")).toHaveLength(0);
    const box = screen.getByRole("textbox", { name: "Message" });
    await act(async () => { (box as HTMLTextAreaElement).value = "hi"; box.dispatchEvent(new Event("input", { bubbles: true })); });
    expect(screen.getByRole("button", { name: "Start session" })).toBeDisabled();
    expect(document.body.textContent).toContain("no profile");
    expect(f.calls.filter((c) => c.fn === "sessions.check")).toHaveLength(0);
    expect(f.calls.filter((c) => c.fn === "sessions.start")).toHaveLength(0);
  });
  it("r1_f1_1_a_sort_only_list_checks_sort_alone", async () => {
    const f = setup([{ name: "Sort", backend: "openai", model: "gpt-5" }]);
    render(<Console served={await served(f.w)} />);
    await tick();
    expect(screen.getAllByRole("radio")).toHaveLength(1);
    expect(f.calls.filter((c) => c.fn === "sessions.check").map((c) => c.args.profile)).toEqual(["Sort"]);    expect(screen.getByRole("textbox", { name: "Message" })).toHaveAttribute("placeholder", "Message ChatGPT…"); // P1.3
  });
  it("r1_f1_1_profiles_and_check_are_required_of_a_kernel", () => {
    // `npm run typecheck` holds this: an optional call is not a function here (2.3: every client has both)
    const k = { profiles: async () => [], check: async () => [] } as unknown as Kernel;
    const p: () => Promise<ProfileInfo[]> = k.profiles;
    const c: (profile: string, overlay: string) => Promise<Refusal[]> = k.check;
    expect([typeof p, typeof c]).toEqual(["function", "function"]);
  });
});
