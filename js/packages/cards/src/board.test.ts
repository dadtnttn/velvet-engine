// Tests of the Board API that games rely on: seeds, ids, refusals, events, snapshots, saves and
// disposal. The shared fixtures (fixtures.test.ts) check the rules step by step; these tests check
// the boundary around them.

import { describe, expect, it } from "vitest";
import {
  Board,
  loadCatalog,
  type BoardEvent,
  type BoardSave,
  type Catalog,
  type Seed,
} from "./index";
import { expectFailure, expectRejection, SHARED_CATALOG } from "./test-support";

/** The shared catalog, read and checked. */
async function shared(): Promise<Catalog> {
  return loadCatalog(SHARED_CATALOG);
}

/** Sells two berries at a market with one roll of the seed, and returns how many coins it made. */
async function coinsFromOneSale(catalog: Catalog, seed: Seed): Promise<number> {
  const board = await Board.create(catalog, seed);
  const market = board.spawn("market", 0, 0);
  board.dropOn(board.spawn("berries", 0, 0), market);
  board.dropOn(board.spawn("berries", 0, 0), market);
  const coins = board
    .tick(1)
    .filter((event) => event.event === "card_spawned" && event.def === "coin");
  board.free();
  return coins.length;
}

/** Plays six rounds of selling on a board with `seed`, and returns every event it reported. */
async function playRounds(catalog: Catalog, seed: Seed): Promise<BoardEvent[]> {
  const board = await Board.create(catalog, seed);
  const events: BoardEvent[] = [];
  for (let round = 0; round < 6; round += 1) {
    const market = board.spawn("market", 0, 0);
    board.dropOn(board.spawn("berries", 0, 0), market);
    board.dropOn(board.spawn("berries", 0, 0), market);
    events.push(...board.tick(1));
    board.dropOn(board.spawn("berries", 0, 0), market);
    board.remove(market);
  }
  board.free();
  return events;
}

describe("seeds", () => {
  it("refuses a seed that is not a whole number of 64 bits", async () => {
    const catalog = await shared();
    const bad: Seed[] = [-1, 1.5, 2 ** 53, Number.NaN, "", "-5", "1e3", 2n ** 64n];
    for (const seed of bad) {
      await expectRejection(Board.create(catalog, seed), "bad-input");
    }
  });

  it("takes a number, a bigint or the digits of one seed alike", async () => {
    const catalog = await shared();
    const asNumber = await playRounds(catalog, 42);
    expect(await playRounds(catalog, 42n)).toEqual(asNumber);
    expect(await playRounds(catalog, "42")).toEqual(asNumber);
  });

  it("keeps all 64 bits of a bigint or digit seed", async () => {
    const catalog = await shared();
    const top = 2n ** 64n - 1n;
    expect(await playRounds(catalog, top)).toEqual(await playRounds(catalog, top.toString()));
    expect(await coinsFromOneSale(catalog, top)).toBe(await coinsFromOneSale(catalog, "18446744073709551615"));
  });

  it("reads the reference rolls of STACKS.md: seed 1 makes the coin, seed 42 does not", async () => {
    const catalog = await shared();
    expect(await coinsFromOneSale(catalog, 1)).toBe(1);
    expect(await coinsFromOneSale(catalog, 42)).toBe(0);
  });
});

describe("ids", () => {
  it("refuses ids that are not whole numbers from 1 up to 2^53 - 1", async () => {
    const board = await Board.create(await shared(), 1);
    expectFailure(() => board.progress(0), "bad-id");
    expectFailure(() => board.progress(1.5), "bad-id");
    expectFailure(() => board.progress(-3), "bad-id");
    expectFailure(() => board.progress(Number.NaN), "bad-id");
    expectFailure(() => board.progress(2 ** 53), "bad-id");
    expectFailure(() => board.dropOn(1, 2 ** 53), "bad-id");
    expectFailure(() => board.remove(0), "bad-id");
    board.free();
  });

  it("accepts the largest safe id, which names no card here", async () => {
    const board = await Board.create(await shared(), 1);
    expect(board.progress(Number.MAX_SAFE_INTEGER)).toBeNull();
    board.free();
  });

  it("reports an id that is not on the table as no-such-card", async () => {
    const board = await Board.create(await shared(), 1);
    board.spawn("wood", 0, 0);
    expectFailure(() => board.remove(99), "no-such-card");
    expectFailure(() => board.split(7), "no-such-card");
    board.free();
  });
});

