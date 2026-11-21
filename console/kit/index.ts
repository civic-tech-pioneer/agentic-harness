// TECHNICAL 18.2: the console's kit, as one entry for the page.
export * from "./model";
export { openAsk, reduce } from "./reduce";
export { decisionJson, openSession } from "./kernel";
export type { Kernel } from "./kernel";
export { FOCUS_END, focusInOpenAsk, releaseOpenAsk, ThreadProvider, Transcript } from "./thread";
export { StatusLine } from "./status";
export { wataneKernel } from "./watane";
export type { Watane } from "./watane";
