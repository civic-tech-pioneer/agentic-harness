// TECHNICAL 18.2, 18.3: a line diff without eval; removed and added lines carry a text prefix, not only a colour.
import { diffLines } from "diff";

export function DiffView({ path, oldText, newText }: { path: string; oldText: string | null; newText: string }) {
  const parts = diffLines(oldText ?? "", newText);
  const lines: { mark: "add" | "del" | "same"; text: string }[] = [];
  for (const p of parts) {
    const mark = p.added ? "add" : p.removed ? "del" : "same";
    const body = p.value.endsWith("\n") ? p.value.slice(0, -1) : p.value;
    for (const text of body.split("\n")) lines.push({ mark, text });
  }
  return (
    <div className="wa-diff" role="group" aria-label={`Changes to ${path}`}>
      {lines.map((l, n) => (
        <div key={n} className="wa-diff-line" data-diff={l.mark}>
          <span className="wa-diff-mark" aria-hidden="true">{l.mark === "add" ? "+" : l.mark === "del" ? "-" : " "}</span>
          <span className="wa-sr">{l.mark === "add" ? "Added: " : l.mark === "del" ? "Removed: " : ""}</span>
          <span>{l.text}</span>
        </div>
      ))}
    </div>
  );
}