describe("refusals", () => {
  it("names a card that the catalog does not have", async () => {
    const board = await Board.create(await shared(), 1);
    expectFailure(() => board.spawn("ghost", 0, 0), "unknown-card");
    board.free();
  });

  it("refuses a position that is not a finite number", async () => {
    const board = await Board.create(await shared(), 1);
    expectFailure(() => board.spawn("wood", Number.NaN, 0), "bad-input");
    expectFailure(() => board.spawn("wood", 0, Number.POSITIVE_INFINITY), "bad-input");
    board.free();
  });

  it("refuses a drop that the top card does not accept, and queues nothing", async () => {
    const board = await Board.create(await shared(), 1);
    const forest = board.spawn("forest", 0, 0);
    const wood = board.spawn("wood", 0, 0);
    board.takeEvents();
    expectFailure(() => board.dropOn(wood, forest), "drop-refused");
    expect(board.takeEvents()).toEqual([]);
    expect(board.stacks().map((stack) => stack.cards)).toEqual([[forest], [wood]]);
    board.free();
  });

  it("refuses a drop that would make a stack taller than its limit", async () => {
    const board = await Board.create(await shared(), 1);
    const house = board.spawn("house", 0, 0);
    const wood = board.spawn("wood", 0, 0);
    expectFailure(() => board.dropOn(wood, house), "stack-full");
    board.free();
  });

  it("refuses a drop onto the card's own stack", async () => {
    const board = await Board.create(await shared(), 1);
    const wood = board.spawn("wood", 0, 0);
    expectFailure(() => board.dropOn(wood, wood), "same-stack");
    board.free();
  });
});

describe("events, moves and snapshots", () => {
  it("returns the events of actions from takeEvents, and tick returns them first", async () => {
    const board = await Board.create(await shared(), 1);
    const forest = board.spawn("forest", 0, 0);
    const worker = board.spawn("worker", 0, 0);
    expect(board.takeEvents()).toEqual([
      { event: "card_spawned", card: forest, def: "forest", x: 0, y: 0 },
      { event: "stack_changed", stack: forest, cards: [forest] },
      { event: "card_spawned", card: worker, def: "worker", x: 0, y: 0 },
      { event: "stack_changed", stack: worker, cards: [worker] },
    ]);
    board.dropOn(worker, forest);
    expect(board.tick(0)).toEqual([
      { event: "stack_changed", stack: worker, cards: [] },
      { event: "stack_changed", stack: forest, cards: [forest, worker] },
      { event: "job_started", stack: forest, recipe: "gather_wood" },
    ]);
    board.free();
  });

  it("counts a negative or non-finite dt as no time", async () => {
    const board = await Board.create(await shared(), 1);
    const forest = board.spawn("forest", 0, 0);
    board.dropOn(board.spawn("worker", 0, 0), forest);
    board.tick(1);
    expect(board.tick(-4)).toEqual([]);
    expect(board.tick(Number.NaN)).toEqual([]);
    expect(board.progress(forest)).toBe(0.5);
    board.free();
  });

  it("moves a whole stack without queueing events, and never changes its job", async () => {
    const board = await Board.create(await shared(), 1);
    const forest = board.spawn("forest", 0, 0);
    const worker = board.spawn("worker", 0, 0);
    board.dropOn(worker, forest);
    board.takeEvents();
    board.moveStack(worker, 9, -1);
    expect(board.takeEvents()).toEqual([]);
    expect(board.cards().map((card) => [card.x, card.y])).toEqual([
      [9, -1],
      [9, -1],
    ]);
    expect(board.runningRecipe(forest)).toBe("gather_wood");
    board.free();
  });

  it("lists cards and stacks by id, with the position and the job of each stack", async () => {
    const board = await Board.create(await shared(), 1);
    const forest = board.spawn("forest", 5, 5);
    const worker = board.spawn("worker", 0, 0);
    board.dropOn(worker, forest);
    board.tick(1);
    expect(board.cards()).toEqual([
      { id: forest, def: "forest", x: 5, y: 5 },
      { id: worker, def: "worker", x: 5, y: 5 },
    ]);
    expect(board.stacks()).toEqual([
      {
        id: forest,
        cards: [forest, worker],
        x: 5,
        y: 5,
        job: { recipe: "gather_wood", progress: 0.5 },
      },
    ]);
    const idle = board.spawn("wood", 0, 0);
    expect(board.stacks().map((stack) => stack.job)).toEqual([
      { recipe: "gather_wood", progress: 0.5 },
      null,
    ]);
    expect(board.progress(idle)).toBeNull();
    expect(board.runningRecipe(idle)).toBeNull();
    board.free();
  });
});

