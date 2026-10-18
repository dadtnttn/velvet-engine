// Vitest global setup: builds the WebAssembly module when it is missing or older than the Rust
// sources it comes from (the rule lives in build.mjs), so `pnpm test` works on a fresh checkout.
// Vitest runs it once per test run, before any test file. A failed build fails the run.

// @ts-expect-error build.mjs is plain JavaScript without typings.
import { build, needsBuild } from "./build.mjs";

/** Vitest's global setup: runs the build when needed, and fails the run if the build fails. */
export function setup(): void {
  if (!(needsBuild as () => boolean)()) return;
  console.log("velvet-wasm is missing or out of date: building it (pnpm build:wasm)");
  (build as () => void)();
}
