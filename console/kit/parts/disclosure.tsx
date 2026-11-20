// TECHNICAL 18.3, 18.6: a folded line that opens on click, Enter and Space (a real button).
import { useState, type ReactNode } from "react";

export function Disclosure(props: { label: string; className: string; defaultOpen?: boolean; children: ReactNode }) {
  const [open, setOpen] = useState(props.defaultOpen ?? false);
  return (
    <div className={props.className} data-open={open ? "true" : "false"}>
      <button type="button" className="wa-fold" aria-expanded={open} onClick={() => setOpen(!open)}>{props.label}</button>
      {open ? <div className="wa-fold-body">{props.children}</div> : null}
    </div>
  );
}
