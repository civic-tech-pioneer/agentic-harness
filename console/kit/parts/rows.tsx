// TECHNICAL 18.3 (the table), 18.6: one component per row kind.
import React, { createContext, useCallback, useContext, useEffect, useId, useMemo, useRef, useState } from "react";
import { errorWords } from "../errors";
import { askArg, cardName, isAskRefusal, kernelClosed, type Actions, type Ask, type Row, type ToolCall } from "../model";
import { AskBar } from "./askbar";
import { DiffView } from "./diff";
import { MessageText } from "./code";
import { Disclosure, useFold } from "./disclosure";
import { Pencil, Search } from "./icons";
import { ProgramOutput } from "./output";

type Of<K extends Row["kind"]> = Extract<Row, { kind: K }>;
const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

export function UserRow({ row }: { row: Of<"user"> }) {
  return <div className="wa-user" data-align="right">{row.text}</div>;
}

export function AgentRow({ row }: { row: Of<"agent"> }) {
  return (
    <div className="wa-agent" aria-busy={row.writing ? "true" : undefined}>
      {row.writing && !row.text ? "Writing…" : <MessageText text={row.text} writing={!!row.writing} />}
    </div>
  );
}

export function ThoughtRow({ row }: { row: Of<"thought"> }) {
  if (row.streaming) {
    return <div className="wa-thought" data-open="true" aria-busy="true"><p className="wa-fold-label">Thinking…</p><div className="wa-fold-body">{row.text}</div></div>;
  }
  const label = row.seconds === undefined ? "Thought" : `Thought for ${row.seconds} s`;
  return <Disclosure className="wa-thought" label={label} stateKey={row.key}>{row.text}</Disclosure>;
}

export function ExploredRow({ row }: { row: Of<"explored"> }) {
  const parts = [row.reads ? plural(row.reads, "read", "reads") : "", row.searches ? plural(row.searches, "search", "searches") : ""].filter(Boolean);
  return (
    <Disclosure className="wa-explored" stateKey={row.key} icon={<Search />} label={`Explored · ${parts.join(", ")}`}>
      <ul>{row.calls.map((c) => <li key={c.id}>{c.title}</li>)}</ul>
    </Disclosure>
  );
}

const STATUS: Record<ToolCall["status"], [string, string]> = {
  pending: ["Pending", "b-off"], in_progress: ["Running", "b-off"], completed: ["Done", "b-ok"], failed: ["Failed", "b-bad"] };
const DECIDED: Record<"allow" | "deny" | "none", [string, string]> = { allow: ["Allowed", "b-ok"], deny: ["Denied", "b-bad"], none: ["No objection", "b-off"] };
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

/** 18.5: the turn is `running` with activity `checking`; an answered ask then reads "Checking with your judge". */
export const JudgeContext = createContext(false);

