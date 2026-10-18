// The bin file, run as a program the way `pnpm velvet-js` runs it: the resolver hook must let Node load the
// packages' TypeScript without a build step.

import { execFile } from "node:child_process";
import { join } from "node:path";
import { promisify } from "node:util";
import { describe, expect, it } from "vitest";
import { ENGINE_JS_DIR, TOOLS_DIR } from "./paths";
import { HELLO_CARDS } from "./test-support";

const run = promisify(execFile);

/** The bin file of this package. */
const BIN = join(TOOLS_DIR, "bin", "velvet-js.mjs");

/**
 * Runs the bin with the arguments, from the engine's js folder, and returns its exit code and output.
 * @param args - the command-line arguments.
 */
async function runBin(args: readonly string[]): Promise<{ code: number; stdout: string }> {
  try {
    const { stdout } = await run(process.execPath, [BIN, ...args], { cwd: ENGINE_JS_DIR, maxBuffer: 8 * 1024 * 1024 });
    return { code: 0, stdout };
  } catch (error) {
    const failure = error as { code?: number; stdout?: string };
    return { code: typeof failure.code === "number" ? failure.code : 1, stdout: failure.stdout ?? "" };
  }
}

describe("the velvet-js bin", () => {
  it("runs the program help from TypeScript source, with no build step", async () => {
    const result = await runBin(["--help"]);
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("Commands:");
  });

  it("checks the example game, and exits 0", async () => {
    const result = await runBin(["check", HELLO_CARDS]);
    expect(result.code).toBe(0);
    expect(result.stdout).toMatch(/^hello-cards: ok, 0 errors, \d+ warnings$/mu);
  });
});
