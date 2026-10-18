// Tests for the A* search: four and eight directions, corner cutting, the goal and the start, the cost
// callback, the node limit, determinism, and reuse of one search object on grids of different sizes.

import { describe, expect, it } from "vitest";
import { TilesError } from "./error";
import { buildCollisionGrid, type CollisionGrid } from "./collision";
import { loadTileMap } from "./load";
import { findPath, PathSearch, type PathCell } from "./pathfinding";
import { map, terrainTileset, tileLayer } from "./test-support";

/**
 * Builds the collision grid of a map from rows of tile ids: 2 is stone (solid), 3 is a ledge (partial), 6 is a
 * one-way platform, and 0 is empty.
 */
async function gridOf(rows: number[][]): Promise<CollisionGrid> {
  const height = rows.length;
  const width = rows[0]?.length ?? 0;
  const loaded = await loadTileMap(
    map({ width, height, layers: [tileLayer("level", width, height, rows.flat())] }),
  );
  return buildCollisionGrid(loaded, { nonEmpty: false, solidProperty: "solid", oneWayProperty: "oneWay", shapes: true });
}

/** A 5 by 4 room with a wall of stone standing in column 2, rows 0 to 2, and an opening in row 3. */
const wall = [
  [0, 0, 2, 0, 0],
  [0, 0, 2, 0, 0],
  [0, 0, 2, 0, 0],
  [0, 0, 0, 0, 0],
];

/** The cells of a path, as "x,y" text, for easy comparison. */
function text(path: PathCell[] | null): string[] | null {
  return path === null ? null : path.map((cell) => `${cell.x},${cell.y}`);
}

describe("finding a path in four directions", () => {
  it("finds the shortest path in an open room, with one cell per step", async () => {
    const grid = await gridOf([[0, 0, 0], [0, 0, 0], [0, 0, 0]]);
    const path = findPath(grid, { x: 0, y: 0 }, { x: 2, y: 2 });
    expect(path).toHaveLength(5);
    expect(path?.[0]).toEqual({ x: 0, y: 0 });
    expect(path?.at(-1)).toEqual({ x: 2, y: 2 });
  });

  it("goes round a wall through its opening", async () => {
    const grid = await gridOf(wall);
    const path = findPath(grid, { x: 0, y: 0 }, { x: 4, y: 0 });
    expect(path).not.toBeNull();
    expect(path?.some((cell) => cell.x === 2 && cell.y <= 2)).toBe(false);
    expect(path?.length).toBe(11);
  });

  it("finds no path when the goal is walled off", async () => {
    const grid = await gridOf([
      [0, 2, 0],
      [2, 2, 2],
      [0, 2, 0],
    ]);
    expect(findPath(grid, { x: 0, y: 0 }, { x: 2, y: 2 })).toBeNull();
  });

  it("returns the start alone when the start is the goal", async () => {
    const grid = await gridOf([[0, 0]]);
    expect(text(findPath(grid, { x: 1, y: 0 }, { x: 1, y: 0 }))).toEqual(["1,0"]);
  });

  it("does not enter a cell that is solid, and treats a one-way platform as walkable", async () => {
    const grid = await gridOf([[0, 6, 0]]);
    expect(text(findPath(grid, { x: 0, y: 0 }, { x: 2, y: 0 }))).toEqual(["0,0", "1,0", "2,0"]);
  });

  it("refuses a goal that is blocked, a start or goal outside the grid, and cells that are not whole", async () => {
    const grid = await gridOf([[0, 2, 0]]);
    expect(findPath(grid, { x: 0, y: 0 }, { x: 1, y: 0 })).toBeNull();
    expect(findPath(grid, { x: -1, y: 0 }, { x: 0, y: 0 })).toBeNull();
    expect(findPath(grid, { x: 0, y: 0 }, { x: 3, y: 0 })).toBeNull();
    expect(() => findPath(grid, { x: 0.5, y: 0 }, { x: 0, y: 0 })).toThrow(TilesError);
  });
});

describe("finding a path in eight directions", () => {
  it("takes diagonal steps when they are allowed and not blocked", async () => {
    const grid = await gridOf([[0, 0, 0], [0, 0, 0], [0, 0, 0]]);
    expect(text(findPath(grid, { x: 0, y: 0 }, { x: 2, y: 2 }, { diagonal: true }))).toEqual(["0,0", "1,1", "2,2"]);
  });

  it("does not cut the corner of a blocked cell by default", async () => {
    const grid = await gridOf([
      [0, 2],
      [2, 0],
    ]);
    expect(findPath(grid, { x: 0, y: 0 }, { x: 1, y: 1 }, { diagonal: true })).toBeNull();
  });

  it("cuts the corner when cornerCutting is set", async () => {
    const grid = await gridOf([
      [0, 2],
      [2, 0],
    ]);
    expect(text(findPath(grid, { x: 0, y: 0 }, { x: 1, y: 1 }, { diagonal: true, cornerCutting: true }))).toEqual([
      "0,0",
      "1,1",
    ]);
  });
});

