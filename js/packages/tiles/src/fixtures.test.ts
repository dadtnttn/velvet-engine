// Runs the Tiled conformance suite, docs/formats/fixtures/tiles/cases.json. The suite is shared with other
// readers, so each case states its results in the format's own terms: sizes, tilesets, layers with their
// effective values, cell ids and flips, chunks, object shapes, and the code and path of each refusal.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { TilesError } from "./error";
import { findLayer } from "./map";
import { loadTileMap } from "./load";
import type { Layer, ObjectLayer, TileLayer, TileMap } from "./types";
import type { TilesErrorCode } from "./error";

const FIXTURES = new URL("../../../../docs/formats/fixtures/tiles/", import.meta.url);

/** A valid map and what it must give back. Every field after `file` is optional. */
interface ValidCase {
  readonly file: string;
  readonly infinite: boolean;
  readonly width?: number;
  readonly height?: number;
  readonly tileWidth?: number;
  readonly tileHeight?: number;
  readonly backgroundColor?: string;
  /** The map's parallax origin, when the case states it. */
  readonly parallaxOrigin?: { readonly x: number; readonly y: number };
  readonly tilesets?: readonly {
    readonly name: string;
    readonly firstGid: number;
    readonly tileCount: number;
    readonly columns: number;
    readonly offsetX?: number;
    readonly offsetY?: number;
  }[];
  readonly layers?: readonly {
    readonly name: string;
    readonly kind: Layer["kind"];
    readonly visible: boolean;
    readonly opacity: number;
    readonly offsetX: number;
    readonly offsetY: number;
    /** The effective parallax factors, when the case states them. */
    readonly parallax?: { readonly x: number; readonly y: number };
    /** The layer's own tint, when the case states it: null for none. */
    readonly tint?: string | null;
  }[];
  readonly layerOrder?: readonly string[];
  readonly cells?: Readonly<Record<string, readonly (readonly number[])[]>>;
  readonly flips?: Readonly<Record<string, readonly (readonly number[])[]>>;
  readonly chunks?: Readonly<
    Record<
      string,
      readonly { readonly x: number; readonly y: number; readonly width: number; readonly height: number; readonly gids: readonly number[] }[]
    >
  >;
  readonly objectShapes?: Readonly<Record<string, readonly string[]>>;
  readonly objectsNamed?: Readonly<Record<string, Readonly<Record<string, unknown>>>>;
  readonly external?: Readonly<Record<string, string>>;
  /** The animation frames of tiles, by tileset name and local id: each frame is [tileId, duration]. */
  readonly animations?: Readonly<Record<string, Readonly<Record<string, readonly (readonly [number, number])[]>>>>;
  /** The number of objects in the collision group of each tile, by tileset name and local id. */
  readonly collisionCounts?: Readonly<Record<string, Readonly<Record<string, number>>>>;
}

/** A map that must be refused: the code, and a path that must be among the issues when given. */
interface InvalidCase {
  readonly file: string;
  readonly code: TilesErrorCode;
  readonly path?: string;
}

/** The whole suite. */
interface Suite {
  readonly valid: readonly ValidCase[];
  readonly invalid: readonly InvalidCase[];
}

/** Reads a file of the fixtures folder as text. */
function read(name: string): string {
  return readFileSync(new URL(name, FIXTURES), "utf8");
}

/** The names of the leaf layers (tile, object, image) of a map, in depth-first order. */
function leafNames(layers: readonly Layer[]): string[] {
  return layers.flatMap((layer) => (layer.kind === "group" ? leafNames(layer.layers) : [layer.name]));
}

/** Every object of a map, with the name of its layer. */
function objectsOf(map: TileMap): { readonly layer: string; readonly object: ObjectLayer["objects"][number] }[] {
  const found: { layer: string; object: ObjectLayer["objects"][number] }[] = [];
  const visit = (layers: readonly Layer[]): void => {
    for (const layer of layers) {
      if (layer.kind === "objects") for (const object of layer.objects) found.push({ layer: layer.name, object });
      else if (layer.kind === "group") visit(layer.layers);
    }
  };
  visit(map.layers);
  return found;
}

