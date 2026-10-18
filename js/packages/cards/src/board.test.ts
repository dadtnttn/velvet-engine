// Tests of the Board API that games rely on: seeds, ids, refusals, events, snapshots, saves and
// disposal. The shared fixtures (fixtures.test.ts) check the rules step by step; these tests check
// the boundary around them.

import { describe, expect, it } from "vitest";
import {
  Board,
  loadCatalog,
  type BoardEvent,
  type BoardPoint,
  type BoardRect,
  type BoardSave,
  type BoardSize,
  type Catalog,
  type OutputPlacement,
  type Seed,
} from "./index";
import { findFreeSpot } from "./spot";
import { expectFailure, expectRejection, generator, SHARED_CATALOG } from "./test-support";

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

describe("free spots", () => {
  it("finds the nearest spot that a new stack does not overlap, and never moves a card", async () => {
    const board = await Board.create(await shared(), 1);
    const worker = board.spawn("worker", 0, 0);
    board.takeEvents();
    expect(board.freeSpot({ x: 0, y: 0 }, { width: 1, height: 1 })).toEqual({ x: 0, y: -1 });
    expect(board.freeSpot({ x: 3, y: 3 }, { width: 1, height: 1 })).toEqual({ x: 3, y: 3 });
    expect(board.takeEvents(), "a search queues no events").toEqual([]);
    expect(board.cards().map((card) => [card.id, card.x, card.y])).toEqual([[worker, 0, 0]]);
    board.free();
  });

  it("ignores the stack named by except, so that a stack can stay where it is", async () => {
    const board = await Board.create(await shared(), 1);
    const worker = board.spawn("worker", 0, 0);
    expect(board.freeSpot({ x: 0, y: 0 }, { width: 1, height: 1 }, { except: worker })).toEqual({ x: 0, y: 0 });
    expectFailure(() => board.freeSpot({ x: 0, y: 0 }, { width: 1, height: 1 }, { except: 99 }), "no-such-card");
    board.free();
  });

  it("keeps the spot inside the bounds, and returns null when none is left", async () => {
    const board = await Board.create(await shared(), 1);
    board.spawn("worker", 0, 0);
    const bounds = { x: 0, y: 0, width: 4, height: 4 };
    expect(board.freeSpot({ x: -3, y: 0 }, { width: 1, height: 1 }, { bounds })).toEqual({ x: 0, y: 1 });
    const nowhere = { x: 0, y: 0, width: 0, height: 0 };
    expect(board.freeSpot({ x: 0, y: 0 }, { width: 1, height: 1 }, { bounds: nowhere })).toBeNull();
    board.free();
  });

  it("sees the stacks as they are after a move, and its spots are where a stack can be placed", async () => {
    const board = await Board.create(await shared(), 1);
    const size = { width: 1, height: 1 };
    const first = board.spawn("worker", 0, 0);
    board.spawn("forest", 0, -1);
    board.moveStack(first, 5, 5);
    const spot = board.freeSpot({ x: 5, y: 5 }, size);
    expect(spot).toEqual({ x: 5, y: 4 });
    if (spot === null) throw new Error("a free spot was expected");
    board.spawn("worker", spot.x, spot.y);
    const bottoms = board.stacks().map((stack) => ({ x: stack.x, y: stack.y }));
    for (const [index, a] of bottoms.entries()) {
      for (const b of bottoms.slice(index + 1)) {
        const apart = Math.abs(a.x - b.x) >= size.width || Math.abs(a.y - b.y) >= size.height;
        expect(apart, `stacks at (${a.x}, ${a.y}) and (${b.x}, ${b.y})`).toBe(true);
      }
    }
    board.free();
  });

  it("refuses a point, a size or bounds that are not valid, with bad-input", async () => {
    const board = await Board.create(await shared(), 1);
    const size = { width: 1, height: 1 };
    expectFailure(() => board.freeSpot({ x: Number.NaN, y: 0 }, size), "bad-input");
    expectFailure(() => board.freeSpot({ x: 0, y: 0 }, { width: 0, height: 1 }), "bad-input");
    expectFailure(() => board.freeSpot({ x: 0, y: 0 }, { width: 1, height: Number.POSITIVE_INFINITY }), "bad-input");
    const inverted = { x: 0, y: 0, width: -1, height: 2 };
    expectFailure(() => board.freeSpot({ x: 0, y: 0 }, size, { bounds: inverted }), "bad-input");
    board.free();
  });
});

