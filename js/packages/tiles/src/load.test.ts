// Tests for loading a map: the input forms, the header, the tilesets (embedded and external), the typed
// properties, the objects, the tile data that is checked on load, and the errors with their codes and paths.

import { describe, expect, it } from "vitest";
import { TilesError } from "./error";
import { findLayer, resolveGid } from "./map";
import { loadTileMap } from "./load";
import { base64Cells, map, object, objectLayer, terrainTileset, tileLayer } from "./test-support";
import type { ObjectLayer, TileLayer } from "./types";

/** Loads a map that must be refused, and returns the error. */
async function refusal(input: unknown): Promise<TilesError> {
  try {
    await loadTileMap(input as object);
  } catch (error) {
    if (error instanceof TilesError) return error;
    throw error;
  }
  throw new Error("the map was accepted");
}

describe("the input", () => {
  it("accepts the map as text or as a parsed object, with the same result", async () => {
    const text = JSON.stringify(map());
    const fromText = await loadTileMap(text);
    const fromObject = await loadTileMap(map());
    expect(fromText.width).toBe(fromObject.width);
    expect(fromText.tilesets.map((tileset) => tileset.name)).toEqual(["terrain"]);
  });

  it("refuses text that is not JSON with code json", async () => {
    const error = await loadTileMap("{ nope").catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(TilesError);
    expect((error as TilesError).code).toBe("json");
  });

  it("refuses a document that is not an object", async () => {
    const error = await refusal([1, 2]);
    expect(error.code).toBe("structure");
  });

  it("refuses a document whose type is not map", async () => {
    const error = await refusal(map({ extra: { type: "tileset" } }));
    expect(error.issues).toContainEqual(expect.objectContaining({ code: "value", path: "type" }));
  });
});

describe("the header", () => {
  it("reads the sizes, the tile size, the background colour and the properties", async () => {
    const loaded = await loadTileMap(
      map({
        width: 5,
        height: 2,
        extra: {
          backgroundcolor: "#ff102030",
          properties: [{ name: "title", type: "string", value: "Hall" }],
        },
      }),
    );
    expect([loaded.width, loaded.height, loaded.tileWidth, loaded.tileHeight]).toEqual([5, 2, 16, 16]);
    expect(loaded.infinite).toBe(false);
    expect(loaded.backgroundColor).toBe("#ff102030");
    expect(loaded.properties).toEqual({ title: { type: "string", value: "Hall" } });
  });

  it("leaves the background colour undefined when the map sets none", async () => {
    expect((await loadTileMap(map())).backgroundColor).toBeUndefined();
  });

  it("refuses an orientation other than orthogonal", async () => {
    const error = await refusal(map({ extra: { orientation: "isometric" } }));
    expect(error.issues).toContainEqual(expect.objectContaining({ code: "unsupported", path: "orientation" }));
  });

  it("keeps the tilesets in order of their first global id", async () => {
    const second = terrainTileset({ firstgid: 9, name: "props", tilecount: 4, image: "props.png", tiles: [] });
    const loaded = await loadTileMap(map({ tilesets: [second, terrainTileset()] }));
    expect(loaded.tilesets.map((tileset) => [tileset.name, tileset.firstGid])).toEqual([
      ["terrain", 1],
      ["props", 9],
    ]);
  });

  it("reports every problem of a map in one error, with the code of the first", async () => {
    const error = await refusal(
      map({
        extra: { orientation: "isometric" },
        layers: [tileLayer("ground", 4, 3, Array.from({ length: 12 }, () => 1), { opacity: 2 })],
        tilesets: [terrainTileset({ name: "" })],
      }),
    );
    expect(error.issues.length).toBeGreaterThanOrEqual(3);
    expect(error.code).toBe(error.issues[0]?.code);
    expect(error.issues.map((issue) => issue.path)).toEqual(
      expect.arrayContaining(["orientation", "tilesets[0].name", "layers[0].opacity"]),
    );
  });
});

