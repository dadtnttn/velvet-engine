// Vite settings for this game.
//
// `base: "./"` makes every built URL relative, so the files load from the custom protocol that Velvet
// Desktop serves (and from a plain folder). A WebAssembly module, if the game adds one, is written as a
// file next to the page: `assetsInlineLimit: 0` stops Vite from inlining it as base64.
//
// The engine's packages are linked from outside this folder, so Vite must be allowed to serve them:
// `server.fs.allow` names the engine's js folder.

import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

// The engine's folder, relative to this file (an absolute path is kept as it is).
const engine = resolve(dirname(fileURLToPath(import.meta.url)), "{{ENGINE_RELATIVE_JSON}}");

export default defineConfig({
  base: "./",
  server: {
    port: 5173,
    strictPort: true,
    fs: { allow: [".", engine] },
  },
  build: {
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
    assetsInlineLimit: 0,
  },
});
