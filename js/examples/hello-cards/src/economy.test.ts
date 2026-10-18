// Tests of the card rules as the game plays them: the catalog reads, a villager gathers without using
// anything up, and a craft uses its ingredients.

import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import { Board, loadCatalog, type BoardEvent, type Catalog } from "@velvet/cards";

/** The game's catalog, read from its file and checked by the engine. */
async function gameCatalog(): Promise<Catalog> {
  return loadCatalog(await readFile(new URL("../public/data/catalog.json", import.meta.url), "utf8"));
}

/** How many cards of a definition lie on the table. */
function countOf(board: Board, def: string): number {
  return board.cards().filter((card) => card.def === def).length;
}

/** The events of one kind. */
function eventsNamed(events: readonly BoardEvent[], name: BoardEvent["event"]): BoardEvent[] {
  return events.filter((event) => event.event === name);
}

describe("the catalog", () => {
  it("has between 8 and 12 recipes, and every card that a recipe makes", async () => {
    const catalog = await gameCatalog();
    expect(catalog.recipes.length).toBeGreaterThanOrEqual(8);
    expect(catalog.recipes.length).toBeLessThanOrEqual(12);
    const ids = new Set(catalog.cards.map((card) => card.id));
    for (const recipe of catalog.recipes) {
      for (const output of recipe.outputs) expect(ids.has(output.card)).toBe(true);
    }
  });
});

describe("gathering", () => {
  it("gives a villager on a forest wood after the recipe's seconds, and uses up nothing", async () => {
    const catalog = await gameCatalog();
    const recipe = catalog.recipes.find((candidate) => candidate.id === "gather_wood");
    expect(recipe?.seconds).toBe(2);
    const seconds = recipe?.seconds ?? Number.NaN;

    const board = await Board.create(catalog, 1);
    const forest = board.spawn("forest", 0, 0);
    const villager = board.spawn("villager", 0, 1);
    board.dropOn(villager, forest);

    const started = board.tick(0);
    expect(started).toContainEqual({ event: "job_started", stack: expect.any(Number), recipe: "gather_wood" });

    expect(board.tick(seconds - 0.5)).toEqual([]);
    expect(countOf(board, "wood")).toBe(0);

    const finished = board.tick(0.5);
    expect(eventsNamed(finished, "job_finished")).toHaveLength(1);
    expect(countOf(board, "wood")).toBe(1);
    expect(countOf(board, "forest")).toBe(1);
    expect(countOf(board, "villager")).toBe(1);

    // The job keeps running: the next round makes a second piece of wood.
    board.tick(seconds);
    expect(countOf(board, "wood")).toBe(2);
    board.free();
  });
});

describe("crafting", () => {
  it("makes a plank from two pieces of wood at a villager's bench, and uses the wood up", async () => {
    const board = await Board.create(await gameCatalog(), 3);
    const first = board.spawn("wood", 0, 0);
    const second = board.spawn("wood", 0, 2);
    const villager = board.spawn("villager", 0, 4);
    board.dropOn(second, first);
    board.dropOn(villager, first);
    board.tick(0);

    const finished = board.tick(3);
    expect(eventsNamed(finished, "job_finished")).toEqual([
      { event: "job_finished", stack: expect.any(Number), recipe: "make_planks" },
    ]);
    expect(eventsNamed(finished, "card_consumed")).toHaveLength(2);
    expect(countOf(board, "plank")).toBe(1);
    expect(countOf(board, "wood")).toBe(0);
    expect(countOf(board, "villager")).toBe(1);
    board.free();
  });
});