describe("external tilesets", () => {
  it("asks the loader for each external source and reads what it returns", async () => {
    const asked: string[] = [];
    const loaded = await loadTileMap(
      map({
        tilesets: [
          { firstgid: 1, source: "tilesets/terrain.tsj" },
        ],
      }),
      {
        loadTileset: async (source) => {
          asked.push(source);
          return terrainTileset({ firstgid: undefined, name: "external" });
        },
      },
    );
    expect(asked).toEqual(["tilesets/terrain.tsj"]);
    expect(loaded.tilesets[0]?.name).toBe("external");
    expect(loaded.tilesets[0]?.firstGid).toBe(1);
  });

  it("refuses an external tileset when no loader was given", async () => {
    const error = await loadTileMap(map({ tilesets: [{ firstgid: 1, source: "a.tsj" }] })).catch((caught: unknown) => caught);
    expect((error as TilesError).code).toBe("loader");
  });

  it("wraps a loader failure and keeps its cause", async () => {
    const cause = new Error("missing file");
    const error = await loadTileMap(map({ tilesets: [{ firstgid: 1, source: "a.tsj" }] }), {
      loadTileset: async () => {
        throw cause;
      },
    }).catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(TilesError);
    expect((error as TilesError).code).toBe("loader");
    expect((error as TilesError).cause).toBe(cause);
  });

  it("prefixes the paths of problems inside an external tileset with its source", async () => {
    const broken = terrainTileset({ tiles: [{ id: 99, type: "x" }] });
    const failure = await loadTileMap(map({ tilesets: [{ firstgid: 1, source: "tilesets/terrain.tsj" }] }), {
      loadTileset: async () => broken,
    }).catch((caught: unknown) => caught);
    expect((failure as TilesError).issues).toContainEqual(
      expect.objectContaining({ code: "value", path: "[tilesets/terrain.tsj].tiles[0].id" }),
    );
  });

  it("refuses a loaded file that is not a JSON object", async () => {
    const failure = await loadTileMap(map({ tilesets: [{ firstgid: 1, source: "a.tsj" }] }), {
      loadTileset: async () => [] as unknown as object,
    }).catch((caught: unknown) => caught);
    expect((failure as TilesError).issues[0]).toEqual(expect.objectContaining({ code: "structure", path: "[a.tsj]" }));
  });
});

