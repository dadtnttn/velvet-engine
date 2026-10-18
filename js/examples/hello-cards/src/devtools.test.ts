// @vitest-environment happy-dom
// The developer tools of hello-cards: the coins command puts coin cards on free spots, and the overlay mounts on
// the host and goes when it is destroyed.

import { beforeAll, describe, expect, it } from "vitest";
import { Board, loadCatalog, type Catalog } from "@velvet/cards";
import { GameClock, GameLoop } from "@velvet/core";
import { createStage } from "@velvet/ui";
import { addCoins, coinCount, mountDevtools } from "./devtools";
import { loadWasmForTests, readPublic } from "./test-support";

let catalog: Catalog;

beforeAll(async () => {
  await loadWasmForTests();
  catalog = await loadCatalog(await readPublic("data/catalog.json"));
});

describe("the coins command", () => {
  it("puts each coin on a free spot of its own, and skips a spot that is taken", async () => {
    const board = await Board.create(catalog, 7);
    board.spawn("forest", 4.5, 2.5); // the spot the coins look for first
    expect(addCoins(board, 3)).toBe(3);
    expect(coinCount(board)).toBe(3);
    const spots = board.stacks().map((stack) => `${stack.x},${stack.y}`);
    expect(new Set(spots).size).toBe(spots.length);
  });

  it("refuses a count that is not a whole number from 1 to 20, and changes nothing", async () => {
    const board = await Board.create(catalog, 7);
    expect(() => addCoins(board, 0)).toThrow(RangeError);
    expect(() => addCoins(board, 21)).toThrow(RangeError);
    expect(() => addCoins(board, 2.5)).toThrow(RangeError);
    expect(board.cards()).toHaveLength(0);
  });
});

describe("the developer tools", () => {
  it("mount an overlay on the host, and remove it when they are destroyed", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const stage = createStage(host, { width: 1920, height: 1080 });
    const board = await Board.create(catalog, 3);
    const tools = mountDevtools({
      host,
      clock: new GameClock(),
      loop: new GameLoop({ update: () => undefined }),
      stage,
      board: () => board,
      flushEvents: () => undefined,
    });
    expect(tools.enabled).toBe(true);
    expect(host.querySelector(".vdt-root")).not.toBeNull();
    tools.destroy();
    expect(host.querySelector(".vdt-root")).toBeNull();
    stage.destroy();
    host.remove();
  });
});