export function ToolRow({ row, actions }: { row: Of<"tool">; actions: Actions }) {
  const { call, ask } = row;
  const judge = useContext(JudgeContext);
  const pending = !!ask && !ask.decision;
  const open = pending && ask.kind === "ask";
  // 9.4 rule 4: an answer sent and not yet decided counts as decided; a refusal other than "already decided" gives the bar back
  const [sent, setSent] = useState<{ ask: Ask; refused: boolean } | null>(null);
  // read when the refusal arrives, before the re-mount: did focus sit in this card (on its answered line or inside it)?
  const lineRef = useRef<HTMLElement | null>(null);
  // left: the participant moved focus out of the card after the answer went out (null while the answer call runs: a loss then is the card's own)
  const focusIn = useRef(false), wasIn = useRef(false), left = useRef<boolean | null>(null);
  const inCard = (at: Element | null) => !!at && !!lineRef.current?.parentElement?.contains(at);
  const answered = open && actions.answeredAsks?.get(ask.id) === ask;
  const asking = open && !answered;
  const send = useMemo<Actions>(() => ({ ...actions, answer: (id, d) => {
    wasIn.current = inCard(document.activeElement); // the pressed button, or the reason field, holds focus here
    left.current = null;
    setSent({ ask: ask!, refused: false });
    let p: void | Promise<void>;
    try { p = actions.answerDrawn ? actions.answerDrawn(ask!, d) : actions.answer(id, d); } catch (e) { p = Promise.reject(e); }
    left.current = false;
    return Promise.resolve(p).catch((e) => {
      if (isAskRefusal(e)) return;
      // focus on body counts only when this card lost it itself (the pressed button went away), never when the participant left it
      const at = document.activeElement;
      focusIn.current = inCard(at) || (wasIn.current && !left.current && (!at || at === document.body));
      setSent({ ask: ask!, refused: true });
    });
  } }), [actions, ask]);
  const checking = pending && ask.kind === "check";
  // 9.4: a card with an ask names the call by the ask's input, never by what the stream said
  const path = ask ? askArg(ask.input) : call.locations[0]?.path;
  const name = cardName(call, ask);
  const diffs = call.content.filter((c) => c.type === "diff").length;
  const [tag, tagCls] = answered ? ["Answered", "b-off"] : asking ? ["Waiting for you", "b-wait"] : checking ? ["Checking", "b-off"]
    // an ask the kernel closed was cancelled, whatever cleanup result follows (S156 r1 F1.1); an allowed call still pending when its
    // turn ended has no result yet, which is not knowing it never ran: Claude may still report it (S156 r1 F1.2)
    : ask?.decision && kernelClosed(ask.decision) !== undefined ? ["Cancelled", "b-off"]
    : ask?.decision && call.status === "pending" ? (row.ended && ask.decision.outcome !== "deny" ? ["No result", "b-off"] : DECIDED[ask.decision.outcome])
    : row.ended ? ["Ended", "b-off"] : STATUS[call.status];
  const main = (
    <>
      {ask ? <div className="wa-input"><div className="wa-input-label">Input</div><InputList input={ask.input} /></div> : null}
      {call.content.map((c, n) =>
        c.type === "diff" ? (
          <React.Fragment key={n}>
            {diffs > 1 || c.path !== path ? <div className="wa-diff-path">{c.path}</div> : null}
            <DiffView path={c.path} oldText={c.oldText} newText={c.newText} />
          </React.Fragment>)
        : c.type === "content" && !pending ? <ProgramOutput key={n} text={c.content.text} />
        : null)}
      {!pending && typeof call.rawOutput === "string" && !call.content.some((c) => c.type === "content") ? <ProgramOutput text={call.rawOutput} /> : null}
    </>
  );
  const body = open ? (
    <>
      <AskScroll>{main}</AskScroll>
      <AskBar key={ask.id} call={call} askId={ask.id} {...(ask.reach !== undefined ? { reach: ask.reach } : {})} actions={send}
        sent={answered} judge={judge} refused={sent?.ask === ask && sent.refused} lineRef={lineRef} focusIn={focusIn} />
    </>
  ) : main;
  const head = (
    <>
      {call.kind === "edit" ? <Pencil /> : null}
      {path ? <><span className="wa-tool-name name">{KIND(call.kind)}</span>{" "}</> : null}
      {path ? <code className="wa-tool-arg">{path}</code> : <span className="wa-tool-arg">{call.title}</span>}{" "}
      <span className={`wa-tool-status badge ${tagCls}`} data-status={call.status}>{tag}</span>
      {row.checking && asking ? <>{" "}<span className="wa-tool-checking badge b-off">Checking</span></> : null}
    </>
  );
  return (
    <section className={`wa-tool card${asking ? " ask" : ""}`} data-call={call.id} data-ask={ask?.id} data-status={call.status} data-asking={asking ? "true" : undefined} aria-label={name}
      onBlur={(e) => { const t = e.target, to = e.relatedTarget;
        // a blur of an element that stays in the page took focus out of the card; a removed one (the pressed control) did not
        if (left.current === false && !(to && e.currentTarget.contains(to))) queueMicrotask(() => { if (t.isConnected && left.current === false) left.current = true; }); }}>
      <ToolCard stateKey={row.key} head={head} open={call.status === "failed" || pending} initial={!!ask}>{body}</ToolCard>
      {row.child !== undefined ? <div className="wa-child"><ChildLink session={row.child} actions={actions} /></div> : null}
    </section>
  );
}