describe("typed properties", () => {
  it("reads every property type with its value", async () => {
    const properties = [
      { name: "plain" },
      { name: "label", type: "string", value: "gate" },
      { name: "count", type: "int", value: 3 },
      { name: "speed", type: "float", value: 0.25 },
      { name: "open", type: "bool", value: true },
      { name: "tint", type: "color", value: "#ff00ff00" },
      { name: "unset", type: "color", value: "" },
      { name: "sprite", type: "file", value: "sprites/gate.png" },
      { name: "target", type: "object", value: 12 },
    ].map((property) => ({ value: "", ...property }));
    const loaded = await loadTileMap(map({ layers: [tileLayer("ground", 4, 3, Array.from({ length: 12 }, () => 1), { properties })] }));
    const layer = findLayer(loaded, "ground") as TileLayer;
    expect(layer.properties).toEqual({
      plain: { type: "string", value: "" },
      label: { type: "string", value: "gate" },
      count: { type: "int", value: 3 },
      speed: { type: "float", value: 0.25 },
      open: { type: "bool", value: true },
      tint: { type: "color", value: "#ff00ff00" },
      unset: { type: "color", value: "" },
      sprite: { type: "file", value: "sprites/gate.png" },
      target: { type: "object", value: 12 },
    });
  });

  it("types the members of a class by their JSON type, and keeps nested classes", async () => {
    const stats = {
      name: "stats",
      type: "class",
      propertytype: "Stats",
      value: { alive: true, hp: 10, inner: { level: 2 }, name: "orc", scale: 0.5 },
    };
    const loaded = await loadTileMap(
      map({ layers: [objectLayer([object({ properties: [stats] })])] }),
    );
    const layer = findLayer(loaded, "objects") as ObjectLayer;
    expect(layer.objects[0]?.properties["stats"]).toEqual({
      type: "class",
      propertyType: "Stats",
      value: {
        alive: { type: "bool", value: true },
        hp: { type: "int", value: 10 },
        inner: {
          type: "class",
          propertyType: "",
          value: { level: { type: "int", value: 2 } },
        },
        name: { type: "string", value: "orc" },
        scale: { type: "float", value: 0.5 },
      },
    });
  });

  it("keeps a member named __proto__ as a plain member", async () => {
    const members = JSON.parse('{"__proto__": {"x": 1}, "hp": 2}') as Record<string, unknown>;
    const loaded = await loadTileMap(
      map({
        layers: [objectLayer([object({ properties: [{ name: "c", type: "class", propertytype: "C", value: members }] })])],
      }),
    );
    const property = (findLayer(loaded, "objects") as ObjectLayer).objects[0]?.properties["c"];
    expect(property?.type === "class" && Object.keys(property.value)).toEqual(["__proto__", "hp"]);
  });

  it("refuses a colour that is not #RRGGBB or #AARRGGBB", async () => {
    const error = await refusal(
      map({ layers: [tileLayer("ground", 4, 3, Array.from({ length: 12 }, () => 1), { properties: [{ name: "c", type: "color", value: "red" }] })] }),
    );
    expect(error.issues).toContainEqual(expect.objectContaining({ code: "value", path: "layers[0].properties[0].value" }));
  });

  it("refuses a property type that the reader does not know", async () => {
    const error = await refusal(
      map({ layers: [tileLayer("ground", 4, 3, Array.from({ length: 12 }, () => 1), { properties: [{ name: "v", type: "vector", value: 1 }] })] }),
    );
    expect(error.issues).toContainEqual(expect.objectContaining({ code: "unsupported", path: "layers[0].properties[0].type" }));
  });

  it("refuses two properties with one name", async () => {
    const error = await refusal(
      map({
        layers: [
          tileLayer("ground", 4, 3, Array.from({ length: 12 }, () => 1), {
            properties: [
              { name: "a", type: "int", value: 1 },
              { name: "a", type: "int", value: 2 },
            ],
          }),
        ],
      }),
    );
    expect(error.issues).toContainEqual(expect.objectContaining({ code: "duplicate", path: "layers[0].properties[1].name" }));
  });
});

describe("object layers", () => {
  it("reads each shape, its position, its rotation and its text", async () => {
    const loaded = await loadTileMap(
      map({
        layers: [
          objectLayer([
            object({ id: 1, name: "room", type: "zone", x: 4, y: 8, width: 20, height: 10, rotation: 45 }),
            object({ id: 2, point: true, width: 0, height: 0 }),
            object({ id: 3, ellipse: true }),
            object({ id: 4, polygon: [{ x: 0, y: 0 }, { x: 4, y: 0 }, { x: 0, y: 4 }] }),
            object({ id: 5, polyline: [{ x: 0, y: 0 }, { x: 4, y: 4 }] }),
            object({ id: 6, text: { text: "Hi" } }),
          ]),
        ],
      }),
    );
    const objects = (findLayer(loaded, "objects") as ObjectLayer).objects;
    expect(objects.map((item) => item.shape)).toEqual([
      "rectangle",
      "point",
      "ellipse",
      "polygon",
      "polyline",
      "text",
    ]);
    expect(objects[0]).toMatchObject({ id: 1, type: "zone", x: 4, y: 8, width: 20, height: 10, rotation: 45 });
    const text = objects[5];
    expect(text?.shape === "text" && text.text).toEqual({
      text: "Hi",
      fontFamily: "sans-serif",
      pixelSize: 16,
      wrap: false,
      color: "#000000",
      bold: false,
      italic: false,
      underline: false,
      strikeout: false,
      kerning: true,
      halign: "left",
      valign: "top",
    });
  });

  it("refuses a polygon with fewer than three points", async () => {
    const error = await refusal(
      map({ layers: [objectLayer([object({ polygon: [{ x: 0, y: 0 }, { x: 4, y: 0 }] })])] }),
    );
    expect(error.issues).toContainEqual(expect.objectContaining({ code: "value", path: "layers[0].objects[0].polygon" }));
  });

  it("reads a tile object with its flip flags and checks that its tile exists", async () => {
    const loaded = await loadTileMap(
      map({ layers: [objectLayer([object({ gid: 0x40000000 + 8, x: 16, y: 32 })])] }),
    );
    const item = (findLayer(loaded, "objects") as ObjectLayer).objects[0];
    expect(item).toMatchObject({ shape: "tile", gid: 8, flips: 2, x: 16, y: 32 });

    const error = await refusal(map({ layers: [objectLayer([object({ gid: 40 })])] }));
    expect(error.issues).toContainEqual(expect.objectContaining({ code: "value", path: "layers[0].objects[0].gid" }));
  });

  it("refuses the hexagonal rotation flag on a tile", async () => {
    const error = await refusal(map({ layers: [tileLayer("ground", 4, 3, Array.from({ length: 12 }, () => 0x10000002))] }));
    expect(error.issues).toContainEqual(expect.objectContaining({ code: "value", path: "layers[0].data" }));
  });

  it("refuses an object template", async () => {
    const error = await refusal(map({ layers: [objectLayer([object({ template: "chest.tx" })])] }));
    expect(error.issues).toContainEqual(expect.objectContaining({ code: "unsupported", path: "layers[0].objects[0].template" }));
  });

  it("reads the draw order and keeps the objects in the file's order", async () => {
    const loaded = await loadTileMap(
      map({
        layers: [
          objectLayer([object({ id: 3, name: "c" }), object({ id: 4, name: "a" })], { draworder: "index" }),
        ],
      }),
    );
    const layer = findLayer(loaded, "objects") as ObjectLayer;
    expect(layer.drawOrder).toBe("index");
    expect(layer.objects.map((item) => item.name)).toEqual(["c", "a"]);
  });
});