describe("output placement", () => {
  it("is the row by default, and a save does not hold it: a loaded board starts with the row", async () => {
    const catalog = await shared();
    const board = await Board.create(catalog, 1);
    expect(board.outputPlacement()).toStrictEqual({ kind: "row" });
    board.setOutputPlacement({ kind: "free", footprint: { width: 1, height: 1 } });
    const save = board.save();
    expect(JSON.stringify(save), "a save holds no placement").not.toContain("footprint");
    const loaded = await Board.load(catalog, save);
    expect(loaded.outputPlacement()).toStrictEqual({ kind: "row" });
    loaded.free();
    board.free();
  });

  it("returns the placement that was set, and leaves out the bounds when none were given", async () => {
    const board = await Board.create(await shared(), 1);
    const bounded: OutputPlacement = {
      kind: "free",
      footprint: { width: 2.5, height: 1 },
      bounds: { x: 0, y: -1, width: 4, height: 2 },
    };
    board.setOutputPlacement(bounded);
    expect(board.outputPlacement()).toStrictEqual(bounded);
    board.setOutputPlacement({ kind: "free", footprint: { width: 2.5, height: 1 } });
    expect(board.outputPlacement()).toStrictEqual({ kind: "free", footprint: { width: 2.5, height: 1 } });
    board.setOutputPlacement({ kind: "row" });
    expect(board.outputPlacement()).toStrictEqual({ kind: "row" });
    board.free();
  });

  it("refuses a placement that cannot hold a stack with bad-placement, and keeps the one in use", async () => {
    const board = await Board.create(await shared(), 1);
    const kept: OutputPlacement = { kind: "free", footprint: { width: 1, height: 1 } };
    board.setOutputPlacement(kept);
    const size = { width: 1, height: 1 };
    expectFailure(() => board.setOutputPlacement({ kind: "free", footprint: { width: 0, height: 1 } }), "bad-placement");
    expectFailure(
      () => board.setOutputPlacement({ kind: "free", footprint: size, bounds: { x: 0, y: 0, width: -1, height: 2 } }),
      "bad-placement",
    );
    expectFailure(
      () =>
        board.setOutputPlacement({
          kind: "free",
          footprint: { width: 3e38, height: 3e38 },
          bounds: { x: 3e38, y: 0, width: 3e38, height: 1 },
        }),
      "bad-placement",
    );
    expectFailure(() => board.setOutputPlacement({ kind: "free", footprint: { width: Number.NaN, height: 1 } }), "bad-input");
    expectFailure(() => board.setOutputPlacement({ kind: "free", footprint: { width: 1e39, height: 1 } }), "bad-input");
    expectFailure(
      () => board.setOutputPlacement({ kind: "free", footprint: size, bounds: { x: Number.NaN, y: 0, width: 1, height: 1 } }),
      "bad-input",
    );
    expect(board.outputPlacement()).toStrictEqual(kept);
    board.free();
  });

  it("places a job's outputs by the placement given at creation, and refuses a bad one there", async () => {
    const catalog = await shared();
    const board = await Board.create(catalog, 1, {
      outputPlacement: { kind: "free", footprint: { width: 1, height: 1 } },
    });
    const forest = board.spawn("forest", 0, 0);
    board.dropOn(board.spawn("worker", 0, 0), forest);
    board.takeEvents();
    expect(board.tick(2)).toContainEqual({ event: "card_spawned", card: 3, def: "wood", x: 1, y: 0 });
    board.free();

    const bad: OutputPlacement = { kind: "free", footprint: { width: 0, height: 1 } };
    await expectRejection(Board.create(catalog, 1, { outputPlacement: bad }), "bad-placement");
    const plain = await Board.create(catalog, 1);
    const saved = plain.save();
    plain.free();
    await expectRejection(Board.load(catalog, saved, { outputPlacement: bad }), "bad-placement");
  });

  it("searches again without bounds when the bounds hold no free spot", async () => {
    const board = await Board.create(await shared(), 1, {
      outputPlacement: {
        kind: "free",
        footprint: { width: 1, height: 1 },
        bounds: { x: 0, y: 0, width: 0, height: 0 },
      },
    });
    const bush = board.spawn("berry_bush", 0, 0);
    board.dropOn(board.spawn("worker", 0, 0), bush);
    board.takeEvents();
    // The only spot in the bounds is the bush's own, so the search runs again without bounds: the first copy takes
    // the spot to the right of the bush, and the second one the spot at (1, -1).
    expect(board.tick(2).filter((event) => event.event === "card_spawned")).toEqual([
      { event: "card_spawned", card: 3, def: "berries", x: 1, y: 0 },
      { event: "card_spawned", card: 4, def: "berries", x: 1, y: -1 },
    ]);
    board.free();
  });

  it("refuses every placement call on a freed board with disposed", async () => {
    const board = await Board.create(await shared(), 1);
    board.free();
    expectFailure(() => board.setOutputPlacement({ kind: "row" }), "disposed");
    expectFailure(() => board.outputPlacement(), "disposed");
  });
});

