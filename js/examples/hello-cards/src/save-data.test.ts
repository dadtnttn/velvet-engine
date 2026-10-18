// Tests of the save shape: a board and a clock go into a slot of the save store and come back as the
// same game. The store writes to a fake backend, so the test needs no browser and no bridge.

import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import { Board, loadCatalog } from "@velvet/cards";
import { createSaveStore, type GameClockState } from "@velvet/core";
import { buildSaveData, parseSaveData } from "./save-data";

/** A save backend that keeps texts in memory and counts the writes. It stands in for the bridge. */
class FakeBackend {
  readonly texts = new Map<string, string>();
  writes = 0;

  async list(): Promise<string[]> {
    return [...this.texts.keys()];
  }

  async read(slot: string): Promise<string | null> {
    return this.texts.get(slot) ?? null;
  }

  async write(slot: string, text: string): Promise<void> {
    this.writes += 1;
    this.texts.set(slot, text);
  }

  async remove(slot: string): Promise<void> {
    this.texts.delete(slot);
  }
}

/** The game's catalog, read from its file and checked by the engine. */
async function gameCatalog() {
  return loadCatalog(await readFile(new URL("../public/data/catalog.json", import.meta.url), "utf8"));
}

/** A small game in progress: a villager on a forest, with a job running. */
async function playedBoard(): Promise<Board> {
  const board = await Board.create(await gameCatalog(), 42);
  const forest = board.spawn("forest", 0, 0);
  const villager = board.spawn("villager", 0, 1.5);
  board.dropOn(villager, forest);
  board.tick(0.75);
  return board;
}

describe("the save document", () => {
  it("writes the board and the clock, and reads them back as the same game", async () => {
    const board = await playedBoard();
    const clock: GameClockState = { elapsed: 12.5, speed: 2, paused: false };
    board.takeEvents();
    const data = buildSaveData(board, clock);

    const backend = new FakeBackend();
    const store = createSaveStore({ game: "hello-cards", version: 1, backend });
    await store.save("slot-1", data);
    expect(backend.writes).toBe(1);

    const loaded = await store.load("slot-1");
    const parsed = parseSaveData(loaded?.data);
    expect(parsed).not.toBeNull();
    if (parsed === null) return;
    expect(parsed.clock).toEqual(clock);

    const restored = await Board.load(await gameCatalog(), parsed.board);
    expect(restored.save()).toEqual(board.save());
    restored.free();
    board.free();
  });

  it("answers null for an empty slot", async () => {
    const store = createSaveStore({ game: "hello-cards", version: 1, backend: new FakeBackend() });
    expect(await store.load("quick")).toBeNull();
  });

  it("refuses data that is not a game save", () => {
    expect(parseSaveData(null)).toBeNull();
    expect(parseSaveData({})).toBeNull();
    expect(
      parseSaveData({ board: { format: "velvet-stacks" }, clock: { elapsed: 0, speed: 1, paused: false } }),
    ).toBeNull();
  });

  it("refuses a clock that cannot be restored", async () => {
    const board = await playedBoard();
    const data = buildSaveData(board, { elapsed: -1, speed: 1, paused: false });
    expect(parseSaveData(data)).toBeNull();
    board.free();
  });
});
