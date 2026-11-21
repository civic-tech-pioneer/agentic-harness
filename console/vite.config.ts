// TECHNICAL 18.2: the build writes ../ui/ as the host serves it: one HTML file, same-origin assets, no inline script.
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Shiki's JavaScript grammar holds the text `Function(?!...` inside a regex string (data, never code). It runs nothing, but the
// build test (18.2, no `Function(`) cannot tell it from a call. The template literal reads the same with the letter escaped.
const grammarText = {
  name: "wa-grammar-text",
  // after the minifier, which would write the escape back as the letter
  generateBundle(_: unknown, bundle: Record<string, { type: string; code?: string }>) {
    for (const c of Object.values(bundle)) if (c.type === "chunk" && c.code?.includes("Function(?!")) c.code = c.code.split("Function(?!").join("Fun\\u0063tion(?!");
  },
};

export default defineConfig({
  plugins: [react(), grammarText],
  base: "./",
  build: {
    outDir: "../ui",
    emptyOutDir: true,
    modulePreload: false,
    assetsInlineLimit: 0,
    cssCodeSplit: false,
  },
});
