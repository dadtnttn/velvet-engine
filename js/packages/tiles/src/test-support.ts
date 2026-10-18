// Helpers for the tests: small maps written as JSON objects, in Tiled's shape, so that each test states only
// the part that it checks. The test tileset has eight 16-pixel tiles, four to a row. Local ids: 0 grass;
// 1 stone (a solid property); 2 ledge (a collision rectangle over its top six pixels); 3 water (animated,
// frames 3 and 4 for 200 and 100 ms); 5 platform (solid and one-way); 7 spikes (a damage property).
// Global ids are the local ids plus one, so stone is 2 and spikes are 8.

/** Options for {@link map}. */
export interface MapOptions {
  /** The width in cells. Default 4. */
  width?: number;
  /** The height in cells. Default 3. */
  height?: number;
  /** The layers. Default: one tile layer "ground", stone along the top row and grass below. */
  layers?: Record<string, unknown>[];
  /** The tilesets. Default: the test tileset. */
  tilesets?: unknown[];
  /** Other fields of the map, copied in as they are. */
  extra?: Record<string, unknown>;
}

/** The embedded tileset that the test maps use. `extra` replaces or adds fields. */
export function terrainTileset(extra: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    columns: 4,
    firstgid: 1,
    image: "terrain.png",
    imageheight: 32,
    imagewidth: 64,
    margin: 0,
    name: "terrain",
    spacing: 0,
    tilecount: 8,
    tileheight: 16,
    tilewidth: 16,
    tiles: [
      { id: 1, properties: [{ name: "solid", type: "bool", value: true }] },
      {
        id: 2,
        objectgroup: {
          draworder: "index",
          name: "",
          objects: [{ height: 6, id: 1, name: "", rotation: 0, type: "", visible: true, width: 16, x: 0, y: 0 }],
          type: "objectgroup",
          visible: true,
          x: 0,
          y: 0,
        },
      },
      {
        id: 3,
        animation: [
          { duration: 200, tileid: 3 },
          { duration: 100, tileid: 4 },
        ],
      },
      {
        id: 5,
        properties: [
          { name: "oneWay", type: "bool", value: true },
          { name: "solid", type: "bool", value: true },
        ],
      },
      { id: 7, properties: [{ name: "damage", type: "int", value: 3 }] },
    ],
    type: "tileset",
    version: "1.10",
    ...extra,
  };
}

/** A finite tile layer with csv data. `extra` replaces or adds fields. */
export function tileLayer(
  name: string,
  width: number,
  height: number,
  data: number[],
  extra: Record<string, unknown> = {},
): Record<string, unknown> {
  return { data, height, id: 1, name, opacity: 1, type: "tilelayer", visible: true, width, x: 0, y: 0, ...extra };
}

/** A finite map of 16-pixel cells, with the test tileset and the given layers or a default layer. */
export function map(options: MapOptions = {}): Record<string, unknown> {
  const width = options.width ?? 4;
  const height = options.height ?? 3;
  const data = Array.from({ length: width * height }, (_, index) => (index < width ? 2 : 1));
  return {
    height,
    infinite: false,
    layers: options.layers ?? [tileLayer("ground", width, height, data)],
    nextlayerid: 2,
    nextobjectid: 1,
    orientation: "orthogonal",
    renderorder: "right-down",
    tilesets: options.tilesets ?? [terrainTileset()],
    tileheight: 16,
    tilewidth: 16,
    type: "map",
    version: "1.10",
    width,
    ...options.extra,
  };
}

/** An object as Tiled writes it: a 16-pixel rectangle at the origin, unless `extra` says otherwise. */
export function object(extra: Record<string, unknown> = {}): Record<string, unknown> {
  return { id: 1, name: "", rotation: 0, visible: true, width: 16, height: 16, x: 0, y: 0, ...extra };
}

/** An object layer that holds the given objects. */
export function objectLayer(
  objects: Record<string, unknown>[],
  extra: Record<string, unknown> = {},
): Record<string, unknown> {
  return {
    draworder: "topdown",
    id: 2,
    name: "objects",
    objects,
    opacity: 1,
    type: "objectgroup",
    visible: true,
    x: 0,
    y: 0,
    ...extra,
  };
}

/** Encodes cells as Tiled's base64 text without compression: little-endian 32-bit numbers. */
export function base64Cells(ids: number[]): string {
  const bytes = Buffer.alloc(ids.length * 4);
  ids.forEach((id, index) => bytes.writeUInt32LE(id, index * 4));
  return bytes.toString("base64");
}
