// TECHNICAL 18.2: program output as escaped text; ANSI sequences are stripped, nothing is injected as HTML.
import Anser from "anser";

export function plainOutput(text: string): string {
  return Anser.ansiToText(text).replace(/\x1b/g, "");
}

export function ProgramOutput({ text }: { text: string }) {
  return <pre className="wa-output">{plainOutput(text)}</pre>;
}
