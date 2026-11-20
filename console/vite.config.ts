// TECHNICAL 18.2: the build writes ../ui/ as the host serves it: one HTML file, same-origin assets, no inline script.
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  base: "./",
  build: {
    outDir: "../ui",
    emptyOutDir: true,
    modulePreload: false,
    assetsInlineLimit: 0,
    cssCodeSplit: false,
  },
});