describe("split", () => {
  /** A worker with a piece of wood on it, at (x, y): two cards that start no job. */
  async function workerWithWood(
    x: number,
    y: number,
  ): Promise<{ board: Board; worker: number; wood: number }> {
    const board = await Board.create(await shared(), 1);
    const worker = board.spawn("worker", x, y);
    const wood = board.spawn("wood", x, y);
    board.dropOn(wood, worker);
    board.takeEvents();
    return { board, worker, wood };
  }

  it("leaves the new stack where the old one is, on top of it, by default", async () => {
    const { board, worker, wood } = await workerWithWood(4, -2);
    const stack = board.split(wood);
    expect(board.stacks().map(({ cards, x, y }) => ({ cards, x, y }))).toEqual([
      { cards: [worker], x: 4, y: -2 },
      { cards: [wood], x: 4, y: -2 },
    ]);
    expect(board.stacks().find((view) => view.cards.includes(wood))?.id).toBe(stack);
    board.free();
  });

  it("moves the new stack to the position given, and leaves the old stack in place", async () => {
    const { board, worker, wood } = await workerWithWood(4, -2);
    const stack = board.split(wood, { x: 9, y: 1.5 });
    expect(board.stacks().find((view) => view.id === stack)).toMatchObject({
      cards: [wood],
      x: 9,
      y: 1.5,
    });
    expect(board.stacks().find((view) => view.cards.includes(worker))).toMatchObject({
      x: 4,
      y: -2,
    });
    board.free();
  });

  it("queues the same events with a position as without one", async () => {
    const plain = await workerWithWood(4, -2);
    plain.board.split(plain.wood);
    const placed = await workerWithWood(4, -2);
    placed.board.split(placed.wood, { x: 9, y: 1.5 });
    expect(placed.board.takeEvents()).toEqual(plain.board.takeEvents());
    plain.board.free();
    placed.board.free();
  });

  it("moves the whole stack when the bottom card is split with a position", async () => {
    const { board, worker, wood } = await workerWithWood(4, -2);
    const stack = board.split(worker, { x: 0, y: 0 });
    expect(board.stacks()).toEqual([
      expect.objectContaining({ id: stack, cards: [worker, wood], x: 0, y: 0 }),
    ]);
    board.free();
  });

  it("refuses a position that is not two finite numbers, and changes nothing", async () => {
    const { board, worker, wood } = await workerWithWood(4, -2);
    expectFailure(() => board.split(wood, { x: Number.NaN, y: 0 }), "bad-input");
    expectFailure(() => board.split(wood, { x: 0, y: Number.POSITIVE_INFINITY }), "bad-input");
    // Larger than a binary32 number, so the module would hold it as infinity.
    expectFailure(() => board.split(wood, { x: 0, y: 1e39 }), "bad-input");
    expect(board.stacks().map((view) => view.cards)).toEqual([[worker, wood]]);
    expect(board.takeEvents()).toEqual([]);
    board.free();
  });

  it("refuses a card that is not on the table, and moves nothing", async () => {
    const { board, worker, wood } = await workerWithWood(4, -2);
    expectFailure(() => board.split(99, { x: 0, y: 0 }), "no-such-card");
    expect(board.stacks().map((view) => view.cards)).toEqual([[worker, wood]]);
    board.free();
  });
});

describe("saves", () => {
  it("continues a job that a save holds, with the same events later", async () => {
    const catalog = await shared();
    const board = await Board.create(catalog, 3);
    const forest = board.spawn("forest", 0, 0);
    board.dropOn(board.spawn("worker", 0, 0), forest);
    board.tick(1);
    const save = board.save();
    const restored = await Board.load(catalog, JSON.parse(JSON.stringify(save)) as BoardSave);
    expect(restored.save()).toEqual(save);
    expect(restored.tick(1.5)).toEqual(board.tick(1.5));
    expect(restored.progress(forest)).toBe(board.progress(forest));
    board.free();
    restored.free();
  });

  it("refuses a save that names a card the catalog does not have", async () => {
    const catalog = await shared();
    const board = await Board.create(catalog, 1);
    board.spawn("wood", 0, 0);
    const save = board.save();
    const named = { ...save, cards: save.cards.map((card) => ({ ...card, def: "ghost" })) };
    await expectRejection(Board.load(catalog, named), "unknown-card");
    board.free();
  });

  it("refuses a save that puts a card in two stacks", async () => {
    const catalog = await shared();
    const board = await Board.create(catalog, 1);
    board.spawn("wood", 0, 0);
    board.spawn("stone", 0, 0);
    const save = board.save();
    const twice: BoardSave = {
      ...save,
      stacks: [
        { id: 1, cards: [1, 2] },
        { id: 2, cards: [2] },
      ],
    };
    await expectRejection(Board.load(catalog, twice), "invalid-save");
    board.free();
  });

  it("refuses a save of another document, and ids that a number cannot hold", async () => {
    const catalog = await shared();
    const board = await Board.create(catalog, 1);
    board.spawn("wood", 0, 0);
    const save = board.save();
    const wrongKind = { ...save, kind: "catalog" } as unknown as BoardSave;
    await expectRejection(Board.load(catalog, wrongKind), "unsupported-document");
    const hugeCounter = { ...save, next_card: 2 ** 53 };
    await expectRejection(Board.load(catalog, hugeCounter), "bad-id");
    board.free();
  });
});

describe("disposal", () => {
  it("frees once, and refuses every call afterwards with disposed", async () => {
    const board = await Board.create(await shared(), 1);
    board.free();
    board.free();
    expectFailure(() => board.spawn("wood", 0, 0), "disposed");
    expectFailure(() => board.tick(1), "disposed");
    expectFailure(() => board.save(), "disposed");
  });
});
