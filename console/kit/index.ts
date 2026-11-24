// TECHNICAL 18.2: the console's kit, as one entry for the page.
export * from "./model";
export { openAsk, reduce } from "./reduce";
export { decisionJson, epochOf, openSession } from "./kernel";
export type { Kernel, ProfileInfo, Refusal } from "./kernel";
export { FOCUS_END, focusInOpenAsk, releaseOpenAsk, ThreadProvider, Transcript } from "./thread";
export { StatusLine } from "./status";
export { wataneKernel } from "./watane";
export type { Watane } from "./watane";
export { errorWords } from "./errors";
