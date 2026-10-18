// @vitest-environment happy-dom
// The loader where a test environment has replaced the global URL class (happy-dom does). The
// default source is built with `new URL(...)` from that class, and Node must still read the file
// from disk, whatever URL implementation made the address.

import { URL as NodeURL } from "node:url";
import { describe, expect, it } from "vitest";
import { loadVelvetWasm } from "./index";

describe("loadVelvetWasm under happy-dom", () => {
  it("runs where the global URL class is not Node's", () => {
    expect(globalThis.URL).not.toBe(NodeURL);
  });

  it("loads the module from its default source, which Node reads from disk", async () => {
    const wasm = await loadVelvetWasm();
    const reply = JSON.parse(wasm.parseGuion("## inicio\n", "saludo.md", "{}")) as { id: string };
    expect(reply.id).toBe("saludo");
  });
});
