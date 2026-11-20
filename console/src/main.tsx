// TECHNICAL 18.2: the page mounts App on the scripted fake kernel (no real kernel yet: dev and build alike).
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { scriptedKernel, storyInfo, storySessions } from "../fake/page";
import "./theme.css";
import "../kit/styles/kit.css";

createRoot(document.getElementById("root")!).render(<StrictMode><App kernel={scriptedKernel()} info={storyInfo} sessions={storySessions} /></StrictMode>);
