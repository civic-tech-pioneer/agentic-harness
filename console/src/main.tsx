// TECHNICAL 18.2: the page mounts App on the scripted fake kernel (no real kernel yet: dev and build alike).
// `?long=N` opens a long session instead of the story (P14: to try the console on a long transcript).
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { longInfo, longKernel, scriptedKernel, storyInfo, storySessions } from "../fake/page";
import "./theme.css";
import "../kit/styles/kit.css";

const long = Number(new URLSearchParams(location.search).get("long"));
const n = Number.isInteger(long) && long > 0 && long <= 100_000 ? long : 0;

createRoot(document.getElementById("root")!).render(
  <StrictMode>{n ? <App kernel={longKernel(n)} info={longInfo(n)} sessions={storySessions} /> : <App kernel={scriptedKernel()} info={storyInfo} sessions={storySessions} />}</StrictMode>);
