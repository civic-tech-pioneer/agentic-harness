// TECHNICAL 18.2, 18.3: a line diff without eval; removed and added lines carry a text prefix, not only a colour.
// Unchanged lines more than CONTEXT away from a change fold into one "⋯ n unchanged lines" line.
import { diffLines } from "diff";

const CONTEXT = 3;
type Mark = "add" | "del" | "same";
type Line = { mark: Mark; text: string } | { gap: number };

function fold(all: { mark: Mark; text: string }[]): Line[] {
  const near = all.map(() => false);
  all.forEach((l, n) => {
    if (l.mark === "same") return;
    for (let k = Math.max(0, n - CONTEXT); k <= Math.min(all.length - 1, n + CONTEXT); k++) near[k] = true;
  });
  const out: Line[] = [];
  let run = 0;
  all.forEach((l, n) => {
    if (l.mark === "same" && !near[n]) { run++; return; }
    if (run) { out.push({ gap: run }); run = 0; }
    out.push(l);
  });
  if (run) out.push({ gap: run });
  return out;
}

export function DiffView({ path, oldText, newText }: { path: string; oldText: string | null; newText: string }) {
  const all: { mark: Mark; text: string }[] = [];
  for (const p of diffLines(oldText ?? "", newText)) {
    const mark: Mark = p.added ? "add" : p.removed ? "del" : "same";
    const body = p.value.endsWith("\n") ? p.value.slice(0, -1) : p.value;
    for (const text of body.split("\n")) all.push({ mark, text });
  }
  return (
    <div className="wa-diff diff" role="group" aria-label={`Changes to ${path}`}>
      {fold(all).map((l, n) => "gap" in l ? (
        <div key={n} className="dl gap">{`⋯ ${l.gap} unchanged ${l.gap === 1 ? "line" : "lines"}`}</div>
      ) : (
        <div key={n} className={`wa-diff-line dl ${l.mark}`} data-diff={l.mark}>
          <span className="wa-diff-mark sg" aria-hidden="true">{l.mark === "add" ? "+" : l.mark === "del" ? "-" : " "}</span>
          <span className="wa-sr">{l.mark === "add" ? "Added: " : l.mark === "del" ? "Removed: " : ""}</span>
          <span>{l.text}</span>
        </div>
      ))}
    </div>
  );
}