function AskScroll({ children }: { children: React.ReactNode }) {
  const ref = useRef<HTMLDivElement>(null);
  const [more, setMore] = useState(false);
  const measure = useCallback(() => {
    const el = ref.current;
    if (!el) return;
    setMore(el.scrollHeight > el.clientHeight + 1 && el.scrollTop + el.clientHeight < el.scrollHeight - 1);
  }, []);
  useEffect(measure);
  useEffect(() => {
    const el = ref.current;
    if (el && typeof ResizeObserver !== "undefined") {
      const ro = new ResizeObserver(measure);
      ro.observe(el);
      return () => ro.disconnect();
    }
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, [measure]);
  return (
    <div ref={ref} className="wa-ask-scroll" tabIndex={0} role="group" aria-label="Changes and input" onScroll={measure}>
      {children}
      {more ? <div className="wa-more">More below ↓</div> : null}
    </div>
  );
}

function ToolCard({ stateKey, head, open, initial, children }: { stateKey: string; head: React.ReactNode; open: boolean; initial: boolean; children: React.ReactNode }) {
  const [shown, setShown] = useFold(stateKey, open || initial);
  return (
    <>
      <button type="button" className="wa-tool-head chead" aria-expanded={shown || open} onClick={() => setShown(!shown)}>{head}</button>
      {shown || open ? <div className="wa-tool-body">{children}</div> : null}
    </>
  );
}

/** 18.3 `child`: the link that opens the child's transcript; a refusal shows the ordinary error beside it and leaves this session open. */
function ChildLink({ session, actions }: { session: string; actions: Actions }) {
  const [err, setErr] = useState<string | null>(null);
  const reasonId = useId();
  return (
    <>
      <button type="button" className="wa-child-link" aria-describedby={actions.openChild ? undefined : reasonId} aria-disabled={actions.openChild ? undefined : "true"}
        onClick={() => { setErr(null); if (!actions.openChild) return; Promise.resolve().then(() => actions.openChild!(session)).catch((e) => setErr(errorWords(e))); }}>
        Open child session <code>{session}</code>
      </button>
      {actions.openChild ? null : <> <span id={reasonId}>Child sessions cannot be opened from this view.</span></>}
      {err ? <span className="wa-child-err" role="alert">{err}</span> : null}
    </>
  );
}

export function ChildRow({ row, actions }: { row: Of<"child">; actions: Actions }) {
  return <p className="wa-child" data-call={row.toolCallId}><ChildLink session={row.session} actions={actions} /></p>;
}

export function DecisionRow({ row }: { row: Of<"decision"> }) {
  return <p className="wa-decision">{row.who} · {row.what}{row.reason ? ` — ${row.reason}` : ""}</p>;
}

export function FooterRow({ row }: { row: Of<"footer"> }) {
  const duration = row.stopped !== undefined ? `Stopped at the ${row.stopped} after ${row.seconds} s`
    : row.interrupted ? `Stopped after ${row.seconds} s`
    : row.failed ? "Turn failed"
    : `Completed in ${row.seconds} s`;
  const parts = [duration, plural(row.tools, "tool", "tools")];
  if (row.tokens !== undefined) parts.push(`${row.tokens >= 1000 ? `${Math.round(row.tokens / 1000)}k` : row.tokens} tokens`);
  return <p className="wa-footer">{parts.join(" · ")}</p>;
}

export function UnknownRow({ row }: { row: Of<"unknown"> }) {
  const [raw, setRaw] = useFold(row.key, false);
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
    case "child": return <ChildRow row={row} actions={actions} />;
    case "footer": return <FooterRow row={row} />;
    case "unknown": return <UnknownRow row={row} />;
  }
}
