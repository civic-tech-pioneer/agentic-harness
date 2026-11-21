// S23-fix7 AC2: the subscription token is made per effect run; StrictMode's double effect run (dev) still shows the session.
import { describe, expect, it } from "vitest";
import { StrictMode } from "react";
import { render, screen } from "@testing-library/react";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { App } from "./App";
import { storyInfo as INFO } from "../fake/page";

describe("S23-fix7 AC2", () => {
  it("ac2_strict_mode_double_effect_still_shows_the_session", async () => {
    const f = fakeKernel(story);
    for (let n = 0; n < story.length; n++) f.play(n);
    render(<StrictMode><App kernel={f} info={INFO} /></StrictMode>);
    await screen.findByRole("button", { name: "Needs you (1)" });
    expect(document.querySelector("[data-ask][data-asking]")).not.toBeNull();
  });
});
