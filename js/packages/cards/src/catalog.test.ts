// Tests of loadCatalog: the shared catalog in its normal form, and every refusal of the engine
// with the code that names the rule that broke.

import { describe, expect, it } from "vitest";
import { loadCatalog, type CardsErrorCode } from "./index";
import { expectRejection, SHARED_CATALOG } from "./test-support";

/** A catalog document with the given cards and recipes, under the velvet-stacks envelope. */
function document(cards: readonly unknown[], recipes: readonly unknown[] = []): string {
  return JSON.stringify({ format: "velvet-stacks", version: 1, kind: "catalog", cards, recipes });
}

/** A recipe that makes wood from wood, with the fields that have defaults left out. */
function recipe(overrides: Readonly<Record<string, unknown>>): Record<string, unknown> {
  return {
    id: "make_wood",
    ingredients: [{ select: { id: "wood" } }],
    seconds: 1,
    outputs: [{ card: "wood" }],
    ...overrides,
  };
}

const WOOD = { id: "wood", name: "card.wood", kind: "resource" };

describe("loadCatalog", () => {
  it("returns the shared catalog in its normal form, with defaults written out", async () => {
    const catalog = await loadCatalog(SHARED_CATALOG);
    expect(catalog.format).toBe("velvet-stacks");
    expect(catalog.kind).toBe("catalog");
    expect(catalog.recipes[0]?.id, "the highest priority runs first").toBe("gather_berries_bonus");
    expect(catalog.recipes[1]?.id, "then the recipe with the most cards").toBe("build_market");
    const wood = catalog.cards.find((card) => card.id === "wood");
    expect(wood?.stack).toEqual({ accepts: [], max: 30 });
    expect(wood?.tags).toEqual(["material", "wood"]);
  });

  it("writes its normal form back unchanged", async () => {
    const catalog = await loadCatalog(SHARED_CATALOG);
    expect(await loadCatalog(JSON.stringify(catalog))).toEqual(catalog);
  });

  it("refuses text that is not a catalog", async () => {
    await expectRejection(loadCatalog("[1]"), "bad-input");
    await expectRejection(loadCatalog("{"), "bad-input");
  });

  it("refuses a document of another kind or version", async () => {
    const board = '{"format":"velvet-stacks","version":1,"kind":"board","cards":[]}';
    await expectRejection(loadCatalog(board), "unsupported-document");
    const newer = '{"format":"velvet-stacks","version":2,"kind":"catalog","cards":[]}';
    await expectRejection(loadCatalog(newer), "unsupported-document");
  });

  const broken: ReadonlyArray<readonly [string, string, CardsErrorCode]> = [
    ["a card id that appears twice", document([WOOD, WOOD]), "duplicate-card"],
    ["an empty card id", document([{ ...WOOD, id: "" }]), "empty-card-id"],
    [
      "a stack limit of zero",
      document([{ ...WOOD, stack: { accepts: [], max: 0 } }]),
      "zero-max",
    ],
    [
      "a stack rule that names no card",
      document([{ ...WOOD, stack: { accepts: [{ id: "ghost" }], max: 3 } }]),
      "unknown-card",
    ],
    [
      "an ingredient that names no card",
      document([WOOD], [recipe({ ingredients: [{ select: { id: "ghost" } }] })]),
      "unknown-card",
    ],
    [
      "an output that names no card",
      document([WOOD], [recipe({ outputs: [{ card: "ghost" }] })]),
      "unknown-card",
    ],
    ["two recipes with one id", document([WOOD], [recipe({}), recipe({})]), "duplicate-recipe"],
    [
      "a recipe with no ingredients",
      document([WOOD], [recipe({ ingredients: [] })]),
      "no-ingredients",
    ],
    [
      "an ingredient that asks for zero cards",
      document([WOOD], [recipe({ ingredients: [{ select: { id: "wood" }, count: 0 }] })]),
      "zero-count",
    ],
    ["a duration below zero", document([WOOD], [recipe({ seconds: -1 })]), "bad-seconds"],
    [
      "an output chance above one",
      document([WOOD], [recipe({ outputs: [{ card: "wood", chance: 1.5 }] })]),
      "bad-chance",
    ],
  ];

  for (const [label, text, code] of broken) {
    it(`refuses ${label} with the code ${code}`, async () => {
      await expectRejection(loadCatalog(text), code);
    });
  }
});
