// The frame's icons (Main-2): aria-hidden svgs in currentColor.
import type { SVGProps } from "react";

const base = { viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 2, strokeLinecap: "round", strokeLinejoin: "round", "aria-hidden": true, focusable: false } as const;

export function Chevron({ size = 14, ...p }: { size?: number } & SVGProps<SVGSVGElement>) {
  return <svg {...base} width={size} height={size} {...p}><path d="M9 6l6 6-6 6" /></svg>;
}
export function Pencil({ size = 15, ...p }: { size?: number } & SVGProps<SVGSVGElement>) {
  return <svg {...base} width={size} height={size} {...p}><path d="M4 20h4L19 9l-4-4L4 16z" /></svg>;
}
export function Search({ size = 14, ...p }: { size?: number } & SVGProps<SVGSVGElement>) {
  return <svg {...base} width={size} height={size} {...p}><circle cx="11" cy="11" r="6" /><path d="M20 20l-4.5-4.5" /></svg>;
}
