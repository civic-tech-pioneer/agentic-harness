// TECHNICAL 18.3 (the table), 18.6: one component per row kind.
import { useState } from "react";
import type { Actions, Row, ToolCall } from "../model";
import { AskBar } from "./askbar";
import { DiffView } from "./diff";
import { Disclosure } from "./disclosure";
import { ProgramOutput } from "./output";

type Of<K extends Row["kind"]> = Extract<Row, { kind: K }>;
const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

export function UserRow({ row }: { row: Of<"user"> }) {
  return <div className="wa-user" data-align="right">{row.text}</div>;
}

export function AgentRow({ row }: { row: Of<"agent"> }) {
  return (
    <div className="wa-agent" aria-busy={row.writing ? "true" : undefined}>
      {row.writing && !row.text ? "Writing…" : row.text}
    </div>
  );
}

export function ThoughtRow({ row }: { row: Of<"thought"> }) {
  if (row.streaming) {
    return <div className="wa-thought" data-open="true" aria-busy="true"><p className="wa-fold-label">Thinking…</p><div className="wa-fold-body">{row.text}</div></div>;
  }
  const label = row.seconds === undefined ? "Thought" : `Thought for ${row.seconds} s`;
  return <Disclosure className="wa-thought" label={label}>{row.text}</Disclosure>;
}

export function ExploredRow({ row }: { row: Of<"explored"> }) {
  const parts = [row.reads ? plural(row.reads, "read", "reads") : "", row.searches ? plural(row.searches, "search", "searches") : ""].filter(Boolean);
  return (
    <Disclosure className="wa-explored" label={`Explored · ${parts.join(", ")}`}>
      <ul>{row.calls.map((c) => <li key={c.id}>{c.title}</li>)}</ul>
    </Disclosure>
  );
}

const STATUS: Record<ToolCall["status"], string> = { pending: "Pending", in_progress: "Running", completed: "Done", failed: "Failed" };
const KIND = (k: string) => k.charAt(0).toUpperCase() + k.slice(1);

function InputList({ input }: { input: unknown }) {
  if (!input || typeof input !== "object") return <pre className="wa-args">{String(input ?? "")}</pre>;
  return (
    <dl className="wa-args">
      {Object.entries(input as Record<string, unknown>).map(([k, v]) => (
        <div key={k}><dt>{k}</dt><dd>{typeof v === "string" ? v : JSON.stringify(v)}</dd></div>
      ))}
    </dl>
  );
}

export function ToolRow({ row, actions }: { row: Of<"tool">; actions: Actions }) {
  const { call, ask } = row;
  const asking = !!ask && !ask.decision;
  const path = call.locations[0]?.path;
  const name = path ? `${KIND(call.kind)} ${path}` : call.title;
  const body = (
    <>
      {asking ? <InputList input={ask.input} /> : null}
      {call.content.map((c, n) =>
        c.type === "diff" ? <DiffView key={n} path={c.path} oldText={c.oldText} newText={c.newText} />
        : c.type === "content" ? <ProgramOutput key={n} text={c.content.text} />
        : null)}
      {typeof call.rawOutput === "string" && !call.content.some((c) => c.type === "content") ? <ProgramOutput text={call.rawOutput} /> : null}
      {asking ? <AskBar call={call} input={ask.input} actions={actions} /> : null}
    </>
  );
  const head = (
    <>
      {path ? <><span className="wa-tool-name">{KIND(call.kind)}</span>{" "}</> : null}
      <span className="wa-tool-arg">{path ?? call.title}</span>{" "}
      <span className="wa-tool-status" data-status={call.status}>{STATUS[call.status]}</span>
    </>
  );
  return (
    <section className="wa-tool" data-status={call.status} data-asking={asking ? "true" : undefined} aria-label={name}>
      <ToolCard head={head} open={call.status === "failed" || asking}>{body}</ToolCard>
    </section>
  );
}

function ToolCard({ head, open, children }: { head: React.ReactNode; open: boolean; children: React.ReactNode }) {
  const [shown, setShown] = useState(open);
  return (
    <>
      <button type="button" className="wa-tool-head" aria-expanded={shown || open} onClick={() => setShown(!shown)}>{head}</button>
      {shown || open ? <div className="wa-tool-body">{children}</div> : null}
    </>
  );
}

export function DecisionRow({ row }: { row: Of<"decision"> }) {
  return <p className="wa-decision">{row.who} · {row.what}{row.reason ? ` — ${row.reason}` : ""}</p>;
}

export function FooterRow({ row }: { row: Of<"footer"> }) {
  const parts = [`Completed in ${row.seconds} s`, plural(row.tools, "tool", "tools")];
  if (row.tokens !== undefined) parts.push(`${row.tokens >= 1000 ? `${Math.round(row.tokens / 1000)}k` : row.tokens} tokens`);
  return <p className="wa-footer">{parts.join(" · ")}</p>;
}

export function UnknownRow({ row }: { row: Of<"unknown"> }) {
  const [raw, setRaw] = useState(false);
  return (
    <div className="wa-unknown">
      <span>An event this console does not know</span>{" "}
      <button type="button" aria-expanded={raw} onClick={() => setRaw(!raw)}>{raw ? "Hide raw" : "Show raw"}</button>
      {raw ? <pre className="wa-output">{JSON.stringify(row.raw, null, 2)}</pre> : null}
    </div>
  );
}

export function RowView({ row, actions }: { row: Row; actions: Actions }) {
  switch (row.kind) {
    case "user": return <UserRow row={row} />;
    case "agent": return <AgentRow row={row} />;
    case "thought": return <ThoughtRow row={row} />;
    case "explored": return <ExploredRow row={row} />;
    case "tool": return <ToolRow row={row} actions={actions} />;
    case "decision": return <DecisionRow row={row} />;
    case "footer": return <FooterRow row={row} />;
    case "unknown": return <UnknownRow row={row} />;
  }
}
