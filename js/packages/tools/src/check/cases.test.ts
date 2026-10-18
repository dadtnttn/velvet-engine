// The stacks case runner: a small case on the example catalog, and the problems it reports for a case that
// does not match the rules, a step it cannot read, or a field that a case does not have.

import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { loadCatalog, type Catalog } from "@velvet/cards";
import { describe, expect, it } from "vitest";
import { HELLO_CARDS } from "../test-support";
import { caseErrorCode, runStacksCase, sameJson } from "./cases";

/** The example's catalog, loaded through velvet-stacks. */
async function exampleCatalog(): Promise<Catalog> {
  return loadCatalog(await readFile(join(HELLO_CARDS, "public", "data", "catalog.json"), "utf8"));
}

/** A case document with the given steps, on the example's catalog. */
function caseOf(steps: readonly unknown[], expect?: unknown): Record<string, unknown> {
  return {
    format: "velvet-stacks",
    version: 1,
    kind: "case",
    name: "test case",
    seed: "7",
    steps,
    ...(expect === undefined ? {} : { expect }),
  };
}

describe("runStacksCase", () => {
  it("plays spawn and drop steps, and checks the table at the end", async () => {
    const catalog = await exampleCatalog();
    const data = caseOf(
      [
        { op: "spawn", def: "villager", at: [0, 0] },
        { op: "spawn", def: "forest", at: [2, 3] },
        { op: "drop", card: 1, onto: 2 },
      ],
      { cards: { villager: 1, forest: 1 }, stacks: [[2, 1]] },
    );
    expect(await runStacksCase("case.json", data, catalog)).toEqual([]);
  });

  it("reports a drop that the rules refuse when the case expects it to work", async () => {
    // A forest accepts only characters: wood dropped on it is refused (the landing card's rule).
    const catalog = await exampleCatalog();
    const data = caseOf([
      { op: "spawn", def: "wood", at: [0, 0] },
      { op: "spawn", def: "forest", at: [2, 0] },
      { op: "drop", card: 1, onto: 2 },
    ]);
    const issues = await runStacksCase("case.json", data, catalog);
    expect(issues).toEqual([expect.objectContaining({ file: "case.json", path: "steps[2]", code: "case-step" })]);
  });

  it("passes a refusal that the case names, and reports one that it does not name", async () => {
    const catalog = await exampleCatalog();
    const steps = [
      { op: "spawn", def: "wood", at: [0, 0] },
      { op: "spawn", def: "forest", at: [2, 0] },
    ];
    expect(await runStacksCase("case.json", caseOf([...steps, { op: "drop", card: 1, onto: 2, error: "DropRefused" }]), catalog)).toEqual([]);
    const wrong = await runStacksCase("case.json", caseOf([...steps, { op: "drop", card: 1, onto: 2, error: "StackFull" }]), catalog);
    expect(wrong).toEqual([expect.objectContaining({ path: "steps[2]", message: expect.stringContaining("drop-refused") })]);
  });

  it("reports a case whose table is not the one it expects, at expect", async () => {
    const catalog = await exampleCatalog();
    const issues = await runStacksCase("case.json", caseOf([{ op: "spawn", def: "villager", at: [0, 0] }], { cards: {} }), catalog);
    expect(issues).toEqual([expect.objectContaining({ path: "expect.cards", code: "case-table" })]);
  });

  it("refuses a document that is not a case, a field that a case does not have, and an unknown op", async () => {
    const catalog = await exampleCatalog();
    expect(await runStacksCase("c.json", { format: "velvet-stacks", version: 1, kind: "catalog" }, catalog)).toEqual([
      expect.objectContaining({ code: "unsupported-document" }),
    ]);
    expect(await runStacksCase("c.json", { ...caseOf([]), colour: "red" }, catalog)).toEqual([
      expect.objectContaining({ path: "colour", code: "unknown-field" }),
    ]);
    expect(await runStacksCase("c.json", caseOf([{ op: "teleport" }]), catalog)).toEqual([
      expect.objectContaining({ path: "steps[0]", code: "case-step" }),
    ]);
  });

  it("reports a seed that is not a string of digits", async () => {
    const catalog = await exampleCatalog();
    expect(await runStacksCase("c.json", { ...caseOf([]), seed: "seven" }, catalog)).toEqual([
      expect.objectContaining({ path: "seed", code: "seed" }),
    ]);
  });
});

describe("the case helpers", () => {
  it("turns an error name into its code, as the fixtures do", () => {
    expect(caseErrorCode("DropRefused")).toBe("drop-refused");
    expect(caseErrorCode("StackFull")).toBe("stack-full");
  });

  it("compares JSON by value, not by the order of keys", () => {
    expect(sameJson({ a: 1, b: [1, { c: 2 }] }, { b: [1, { c: 2 }], a: 1 })).toBe(true);
    expect(sameJson([1, 2], [2, 1])).toBe(false);
    expect(sameJson({ a: 1 }, { a: 1, b: undefined })).toBe(false);
  });
});
