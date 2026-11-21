// TECHNICAL 18.7: jest-dom matchers and cleanup between component tests (DOM tests only; node-environment files skip it).
import { afterEach } from "vitest";

if (typeof document !== "undefined") {
  await import("@testing-library/jest-dom/vitest");
  const { cleanup } = await import("@testing-library/react");
  afterEach(() => cleanup());
}