describe("tile data and tilesets", () => {
  it("decodes base64 data written in the map", async () => {
    const cells = [2, 2, 2, 2, 1, 1, 1, 1, 0, 0, 0, 0];
    const loaded = await loadTileMap(
      map({ layers: [tileLayer("ground", 4, 3, [], { data: base64Cells(cells), encoding: "base64" })] }),
    );
    const chunk = (findLayer(loaded, "ground") as TileLayer).chunks[0];
    expect(Array.from(chunk?.gids ?? [])).toEqual(cells);
  });

  it("refuses a layer whose cells name a tile that no tileset has", async () => {
    const error = await refusal(map({ layers: [tileLayer("ground", 4, 3, [2, 2, 2, 2, 99, 0, 0, 0, 0, 0, 0, 0])] }));
    expect(error.issues).toContainEqual(expect.objectContaining({ code: "value", path: "layers[0].data" }));
  });

  it("refuses two tilesets whose tile ids overlap", async () => {
    const error = await refusal(
      map({ tilesets: [terrainTileset(), terrainTileset({ firstgid: 5, name: "other", image: "other.png" })] }),
    );
    expect(error.issues).toContainEqual(expect.objectContaining({ code: "value", path: "tilesets[1].firstgid" }));
  });

  it("reads the tile properties, collision group and animation of a tile", async () => {
    const loaded = await loadTileMap(map());
    const tileset = loaded.tilesets[0];
    expect(tileset?.tiles.get(1)?.properties).toEqual({ solid: { type: "bool", value: true } });
    expect(tileset?.tiles.get(2)?.collision).toHaveLength(1);
    expect(tileset?.tiles.get(3)?.animation).toEqual([
      { tileId: 3, duration: 200 },
      { tileId: 4, duration: 100 },
    ]);
    expect(tileset?.tiles.get(0)).toBeUndefined();
  });

  it("drops a tile entry that says nothing", async () => {
    const loaded = await loadTileMap(map({ tilesets: [terrainTileset({ tiles: [{ id: 0 }] })] }));
    expect(loaded.tilesets[0]?.tiles.size).toBe(0);
  });

  it("refuses an animation frame that names a tile outside the tileset", async () => {
    const error = await refusal(
      map({ tilesets: [terrainTileset({ tiles: [{ id: 0, animation: [{ duration: 10, tileid: 8 }] }] })] }),
    );
    expect(error.issues).toContainEqual(
      expect.objectContaining({ code: "value", path: "tilesets[0].tiles[0].animation[0].tileid" }),
    );
  });

  it("resolves a global id to its tileset, local id and tile information", async () => {
    const loaded = await loadTileMap(map());
    expect(resolveGid(loaded, 0)).toBeUndefined();
    expect(resolveGid(loaded, 9)).toBeUndefined();
    const ref = resolveGid(loaded, 4);
    expect(ref?.localId).toBe(3);
    expect(ref?.info?.animation).toHaveLength(2);
  });

  it("reads a finite layer's kind, flags and effective values", async () => {
    const loaded = await loadTileMap(map());
    const layer = findLayer(loaded, "ground") as TileLayer;
    expect(layer).toMatchObject({ kind: "tiles", visible: true, opacity: 1, offsetX: 0, offsetY: 0, name: "ground" });
    expect(layer.chunks).toHaveLength(1);
  });

  it("refuses a layer whose type is missing, and reports it at its path", async () => {
    const error = await refusal(map({ layers: [{ name: "x", visible: true }] }));
    expect(error.issues).toContainEqual(expect.objectContaining({ code: "structure", path: "layers[0].type" }));
  });

  it("refuses a tileset without a name", async () => {
    const error = await refusal(map({ tilesets: [terrainTileset({ name: "" })] }));
    expect(error.issues).toContainEqual(expect.objectContaining({ code: "value", path: "tilesets[0].name" }));
  });

  it("keeps the source path of an external tileset, and none for an embedded one", async () => {
    const loaded = await loadTileMap(
      map({ tilesets: [terrainTileset({ name: "inner" }), { firstgid: 9, source: "tilesets/props.tsj" }] }),
      { loadTileset: async () => terrainTileset({ name: "props" }) },
    );
    expect(loaded.tilesets.map((tileset) => [tileset.name, tileset.source])).toEqual([
      ["inner", undefined],
      ["props", "tilesets/props.tsj"],
    ]);
  });
});

