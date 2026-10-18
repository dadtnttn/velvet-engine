// Tests for the queries of a loaded map: finding a layer by name (the first one, depth first, groups included),
// listing the tile layers in the order of the file, and the zero value of a property that has no value.

import { describe, expect, it } from "vitest";
import { loadTileMap } from "./load";
import { findLayer, tileLayers } from "./map";
import { map, objectLayer, object, tileLayer } from "./test-support";
import type { TileLayer } from "./types";

/** A map with a group that holds two tile layers, and a second tile layer that repeats one of their names. */
async function nested() {
  return loadTileMap(
    map({
      layers: [
        tileLayer("ground", 4, 3, Array.from({ length: 12 }, () => 1)),
        {
          id: 5,
          layers: [
            tileLayer("walls", 4, 3, Array.from({ length: 12 }, () => 2), { opacity: 0.5 }),
            { id: 6, layers: [tileLayer("ground", 4, 3, Array.from({ length: 12 }, () => 0))], name: "deep", type: "group" },
          ],
          name: "outer",
          offsetx: 4,
          type: "group",
          visible: true,
        },
        objectLayer([object({ name: "spawn" })]),
      ],
    }),
  );
}

describe("finding a layer by name", () => {
  it("searches the groups, depth first, and returns the first layer of the name", async () => {
    const loaded = await nested();
    const ground = findLayer(loaded, "ground") as TileLayer;
    expect(ground.chunks[0]?.gids[0]).toBe(1);
    expect((findLayer(loaded, "walls") as TileLayer).kind).toBe("tiles");
    expect(findLayer(loaded, "missing")).toBeUndefined();
  });
});

describe("listing the tile layers", () => {
  it("lists every tile layer in the order of the file, groups included, and no other layer", async () => {
    const loaded = await nested();
    expect(tileLayers(loaded).map((layer) => layer.name)).toEqual(["ground", "walls", "ground"]);
  });

  it("gives each listed layer the opacity and offset of its groups", async () => {
    const loaded = await nested();
    const [, walls] = tileLayers(loaded);
    expect(walls).toMatchObject({ opacity: 0.5, offsetX: 4, offsetY: 0 });
  });
});

describe("property values that are left out", () => {
  it("gives a property with no value the zero value of its type", async () => {
    const loaded = await loadTileMap(
      map({
        layers: [
          tileLayer("ground", 4, 3, Array.from({ length: 12 }, () => 1), {
            properties: [
              { name: "a", type: "int" },
              { name: "b", type: "bool" },
              { name: "c" },
            ],
          }),
        ],
      }),
    );
    expect((findLayer(loaded, "ground") as TileLayer).properties).toEqual({
      a: { type: "int", value: 0 },
      b: { type: "bool", value: false },
      c: { type: "string", value: "" },
    });
  });
});
