// Vitest configuration for the whole workspace.
//
// Tests sit next to the code they cover. Discovery covers every package and example:
// packages/<name>/src and examples/<name>/src, files ending in .test.ts.

import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["packages/*/src/**/*.test.ts", "examples/*/src/**/*.test.ts"],
    // Builds the WebAssembly module when it is missing or older than the Rust sources.
    globalSetup: ["./packages/wasm/test-setup.ts"],
  },
});