describe("layer parallax, tint and the parallax origin", () => {
  it("gives a map with no settings an origin of 0 and 0, and layers no parallax and no tint", async () => {
    const loaded = await loadTileMap(map());
    expect(loaded.parallaxOrigin).toEqual({ x: 0, y: 0 });
    expect(findLayer(loaded, "ground")).toMatchObject({ parallax: { x: 1, y: 1 }, tint: null });
  });

  it("reads the parallax origin of the map", async () => {
    const loaded = await loadTileMap(map({ extra: { parallaxoriginx: 32, parallaxoriginy: -8 } }));
    expect(loaded.parallaxOrigin).toEqual({ x: 32, y: -8 });
  });

  it("multiplies the parallax factors of nested groups into their layers, and keeps a group's tint to the group", async () => {
    const cells = Array.from({ length: 12 }, () => 1);
    const inner = tileLayer("inner", 4, 3, cells, { parallaxx: 0.5, parallaxy: 2 });
    const middle = { id: 3, name: "middle", type: "group", visible: true, opacity: 1, x: 0, y: 0, parallaxx: 0.5, layers: [inner] };
    const outer = { id: 2, name: "outer", type: "group", visible: true, opacity: 1, x: 0, y: 0, parallaxx: 0.5, tintcolor: "#ff0000ff", layers: [middle] };
    const loaded = await loadTileMap(map({ layers: [outer] }));
    expect(findLayer(loaded, "inner")).toMatchObject({ parallax: { x: 0.125, y: 2 }, tint: null });
    expect(findLayer(loaded, "outer")).toMatchObject({ parallax: { x: 0.5, y: 1 }, tint: "#ff0000ff" });
  });

  it("keeps a layer's tint as written, and refuses a tint that is not a colour, at the layer's tintcolor", async () => {
    const cells = Array.from({ length: 12 }, () => 1);
    const written = await loadTileMap(map({ layers: [tileLayer("ground", 4, 3, cells, { tintcolor: "#80a0c0" })] }));
    expect(findLayer(written, "ground")).toMatchObject({ tint: "#80a0c0" });
    const error = await refusal(map({ layers: [tileLayer("ground", 4, 3, cells, { tintcolor: "red" })] }));
    expect(error.issues).toContainEqual(expect.objectContaining({ code: "value", path: "layers[0].tintcolor" }));
  });
});
