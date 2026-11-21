// TECHNICAL 18.7: the console's component test run (jsdom for the view tests of later slices).
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    environmentMatchGlobs: [["{kit/reduce,src/layout,test/build}.test.ts", "node"]], // DOM-free files skip jsdom
    setupFiles: ["kit/test-setup.ts"],
    include: ["kit/**/*.test.{ts,tsx}", "fake/**/*.test.{ts,tsx}", "src/**/*.test.{ts,tsx}", "test/**/*.test.{ts,tsx}"],
  },
});
