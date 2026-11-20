// S9 acceptance: an Edit whose old text ends without a newline and whose new text appends a line after it
// (the common shape of a Claude Edit's old_string/new_string) must show the last line unchanged and only
// the appended line as added, not the unchanged line as removed and added again.
import { render } from "@testing-library/react";
import { expect, test } from "vitest";
import { DiffView } from "./parts/diff";

test("regress_S9_diff_last_line: an unchanged last line is not shown as removed and added", () => {
  const { container } = render(<DiffView path="src/sync.ts" oldText={"a\nexport default sync;"} newText={"a\nexport default sync;\nexport { retry };"} />);
  const marked = Array.from(container.querySelectorAll("[data-diff]")).map((e) => `${e.getAttribute("data-diff")} ${e.lastElementChild?.textContent}`);
  expect(marked).toEqual(["same a", "same export default sync;", "add export { retry };"]);
});
