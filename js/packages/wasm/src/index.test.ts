// Tests for the loader: the module loads once and returns its exports, and a failed load says
// what to do, then can be retried. The tests run in Node, where the loader reads the file from disk.

import { describe, expect, it, vi } from "vitest";
import { loadVelvetWasm } from "./index";

describe("loadVelvetWasm", () => {
  it("loads the module once and returns the same exports every time", async () => {
    const first = await loadVelvetWasm();
    expect(typeof first.parseGuion).toBe("function");
    expect(typeof first.GuionLibrary).toBe("function");
    expect(await loadVelvetWasm()).toBe(first);
  });

  it("gives functions that answer with JSON text", async () => {
    const wasm = await loadVelvetWasm();
    const reply = JSON.parse(wasm.parseGuion("## inicio\n", "saludo.md", "{}")) as { id: string };
    expect(reply.id).toBe("saludo");
  });

  it("starts from a URL of the file, which Node reads from disk", async () => {
    vi.resetModules();
    const fresh = await import("./index");
    const location = new URL("../gen/velvet_wasm_bg.wasm", import.meta.url);
    const wasm = await fresh.loadVelvetWasm(location);
    expect(typeof wasm.parseGuion).toBe("function");
  });

  it("reports a clear error for bytes that are not a module, and retries on the next call", async () => {
    vi.resetModules();
    const fresh = await import("./index");
    const failure: unknown = await fresh
      .loadVelvetWasm(new Uint8Array([0, 1, 2, 3]))
      .then(
        () => undefined,
        (caught: unknown) => caught,
      );
    expect(failure).toBeInstanceOf(fresh.WasmLoadError);
    expect((failure as Error).message).toContain("pnpm build:wasm");
    const retried = await fresh.loadVelvetWasm();
    expect(typeof retried.parseGuion).toBe("function");
  });
});
