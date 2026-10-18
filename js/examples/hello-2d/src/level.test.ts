// Tests of the level: the shipped map loads with its start, checkpoint, coins and goal where the design puts them,
// its collision and its spikes read as the rules say, and a map without a start, a goal or coins is refused.

import { describe, expect, it } from "vitest";
import { loadTileMap } from "@velvet/tiles";
import { LevelError, levelFromMap } from "./level";
import { levelFromRows, loadShippedLevel, mapObjectFromRows } from "./test-support";

describe("the shipped level", () => {
  it("is 120 by 30 cells, with the start, one checkpoint, the goal and 29 coins where the design puts them", async () => {
    const level = await loadShippedLevel();
    expect(level.width).toBe(120 * 32);
    expect(level.height).toBe(30 * 32);
    expect(level.spawn).toEqual({ x: 80, y: 832 });
    expect(level.checkpoints).toEqual([
      { x: 80, y: 832 },
      { x: 2224, y: 832 },
    ]);
    expect(level.goal).toEqual({ x: 3728, y: 832 });
    expect(level.coins).toHaveLength(29);
    for (const coin of level.coins) {
      expect(coin.x).toBeGreaterThan(0);
      expect(coin.x).toBeLessThan(level.width);
    }
  });

  it("is solid where the ground and the stone are, and one-way on the planks", async () => {
    const level = await loadShippedLevel();
    expect(level.solid.kindAt(0, 26)).toBe("solid");
    expect(level.solid.isOneWayAt(0, 26)).toBe(false);
    expect(level.solid.kindAt(36, 25)).toBe("solid");
    expect(level.solid.isOneWayAt(36, 25)).toBe(false);
    expect(level.solid.kindAt(21, 24)).toBe("solid");
    expect(level.solid.isOneWayAt(21, 24)).toBe(true);
  });

  it("has its spikes in the hazards, not in the collision: the hero walks into them", async () => {
    const level = await loadShippedLevel();
    expect(level.hazards.kindAt(31, 25)).toBe("solid");
    expect(level.solid.kindAt(31, 25)).toBe("empty");
    expect(level.hazards.kindAt(14, 26)).toBe("solid");
    expect(level.solid.kindAt(14, 26)).toBe("empty");
    expect(level.solid.kindAt(14, 27)).toBe("solid");
  });
});

describe("levels that are refused", () => {
  it("needs exactly one start", async () => {
    await expect(levelFromRows(["..o.F", "#####"])).rejects.toThrow(LevelError);
    await expect(levelFromRows(["S.S.oF", "######"])).rejects.toThrow(/exactly one spawn point/);
  });

  it("needs exactly one goal", async () => {
    await expect(levelFromRows(["S.o", "###"])).rejects.toThrow(/exactly one goal/);
  });

  it("needs at least one coin", async () => {
    await expect(levelFromRows(["S.F", "###"])).rejects.toThrow(/no coins/);
  });

  it("needs its object layer", async () => {
    const map = mapObjectFromRows(["S.oF", "####"]);
    const layers = (map["layers"] as Array<{ name: string }>).filter((layer) => layer.name !== "objects");
    const tiles = await loadTileMap({ ...map, layers });
    expect(() => levelFromMap(tiles)).toThrow(/no object layer/);
  });
});
