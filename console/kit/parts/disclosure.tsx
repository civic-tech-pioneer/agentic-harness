// TECHNICAL 18.3, 18.6: a folded line that opens on click, Enter and Space (a real button), with the frame's chevron.
import { useState, type ReactNode } from "react";
import { Chevron } from "./icons";

export function Disclosure(props: { label: string; className: string; defaultOpen?: boolean; icon?: ReactNode; children: ReactNode }) {
  const [open, setOpen] = useState(props.defaultOpen ?? false);
  return (
    <div className={props.className} data-open={open ? "true" : "false"}>
      <button type="button" className={`wa-fold foldbtn${open ? " open" : ""}`} aria-expanded={open} onClick={() => setOpen(!open)}>
        <Chevron className="chev" />{props.icon}{props.label}
      </button>
      {open ? <div className="wa-fold-body">{props.children}</div> : null}
    </div>
  );
}
