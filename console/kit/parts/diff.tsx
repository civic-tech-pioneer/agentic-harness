// TECHNICAL 18.2, 18.3: a line diff without eval; removed and added lines carry a text prefix, not only a colour.
// Unchanged lines more than CONTEXT away from a change fold into one "⋯ n unchanged lines" line.
import { diffLines } from "diff";

const CONTEXT = 3;
type Mark = "add" | "del" | "same";
type Row = { mark: Mark; text: string; note?: boolean };
type Line = Row | { gap: number };

function fold(all: Row[]): Line[] {
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
  const all: Row[] = [];
  let o = oldText ?? "", n = newText;
  const bare = (t: string) => t !== "" && !t.endsWith("\n");
  const oldBare = bare(o), newBare = bare(n);
  // both sides lack a final newline: compare lines, not the missing newline. Exactly one lacks it: that is a change.
  if (oldBare && newBare) { o += "\n"; n += "\n"; }
  let lastDel = -1, lastAdd = -1;
  for (const p of diffLines(o, n)) {
    const mark: Mark = p.added ? "add" : p.removed ? "del" : "same";
    const body = p.value.endsWith("\n") ? p.value.slice(0, -1) : p.value;
    for (const text of body.split("\n")) {
      all.push({ mark, text });
      if (mark === "del") lastDel = all.length; else if (mark === "add") lastAdd = all.length;
    }
  }
  if (o !== "" && n !== "" && oldBare !== newBare) {
    all.splice(oldBare ? lastDel : lastAdd, 0, { mark: oldBare ? "del" : "add", text: "No newline at end of file", note: true });
  }
  return (
    <div className="wa-diff diff" role="group" aria-label={`Changes to ${path}`}>
      {fold(all).map((l, n) => "gap" in l ? (
        <div key={n} className="dl gap">{`⋯ ${l.gap} unchanged ${l.gap === 1 ? "line" : "lines"}`}</div>
      ) : (
        <div key={n} className={`wa-diff-line dl ${l.mark}${l.note ? " nonl" : ""}`} data-diff={l.mark}>
          <span className="wa-diff-mark sg" aria-hidden="true">{l.mark === "add" ? "+" : l.mark === "del" ? "-" : " "}</span>
          <span className="wa-sr">{l.mark === "add" ? "Added: " : l.mark === "del" ? "Removed: " : ""}</span>
          <span>{l.text}</span>
        </div>
      ))}
    </div>
  );
}
