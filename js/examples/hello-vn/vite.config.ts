// Vite settings for hello-vn.
//
// `base: "./"` makes every built URL relative, so the files load from the custom protocol that
// Velvet Desktop serves (and from a plain folder). The WebAssembly module is written as a file
// next to the page: `assetsInlineLimit: 0` stops Vite from inlining it as base64.

import { defineConfig } from "vite";

export default defineConfig({
  base: "./",
  server: {
    port: 5174,
    strictPort: true,
  },
  build: {
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
    assetsInlineLimit: 0,
  },
});
