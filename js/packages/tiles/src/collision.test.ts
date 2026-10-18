// Tests for collision grids: each rule that makes a cell solid (every tile, a property, the tile's collision
// shapes), the one-way flag, the union of layers, the alignment of shapes in their cell, the bounds of an
// infinite map, and the refusals.

import { describe, expect, it } from "vitest";
import { TilesError } from "./error";
import { buildCollisionGrid } from "./collision";
import { loadTileMap } from "./load";
import { map, objectLayer, object, terrainTileset, tileLayer } from "./test-support";
import type { TileMap } from "./types";

/** Loads a map from its JSON object. */
async function load(input: Record<string, unknown>): Promise<TileMap> {
  return loadTileMap(input);
}

/** A 4 by 2 map: stone on the top row, a flower and a platform below, and an empty cell. */
const layout = [2, 2, 2, 2, 7, 0, 6, 0];

describe("the rules that make a cell solid", () => {
  it("makes every tile of the layers solid by default", async () => {
    const grid = buildCollisionGrid(await load(map({ width: 4, height: 2, layers: [tileLayer("ground", 4, 2, layout)] })));
    expect(grid.kindAt(0, 0)).toBe("solid");
    expect(grid.kindAt(0, 1)).toBe("solid");
    expect(grid.kindAt(2, 1)).toBe("solid");
    expect(grid.kindAt(1, 1)).toBe("empty");
    expect(grid.kindAt(3, 1)).toBe("empty");
  });

  it("reads only the layers that are named", async () => {
    const loaded = await load(
      map({
        width: 4,
        height: 2,
        layers: [tileLayer("ground", 4, 2, [2, 0, 0, 0, 0, 0, 0, 0]), tileLayer("decor", 4, 2, [0, 0, 0, 0, 0, 7, 0, 0])],
      }),
    );
    const grid = buildCollisionGrid(loaded, { layers: ["ground"] });
    expect(grid.kindAt(0, 0)).toBe("solid");
    expect(grid.kindAt(1, 1)).toBe("empty");
  });

  it("makes the tiles with a true solid property solid, and leaves the rest empty", async () => {
    const grid = buildCollisionGrid(await load(map({ width: 4, height: 2, layers: [tileLayer("ground", 4, 2, layout)] })), {
      nonEmpty: false,
      solidProperty: "solid",
    });
    expect(grid.kindAt(0, 0)).toBe("solid");
    expect(grid.kindAt(2, 1)).toBe("solid");
    expect(grid.kindAt(1, 1)).toBe("empty");
    expect(grid.kindAt(0, 1)).toBe("empty");
  });

  it("marks a solid tile that is also one-way, and no other", async () => {
    const grid = buildCollisionGrid(await load(map({ width: 4, height: 2, layers: [tileLayer("ground", 4, 2, layout)] })), {
      nonEmpty: false,
      solidProperty: "solid",
      oneWayProperty: "oneWay",
    });
    expect(grid.isOneWayAt(2, 1)).toBe(true);
    expect(grid.isOneWayAt(0, 0)).toBe(false);
    expect(grid.isOneWayAt(1, 1)).toBe(false);
  });

  it("gives the solid rectangles of a tile's collision shape, placed in the cell", async () => {
    const grid = buildCollisionGrid(await load(map({ width: 4, height: 2, layers: [tileLayer("ground", 4, 2, [3, 0, 0, 0, 0, 0, 0, 0])] })), {
      nonEmpty: false,
      shapes: true,
    });
    expect(grid.kindAt(0, 0)).toBe("partial");
    expect(grid.partsAt(0, 0)).toEqual([{ x: 0, y: 0, width: 16, height: 6 }]);
    expect(grid.partsAt(1, 0)).toEqual([]);
    expect(grid.kindAt(1, 0)).toBe("empty");
  });

  it("makes a cell whole when one of its shapes covers the whole cell", async () => {
    const tileset = terrainTileset({
      tiles: [
        {
          id: 0,
          objectgroup: {
            objects: [{ id: 1, x: 0, y: 0, width: 16, height: 16, rotation: 0, visible: true }],
            draworder: "index",
            type: "objectgroup",
            visible: true,
            x: 0,
            y: 0,
          },
        },
      ],
    });
    const loaded = await load(
      map({ width: 2, height: 1, tilesets: [tileset], layers: [tileLayer("ground", 2, 1, [1, 0])] }),
    );
    const grid = buildCollisionGrid(loaded, { nonEmpty: false, shapes: true });
    expect(grid.kindAt(0, 0)).toBe("solid");
    expect(grid.partsAt(0, 0)).toEqual([]);
  });

  it("places the shapes of a tile bigger than the cell the way drawing places the tile", async () => {
    // A 16 by 32 tile with its bottom on the cell's bottom: its shape at y 16 to 32 is the cell's own rows.
    const tileset = terrainTileset({
      tileheight: 32,
      imageheight: 64,
      tiles: [
        {
          id: 0,
          objectgroup: {
            objects: [{ id: 1, x: 0, y: 16, width: 16, height: 16, rotation: 0, visible: true }],
            draworder: "index",
            type: "objectgroup",
            visible: true,
            x: 0,
            y: 0,
          },
        },
      ],
    });
    const loaded = await load(map({ width: 2, height: 1, tilesets: [tileset], layers: [tileLayer("ground", 2, 1, [1, 0])] }));
    const grid = buildCollisionGrid(loaded, { nonEmpty: false, shapes: true });
    expect(grid.kindAt(0, 0)).toBe("solid");
  });

  it("approximates a rotated rectangle by the box of its rotated corners, and ignores points", async () => {
    const tileset = terrainTileset({
      tiles: [
        {
          id: 0,
          objectgroup: {
            objects: [
              { id: 1, x: 8, y: 8, width: 4, height: 4, rotation: 45, visible: true },
              { id: 2, point: true, x: 2, y: 2, width: 0, height: 0, rotation: 0, visible: true },
            ],
            draworder: "index",
            type: "objectgroup",
            visible: true,
            x: 0,
            y: 0,
          },
        },
      ],
    });
    const loaded = await load(map({ width: 1, height: 1, tilesets: [tileset], layers: [tileLayer("ground", 1, 1, [1])] }));
    const grid = buildCollisionGrid(loaded, { nonEmpty: false, shapes: true });
    const part = grid.partsAt(0, 0)[0];
    expect(grid.partsAt(0, 0)).toHaveLength(1);
    expect(part?.width).toBeCloseTo(4 * Math.SQRT2, 9);
    expect(part?.height).toBeCloseTo(4 * Math.SQRT2, 9);
  });

  it("unions the layers: a whole cell from one layer wins over a part from another", async () => {
    const loaded = await load(
      map({
        width: 2,
        height: 1,
        layers: [tileLayer("ledges", 2, 1, [3, 0]), tileLayer("stone", 2, 1, [2, 0])],
      }),
    );
    const grid = buildCollisionGrid(loaded, { nonEmpty: false, shapes: true, solidProperty: "solid" });
    expect(grid.kindAt(0, 0)).toBe("solid");
    expect(grid.partsAt(0, 0)).toEqual([]);
  });

  it("is one-way only when every rule that made the cell solid is one-way", async () => {
    const loaded = await load(
      map({
        width: 2,
        height: 1,
        layers: [tileLayer("platforms", 2, 1, [6, 0]), tileLayer("walls", 2, 1, [2, 0])],
      }),
    );
    const grid = buildCollisionGrid(loaded, { nonEmpty: false, solidProperty: "solid", oneWayProperty: "oneWay" });
    expect(grid.isOneWayAt(0, 0)).toBe(false);
  });
});

