// TECHNICAL 18.3, 18.6: a folded line that opens on click, Enter and Space (a real button), with the frame's chevron.
import { createContext, useContext, useState, type ReactNode } from "react";
import { Chevron } from "./icons";

/** P14: the open or closed state of folds, per row key, held by the transcript so it outlives a row that leaves the window. */
export const FoldStore = createContext<Map<string, boolean> | null>(null);

/** An open/closed state that is kept in the transcript's store under `key` (when there is one) and so survives the row unmounting. */
export function useFold(key: string | undefined, initial: boolean): [boolean, (open: boolean) => void] {
  const store = useContext(FoldStore);
  const [open, setOpen] = useState(() => {
    if (!store || key === undefined) return initial;
    const kept = store.get(key);
    if (kept !== undefined) return kept;
    store.set(key, initial);
    return initial;
  });
  return [open, (v) => { if (store && key !== undefined) store.set(key, v); setOpen(v); }];
}

export function Disclosure(props: { label: string; className: string; defaultOpen?: boolean; icon?: ReactNode; stateKey?: string; children: ReactNode }) {
  const [open, setOpen] = useFold(props.stateKey, props.defaultOpen ?? false);
  return (
    <div className={props.className} data-open={open ? "true" : "false"}>
      <button type="button" className={`wa-fold foldbtn${open ? " open" : ""}`} aria-expanded={open} onClick={() => setOpen(!open)}>
        <Chevron className="chev" />{props.icon}{props.label}
      </button>
      {open ? <div className="wa-fold-body">{props.children}</div> : null}
    </div>
  );
}
