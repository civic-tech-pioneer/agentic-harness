// TECHNICAL 18.2, 18.3: the rows inside assistant-ui's thread, fed by its External-store runtime.
import { createContext, useContext, useMemo, type ReactNode } from "react";
import { AssistantRuntimeProvider, ThreadPrimitive, useExternalStoreRuntime, type ThreadMessageLike } from "@assistant-ui/react";
import type { Actions, Row, SessionView } from "./model";
import { RowView } from "./parts/rows";

function convert(row: Row): ThreadMessageLike {
  const text = row.kind === "user" || row.kind === "agent" || row.kind === "thought" ? row.text : "";
  return { id: row.key, role: row.kind === "user" ? "user" : "assistant", content: [{ type: "text", text }] };
}

const Rows = createContext<Map<string, Row>>(new Map());

export function ThreadProvider(props: { view: SessionView; actions: Actions; children: ReactNode }) {
  const { view, actions } = props;
  const runtime = useExternalStoreRuntime<Row>({
    messages: view.rows,
    convertMessage: convert,
    isRunning: view.turn?.state === "running",
    onNew: async (m) => {
      const t = m.content.map((p) => (p.type === "text" ? p.text : "")).join("");
      if (t) actions.say(t);
    },
  });
  const rows = useMemo(() => new Map(view.rows.map((r) => [r.key, r])), [view.rows]);
  return (
    <AssistantRuntimeProvider runtime={runtime}>
      <Rows.Provider value={rows}>{props.children}</Rows.Provider>
    </AssistantRuntimeProvider>
  );
}

export function Transcript(props: { view: SessionView; actions: Actions }) {
  const rows = useContext(Rows);
  return (
    <ThreadPrimitive.Root className="wa-transcript">
      <ThreadPrimitive.Messages>
        {({ message }) => {
          const row = rows.get(message.id);
          return row ? <div className="wa-row" data-kind={row.kind}><RowView row={row} actions={props.actions} /></div> : null;
        }}
      </ThreadPrimitive.Messages>
    </ThreadPrimitive.Root>
  );
}