describe("the shape of a grid", () => {
  it("reports the cells of a finite map from the origin, and nothing outside it", async () => {
    const grid = buildCollisionGrid(await load(map({ width: 4, height: 2, layers: [tileLayer("ground", 4, 2, layout)] })));
    expect([grid.originX, grid.originY, grid.columns, grid.rows]).toEqual([0, 0, 4, 2]);
    expect(grid.contains(3, 1)).toBe(true);
    expect(grid.contains(4, 1)).toBe(false);
    expect(grid.contains(-1, 0)).toBe(false);
    expect(grid.kindAt(-1, 0)).toBe("empty");
  });

  it("covers the chunks of an infinite map, which may start at negative cells", async () => {
    const loaded = await load({
      infinite: true,
      layers: [
        {
          chunks: [
            { data: [2, 2], height: 1, width: 2, x: -2, y: -1 },
            { data: [0, 2], height: 1, width: 2, x: 0, y: 0 },
          ],
          id: 1,
          name: "ground",
          type: "tilelayer",
          visible: true,
        },
      ],
      orientation: "orthogonal",
      tileheight: 16,
      tilesets: [terrainTileset()],
      tilewidth: 16,
      type: "map",
      version: "1.10",
    });
    const grid = buildCollisionGrid(loaded);
    expect([grid.originX, grid.originY, grid.columns, grid.rows]).toEqual([-2, -1, 4, 2]);
    expect(grid.kindAt(-2, -1)).toBe("solid");
    expect(grid.kindAt(1, 0)).toBe("solid");
    expect(grid.kindAt(0, 0)).toBe("empty");
    expect(grid.kindAt(-1, -1)).toBe("solid");
    expect(grid.kindAt(-1, 0)).toBe("empty");
    expect(grid.kindAt(-3, 0)).toBe("empty");
  });

  it("is empty when the map has no tile layers", async () => {
    const grid = buildCollisionGrid(await load(map({ layers: [objectLayer([object()])] })));
    expect(grid.columns).toBe(4);
    expect(grid.kindAt(0, 0)).toBe("empty");
  });
});

describe("refusals", () => {
  it("refuses a layer name that does not exist", async () => {
    const loaded = await load(map());
    expect(() => buildCollisionGrid(loaded, { layers: ["nope"] })).toThrow(TilesError);
  });

  it("refuses a layer that is not a tile layer", async () => {
    const loaded = await load(map({ layers: [objectLayer([object()], { name: "objects" })] }));
    expect(() => buildCollisionGrid(loaded, { layers: ["objects"] })).toThrow(/not a tile layer/);
  });

  it("refuses a grid that would be too large", async () => {
    const loaded = await load({
      infinite: true,
      layers: [
        {
          chunks: [
            { data: [2], height: 1, width: 1, x: 0, y: 0 },
            { data: [2], height: 1, width: 1, x: 5000, y: 5000 },
          ],
          id: 1,
          name: "ground",
          type: "tilelayer",
          visible: true,
        },
      ],
      orientation: "orthogonal",
      tileheight: 16,
      tilesets: [terrainTileset()],
      tilewidth: 16,
      type: "map",
      version: "1.10",
    });
    const error = (() => {
      try {
        buildCollisionGrid(loaded);
        return undefined;
      } catch (caught) {
        return caught;
      }
    })();
    expect(error).toBeInstanceOf(TilesError);
    expect((error as TilesError).code).toBe("bad-input");
  });
});