const suite = JSON.parse(read("cases.json")) as Suite;

describe("tiles conformance: valid maps", () => {
  for (const testCase of suite.valid) {
    describe(testCase.file, () => {
      const loadExternal = async (source: string): Promise<object> => JSON.parse(read(source)) as object;

      it("has the sizes and tile size of the case", async () => {
        const loaded = await loadTileMap(read(testCase.file), { loadTileset: loadExternal });
        expect(loaded.infinite).toBe(testCase.infinite);
        if (testCase.width !== undefined) expect(loaded.width).toBe(testCase.width);
        if (testCase.height !== undefined) expect(loaded.height).toBe(testCase.height);
        if (testCase.tileWidth !== undefined) expect(loaded.tileWidth).toBe(testCase.tileWidth);
        if (testCase.tileHeight !== undefined) expect(loaded.tileHeight).toBe(testCase.tileHeight);
        if (testCase.backgroundColor !== undefined) expect(loaded.backgroundColor).toBe(testCase.backgroundColor);
        if (testCase.parallaxOrigin !== undefined) expect(loaded.parallaxOrigin).toEqual(testCase.parallaxOrigin);
      });

      it("has the tilesets of the case, in order of their first id", async () => {
        const loaded = await loadTileMap(read(testCase.file), { loadTileset: loadExternal });
        for (const expected of testCase.tilesets ?? []) {
          const tileset = loaded.tilesets.find((item) => item.name === expected.name);
          expect(tileset, expected.name).toBeDefined();
          expect(tileset?.firstGid).toBe(expected.firstGid);
          expect(tileset?.tileCount).toBe(expected.tileCount);
          expect(tileset?.columns).toBe(expected.columns);
          if (expected.offsetX !== undefined) expect(tileset?.offsetX).toBe(expected.offsetX);
          if (expected.offsetY !== undefined) expect(tileset?.offsetY).toBe(expected.offsetY);
        }
        const firsts = loaded.tilesets.map((item) => item.firstGid);
        expect(firsts).toEqual([...firsts].sort((a, b) => a - b));
      });

      it("has the layers of the case, with their effective values", async () => {
        const loaded = await loadTileMap(read(testCase.file), { loadTileset: loadExternal });
        if (testCase.layerOrder !== undefined) {
          expect(leafNames(loaded.layers)).toEqual(testCase.layerOrder);
        }
        for (const expected of testCase.layers ?? []) {
          const layer = findLayer(loaded, expected.name);
          expect(layer, expected.name).toBeDefined();
          expect(layer?.kind).toBe(expected.kind);
          expect(layer?.visible).toBe(expected.visible);
          expect(layer?.opacity).toBeCloseTo(expected.opacity, 12);
          expect(layer?.offsetX).toBe(expected.offsetX);
          expect(layer?.offsetY).toBe(expected.offsetY);
          if (expected.parallax !== undefined) {
            expect(layer?.parallax.x, `${expected.name} parallax x`).toBeCloseTo(expected.parallax.x, 12);
            expect(layer?.parallax.y, `${expected.name} parallax y`).toBeCloseTo(expected.parallax.y, 12);
          }
          if (expected.tint !== undefined) expect(layer?.tint, `${expected.name} tint`).toBe(expected.tint);
        }
      });

      it("has the cell ids and flips of the case", async () => {
        const loaded = await loadTileMap(read(testCase.file), { loadTileset: loadExternal });
        for (const [name, rows] of Object.entries(testCase.cells ?? {})) {
          const layer = findLayer(loaded, name) as TileLayer;
          expect(layer.kind, name).toBe("tiles");
          expect(Array.from(layer.chunks[0]?.gids ?? []), name).toEqual(rows.flat());
        }
        for (const [name, rows] of Object.entries(testCase.flips ?? {})) {
          const layer = findLayer(loaded, name) as TileLayer;
          expect(Array.from(layer.chunks[0]?.flips ?? []), name).toEqual(rows.flat());
        }
      });

      it("has the chunks of the case", async () => {
        const loaded = await loadTileMap(read(testCase.file), { loadTileset: loadExternal });
        for (const [name, expected] of Object.entries(testCase.chunks ?? {})) {
          const layer = findLayer(loaded, name) as TileLayer;
          expect(layer.chunks).toHaveLength(expected.length);
          for (const want of expected) {
            const chunk = layer.chunks.find((item) => item.x === want.x && item.y === want.y);
            expect(chunk, `${name} at ${want.x},${want.y}`).toBeDefined();
            expect(chunk?.width).toBe(want.width);
            expect(chunk?.height).toBe(want.height);
            expect(Array.from(chunk?.gids ?? [])).toEqual(want.gids);
          }
        }
      });

      it("has the tile animations and collision groups of the case", async () => {
        const loaded = await loadTileMap(read(testCase.file), { loadTileset: loadExternal });
        for (const [name, tiles] of Object.entries(testCase.animations ?? {})) {
          const tileset = loaded.tilesets.find((item) => item.name === name);
          for (const [localId, frames] of Object.entries(tiles)) {
            const found = tileset?.tiles.get(Number(localId))?.animation ?? [];
            expect(found.map((frame) => [frame.tileId, frame.duration]), `${name} ${localId}`).toEqual(frames);
          }
        }
        for (const [name, counts] of Object.entries(testCase.collisionCounts ?? {})) {
          const tileset = loaded.tilesets.find((item) => item.name === name);
          for (const [localId, count] of Object.entries(counts)) {
            expect(tileset?.tiles.get(Number(localId))?.collision.length, `${name} ${localId}`).toBe(count);
          }
        }
      });

      it("has the object shapes and the named objects of the case", async () => {
        const loaded = await loadTileMap(read(testCase.file), { loadTileset: loadExternal });
        for (const [name, shapes] of Object.entries(testCase.objectShapes ?? {})) {
          const layer = findLayer(loaded, name) as ObjectLayer;
          expect(layer.objects.map((item) => item.shape), name).toEqual(shapes);
        }
        for (const [name, expected] of Object.entries(testCase.objectsNamed ?? {})) {
          const found = objectsOf(loaded).find((item) => item.object.name === name)?.object;
          expect(found, name).toBeDefined();
          if (found === undefined) continue;
          for (const [field, value] of Object.entries(expected)) {
            if (field === "points") expect((found as { points?: unknown[] }).points?.length, name).toBe(value);
            else if (field === "text") {
              expect((found as { text?: Record<string, unknown> }).text, name).toMatchObject(value as object);
            } else expect((found as unknown as Record<string, unknown>)[field], `${name}.${field}`).toEqual(value);
          }
        }
      });
    });
  }
});

