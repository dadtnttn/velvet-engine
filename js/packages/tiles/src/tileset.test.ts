// Tests for tilesets: finding the tileset of a global id, the frame an animated tile shows at a time, and the
// reading of a tileset's own fields (its offset, its duplicates and its image collections).

import { describe, expect, it } from "vitest";
import { Problems } from "./reader";
import { animatedLocalId, findTileset, readTileset } from "./tileset";
import { terrainTileset } from "./test-support";
import type { Tileset } from "./types";

/** A tileset of the given first id and count, for the search. */
function tileset(name: string, firstGid: number, tileCount: number): Tileset {
  return {
    name,
    firstGid,
    tileCount,
    tileWidth: 16,
    tileHeight: 16,
    columns: 4,
    image: `${name}.png`,
    imageWidth: 64,
    imageHeight: 64,
    margin: 0,
    spacing: 0,
    offsetX: 0,
    offsetY: 0,
    properties: {},
    tiles: new Map(),
  };
}

/** Reads the test tileset with the given JSON overrides. */
function read(overrides: Record<string, unknown> = {}): { tileset: Tileset | undefined; problems: Problems } {
  const problems = new Problems();
  const tileset = readTileset(problems, terrainTileset(overrides), "tilesets[0]", 1);
  return { tileset, problems };
}

describe("finding the tileset of a global id", () => {
  const sets = [tileset("a", 1, 4), tileset("b", 10, 2), tileset("c", 20, 5)];

  it("finds the tileset that owns each id, at both ends of its range", () => {
    expect(findTileset(sets, 1)?.name).toBe("a");
    expect(findTileset(sets, 4)?.name).toBe("a");
    expect(findTileset(sets, 10)?.name).toBe("b");
    expect(findTileset(sets, 11)?.name).toBe("b");
    expect(findTileset(sets, 24)?.name).toBe("c");
  });

  it("finds nothing for 0, for the gaps between tilesets, and past the last one", () => {
    expect(findTileset(sets, 0)).toBeUndefined();
    expect(findTileset(sets, 5)).toBeUndefined();
    expect(findTileset(sets, 9)).toBeUndefined();
    expect(findTileset(sets, 12)).toBeUndefined();
    expect(findTileset(sets, 25)).toBeUndefined();
  });

  it("finds nothing in a map without tilesets", () => {
    expect(findTileset([], 1)).toBeUndefined();
  });
});

describe("animated tiles", () => {
  const animated = tileset("water", 1, 6);
  const tiles = new Map([[2, { id: 2, type: "", properties: {}, collision: [], animation: [
    { tileId: 2, duration: 200 },
    { tileId: 3, duration: 100 },
    { tileId: 4, duration: 50 },
  ] }]]);
  const water: Tileset = { ...animated, tiles };

  it("shows each frame for its duration, then loops", () => {
    expect(animatedLocalId(water, 2, 0)).toBe(2);
    expect(animatedLocalId(water, 2, 199)).toBe(2);
    expect(animatedLocalId(water, 2, 200)).toBe(3);
    expect(animatedLocalId(water, 2, 299)).toBe(3);
    expect(animatedLocalId(water, 2, 300)).toBe(4);
    expect(animatedLocalId(water, 2, 349)).toBe(4);
    expect(animatedLocalId(water, 2, 350)).toBe(2);
    expect(animatedLocalId(water, 2, 3 * 350 + 210)).toBe(3);
  });

  it("loops backward in time too", () => {
    expect(animatedLocalId(water, 2, -1)).toBe(4);
    expect(animatedLocalId(water, 2, -350)).toBe(2);
  });

  it("shows a tile without an animation as it is", () => {
    expect(animatedLocalId(water, 5, 12345)).toBe(5);
  });
});

describe("reading a tileset", () => {
  it("reads the tile offset and the picture's size", () => {
    const { tileset, problems } = read({ tileoffset: { x: 2, y: -4 }, imagewidth: 64, imageheight: 32 });
    expect(problems.issues).toEqual([]);
    expect(tileset).toMatchObject({ offsetX: 2, offsetY: -4, imageWidth: 64, imageHeight: 32, columns: 4 });
  });

  it("refuses a tileset that has no picture, which is an image collection", () => {
    const { tileset, problems } = read({ image: undefined });
    expect(tileset).toBeUndefined();
    expect(problems.issues[0]).toEqual(expect.objectContaining({ code: "unsupported", path: "tilesets[0].image" }));
  });

  it("refuses two entries with one tile id", () => {
    const { problems } = read({ tiles: [{ id: 1, type: "a" }, { id: 1, type: "b" }] });
    expect(problems.issues).toContainEqual(expect.objectContaining({ code: "duplicate", path: "tilesets[0].tiles[1].id" }));
  });

  it("refuses an entry for a tile that the tileset does not have", () => {
    const { problems } = read({ tiles: [{ id: 8, type: "a" }] });
    expect(problems.issues).toContainEqual(expect.objectContaining({ code: "value", path: "tilesets[0].tiles[0].id" }));
  });

  it("keeps the type of a tile, read from type or from class", () => {
    const { tileset } = read({ tiles: [{ id: 0, type: "grass" }, { id: 6, class: "decor" }] });
    expect(tileset?.tiles.get(0)?.type).toBe("grass");
    expect(tileset?.tiles.get(6)?.type).toBe("decor");
  });
});
