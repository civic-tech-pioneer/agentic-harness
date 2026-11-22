// TECHNICAL 18.2: the page mounts App on the scripted fake kernel (without `window.watane`: tests and `npm run dev`); served by watane it runs on the real kernel (served.ts).
// `?long=N` opens a long session instead of the story (P14: to try the console on a long transcript).
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { Console, Frame } from "./Console";
import { isServed, NO_WATANE_JS, openFailed, served } from "./served";
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
  root.render(<Frame />); // the frame shows at once; the session fills it (no blank page after a reload)
  served(w).then((x) => root.render(<StrictMode><Console served={x} /></StrictMode>),
    (e) => { root.unmount(); openFailed(document.getElementById("root")!, e); });
} else if (isServed(location.hostname)) {
  document.getElementById("root")!.textContent = NO_WATANE_JS; // served: never the fake (18.2)
} else {
  root.render(
    <StrictMode>{n ? <App kernel={longKernel(n)} info={longInfo(n)} sessions={storySessions} /> : <App kernel={scriptedKernel()} info={storyInfo} sessions={storySessions} />}</StrictMode>);
}