describe("costs and limits", () => {
  it("prefers a cheaper way round when a cell costs extra", async () => {
    const grid = await gridOf([
      [0, 0, 0],
      [0, 0, 0],
    ]);
    const cost = (x: number, y: number): number => (x === 1 && y === 0 ? 10 : 0);
    const path = findPath(grid, { x: 0, y: 0 }, { x: 2, y: 0 }, { cost });
    expect(text(path)).toEqual(["0,0", "0,1", "1,1", "2,1", "2,0"]);
  });

  it("treats a cell with an infinite cost as impassable", async () => {
    const grid = await gridOf([[0, 0, 0]]);
    const cost = (x: number): number => (x === 1 ? Infinity : 0);
    expect(findPath(grid, { x: 0, y: 0 }, { x: 2, y: 0 }, { cost })).toBeNull();
  });

  it("refuses a negative or non-numeric cost", async () => {
    const grid = await gridOf([[0, 0, 0]]);
    expect(() => findPath(grid, { x: 0, y: 0 }, { x: 2, y: 0 }, { cost: () => -1 })).toThrow(TilesError);
    expect(() => findPath(grid, { x: 0, y: 0 }, { x: 2, y: 0 }, { cost: () => Number.NaN })).toThrow(TilesError);
  });

  it("gives up after maxNodes expansions, and finds the path with enough of them", async () => {
    const grid = await gridOf(wall);
    expect(findPath(grid, { x: 0, y: 0 }, { x: 4, y: 0 }, { maxNodes: 3 })).toBeNull();
    expect(findPath(grid, { x: 0, y: 0 }, { x: 4, y: 0 }, { maxNodes: 1000 })).not.toBeNull();
  });

  it("refuses a maxNodes below one", async () => {
    const grid = await gridOf([[0]]);
    expect(() => findPath(grid, { x: 0, y: 0 }, { x: 0, y: 0 }, { maxNodes: 0 })).toThrow(TilesError);
  });
});

describe("the search object", () => {
  it("gives the same path every time for the same input", async () => {
    const grid = await gridOf([
      [0, 0, 0, 0],
      [0, 0, 0, 0],
      [0, 0, 0, 0],
    ]);
    const search = new PathSearch();
    const first = text(search.find(grid, { x: 0, y: 0 }, { x: 3, y: 2 }, { diagonal: true }));
    for (let round = 0; round < 3; round++) {
      expect(text(search.find(grid, { x: 0, y: 0 }, { x: 3, y: 2 }, { diagonal: true }))).toEqual(first);
    }
    expect(text(findPath(grid, { x: 0, y: 0 }, { x: 3, y: 2 }, { diagonal: true }))).toEqual(first);
  });

  it("works on grids of different sizes in turn, without carrying results over", async () => {
    const small = await gridOf([[0, 0]]);
    const large = await gridOf(wall);
    const search = new PathSearch();
    expect(text(search.find(small, { x: 0, y: 0 }, { x: 1, y: 0 }))).toEqual(["0,0", "1,0"]);
    expect(search.find(large, { x: 0, y: 0 }, { x: 4, y: 0 })?.length).toBe(11);
    expect(text(search.find(small, { x: 0, y: 0 }, { x: 1, y: 0 }))).toEqual(["0,0", "1,0"]);
    expect(search.find(large, { x: 0, y: 0 }, { x: 2, y: 0 })).toBeNull();
  });

  it("searches a grid of an infinite map with its own coordinates", async () => {
    const loaded = await loadTileMap({
      infinite: true,
      layers: [{ chunks: [{ data: [0, 0, 0], height: 1, width: 3, x: -2, y: 4 }], id: 1, name: "ground", type: "tilelayer", visible: true }],
      orientation: "orthogonal",
      tileheight: 16,
      tilesets: [terrainTileset()],
      tilewidth: 16,
      type: "map",
      version: "1.10",
    });
    const grid = buildCollisionGrid(loaded, { nonEmpty: false, solidProperty: "solid" });
    expect(text(findPath(grid, { x: -2, y: 4 }, { x: 0, y: 4 }))).toEqual(["-2,4", "-1,4", "0,4"]);
  });
});
