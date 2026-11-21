// TECHNICAL 18.2: the page mounts App on the scripted fake kernel (without `window.watane`: tests and `npm run dev`); served by watane it runs on the real kernel (served.ts).
// `?long=N` opens a long session instead of the story (P14: to try the console on a long transcript).
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { served } from "./served";
import type { Watane } from "../kit";
import { longInfo, longKernel, scriptedKernel, storyInfo, storySessions } from "../fake/page";
import "./theme.css";
import "../kit/styles/kit.css";

const long = Number(new URLSearchParams(location.search).get("long"));
const n = Number.isInteger(long) && long > 0 && long <= 100_000 ? long : 0;

const root = createRoot(document.getElementById("root")!);
const w = (window as unknown as { watane?: Watane }).watane;
if (w) {
  // served by watane: the real kernel; a failure shows as text, not as the fake
  served(w).then((x) => root.render(<StrictMode><App kernel={x.kernel} info={x.info} sessions={x.sessions} /></StrictMode>),
    (e) => { document.getElementById("root")!.textContent = `The console could not open a session: ${e instanceof Error ? e.message : String(e)}`; });
} else {
  root.render(
    <StrictMode>{n ? <App kernel={longKernel(n)} info={longInfo(n)} sessions={storySessions} /> : <App kernel={scriptedKernel()} info={storyInfo} sessions={storySessions} />}</StrictMode>);
}