describe("the free output placement agrees with findFreeSpot", () => {
  it("picks, copy by copy, the spot that findFreeSpot picks for the same table", async () => {
    const catalog = await shared();
    const random = generator(19);
    const f32 = (value: number): number => Math.fround(value);
    let compared = 0;
    let widened = 0;
    let rows = 0;
    for (let table = 0; table < 24; table += 1) {
      const footprint: BoardSize = { width: f32(0.5 + random() * 1.5), height: f32(0.5 + random()) };
      const origin: BoardPoint = { x: f32(random() * 4 - 2), y: f32(random() * 4 - 2) };
      // Every fourth table has the origin itself as its only spot, which the forest holds, so every copy searches
      // again without bounds. The others have no bounds, or bounds that may or may not hold a spot.
      let bounds: BoardRect | undefined;
      if (table % 4 === 3) {
        bounds = { x: origin.x, y: origin.y, width: 0, height: 0 };
      } else if (table % 3 === 1) {
        bounds = {
          x: f32(origin.x - 1 + random() * 2),
          y: f32(origin.y - 1 + random() * 2),
          width: f32(random() * 4),
          height: f32(random() * 4),
        };
      }
      const board = await Board.create(catalog, table + 1, { outputPlacement: { kind: "free", footprint, bounds } });
      const forest = board.spawn("forest", origin.x, origin.y);
      board.dropOn(board.spawn("worker", origin.x, origin.y), forest);
      // Stones around the origin: some on the grid of the footprint (so that they touch the spots next to
      // them), the others anywhere near it.
      const stones = 1 + Math.floor(random() * 20);
      for (let n = 0; n < stones; n += 1) {
        const onGrid = random() < 0.5;
        const cellX = Math.floor(random() * 7) - 3;
        const cellY = Math.floor(random() * 7) - 3;
        const x = onGrid ? origin.x + footprint.width * cellX : random() * 10 - 5;
        const y = onGrid ? origin.y + footprint.height * cellY : random() * 10 - 5;
        board.spawn("stone", f32(x), f32(y));
      }
      for (let copy = 0; copy < 3; copy += 1) {
        // The table as the module holds it, before the job finishes: the job's stack and the earlier copies included.
        const obstacles = board.stacks();
        const near = { x: f32(origin.x + footprint.width), y: origin.y };
        // The module searches inside the bounds first, and when no spot there is free, again without bounds.
        const inside = findFreeSpot(obstacles, near, footprint, bounds);
        if (inside === null) widened += 1;
        const found = inside ?? findFreeSpot(obstacles, near, footprint);
        if (found === null) rows += 1;
        const expected = found ?? { x: f32(origin.x + 1.5), y: origin.y };
        board.takeEvents();
        const spawned = board.tick(2).find((event) => event.event === "card_spawned");
        // The module writes a binary32 position in its shortest decimal form, so Math.fround gives back the
        // exact value that the module holds.
        const got = spawned?.event === "card_spawned" ? { x: f32(spawned.x), y: f32(spawned.y) } : undefined;
        expect(got, `table ${table}, copy ${copy}`).toEqual({ x: expected.x, y: expected.y });
        compared += 1;
      }
      board.free();
    }
    expect(compared).toBe(72);
    expect(widened, "some tables must reach the search without bounds too").toBeGreaterThan(0);
    expect(rows, "the row position is only for a table with no free spot at all").toBe(0);
  });
});
