// An agent row's text: prose and fenced code blocks (TECHNICAL 18.2, P14). A block is highlighted once it is complete and near the
// viewport, and once per (language, code); until then it is plain monospace of the same size. Output is React elements, never HTML.
import { useEffect, useRef, useState } from "react";
import { highlight, known, MAX_CODE, peek, type Lines } from "./highlight";

type Seg = { code: false; text: string } | { code: true; lang: string; text: string; complete: boolean };

/** Splits text at ``` fences. An unclosed fence is incomplete while `writing`, complete once the message is finished. */
export function split(text: string, writing: boolean): Seg[] {
  const out: Seg[] = [];
  const lines = text.split("\n");
  let prose: string[] = [];
  const flush = () => { if (prose.length) out.push({ code: false, text: prose.join("\n") }); prose = []; };
  for (let i = 0; i < lines.length; i++) {
    const m = /^ {0,3}```\s*([\w+#.-]*)[^`]*$/.exec(lines[i]);
    if (!m) { prose.push(lines[i]); continue; }
    const body: string[] = [];
    let j = i + 1;
    while (j < lines.length && !/^ {0,3}```\s*$/.test(lines[j])) body.push(lines[j++]);
    flush();
    out.push({ code: true, lang: m[1], text: body.join("\n"), complete: j < lines.length || !writing });
    i = j;
  }
  flush();
  return out;
}

function CodeBlock({ lang, code, complete }: { lang: string; code: string; complete: boolean }) {
  const ref = useRef<HTMLPreElement>(null);
  const [near, setNear] = useState(false);
  const [lines, setLines] = useState<Lines | null>(() => (complete ? peek(lang, code) ?? null : null));
  const can = complete && !!known(lang) && code.length <= MAX_CODE;
  // tokens belong to one (lang, code): when either changes they are dropped at once, the plain text of the new code shows
  const of = useRef({ lang, code });
  if (of.current.lang !== lang || of.current.code !== code) { of.current = { lang, code }; setLines(complete ? peek(lang, code) ?? null : null); }

  useEffect(() => {
    if (!can || near || !ref.current) return;
    if (typeof IntersectionObserver === "undefined") { setNear(true); return; }
    const io = new IntersectionObserver((es) => { if (es.some((e) => e.isIntersecting)) { setNear(true); io.disconnect(); } },
      { rootMargin: `${window.innerHeight || 800}px 0px` });
    io.observe(ref.current);
    return () => io.disconnect();
  }, [can, near]);

  useEffect(() => {
    if (!can || !near) return;
    const hit = peek(lang, code);
    if (hit) { setLines(hit); return; }
    let live = true;
    highlight(lang, code).then((l) => { if (live && l) setLines(l); }, () => {});
    return () => { live = false; };
  }, [can, near, lang, code]);

  return (
    <pre ref={ref} className="wa-code" data-lang={lang || undefined}>
      {lang ? <span className="wa-code-lang">{lang}</span> : null}
      <code>
        {lines
          ? lines.map((line, i) => (
              <span key={i}>{line.map((t, k) => (t.s ? <span key={k} style={t.s}>{t.t}</span> : t.t))}{i < lines.length - 1 ? "\n" : ""}</span>
            ))
          : code}
      </code>
    </pre>
  );
}

export function MessageText({ text, writing }: { text: string; writing: boolean }) {
  const segs = split(text, writing);
  if (segs.every((s) => !s.code)) return <>{text}</>;
  return (
    <>
      {segs.map((s, i) => (s.code ? <CodeBlock key={i} lang={s.lang} code={s.text} complete={s.complete} /> : <span key={i}>{s.text}</span>))}
    </>
  );
}