describe("tiles conformance: refused maps", () => {
  for (const testCase of suite.invalid) {
    it(`${testCase.file} is refused with ${testCase.code}${testCase.path ? ` at ${testCase.path}` : ""}`, async () => {
      const error = await loadTileMap(read(testCase.file)).then(
        () => undefined,
        (caught: unknown) => caught,
      );
      expect(error).toBeInstanceOf(TilesError);
      const refusal = error as TilesError;
      expect(refusal.code).toBe(testCase.code);
      if (testCase.path !== undefined) {
        expect(refusal.issues).toContainEqual(expect.objectContaining({ code: testCase.code, path: testCase.path }));
      }
    });
  }
});

describe("tiles conformance: the map type", () => {
  it("returns plain data that a second load of the same text repeats", async () => {
    const first = await loadTileMap(read("csv.tmj"));
    const second = await loadTileMap(read("csv.tmj"));
    expect(second.layers.map((layer) => layer.name)).toEqual(first.layers.map((layer) => layer.name));
    expect(second.tilesets[0]?.tiles.get(5)?.properties).toEqual(first.tilesets[0]?.tiles.get(5)?.properties);
    const layer = findLayer(first, "ground") as TileLayer;
    expect(layer.chunks[0]?.gids).toBeInstanceOf(Uint32Array);
    expect(layer.chunks[0]?.flips).toBeInstanceOf(Uint8Array);
  });
});
