// Reads the layers of a map, a group at a time. A tile layer is read in one of two forms: a finite layer has
// its data at its own size, and an infinite layer has chunks. Each layer gets the values it is drawn with:
// the visibility and the opacity of its groups are combined into it, and so are their offsets. Parallax
// and tint are refused, because drawing does not apply them.

import { decodeCells } from "./data";
import { readObjects } from "./objects";
import { readProperties } from "./properties";
import { Fields, isRecord, itemPath, type Problems } from "./reader";
import { findTileset } from "./tileset";
import type {
  ImageLayer,
  Layer,
  ObjectLayer,
  TileChunk,
  TileLayer,
  TileProperties,
  Tileset,
} from "./types";

/** What the map gives to every layer that is read: the sizes and the tilesets. */
export interface LayerContext {
  readonly problems: Problems;
  /** The tilesets, in order of their first id. */
  readonly tilesets: readonly Tileset[];
  /** The map's width in cells. Zero for an infinite map. */
  readonly width: number;
  /** The map's height in cells. Zero for an infinite map. */
  readonly height: number;
  readonly infinite: boolean;
}

/** The values that a layer inherits from the groups that hold it. */
interface Inherited {
  readonly visible: boolean;
  readonly opacity: number;
  readonly offsetX: number;
  readonly offsetY: number;
}

/** What the top level of a map gives to its layers: everything shown, opaque, not offset. */
export const TOP_LEVEL: Inherited = { visible: true, opacity: 1, offsetX: 0, offsetY: 0 };

/** The values that every layer shares, before its kind is known. */
interface LayerBase {
  readonly id: number;
  readonly name: string;
  readonly visible: boolean;
  readonly opacity: number;
  readonly offsetX: number;
  readonly offsetY: number;
  readonly properties: TileProperties;
}

/**
 * Reads a list of layers.
 * @param ctx - the map's context.
 * @param list - the JSON list.
 * @param path - the path of the list.
 * @param inherited - the values of the group that holds the list.
 */
export async function readLayers(
  ctx: LayerContext,
  list: readonly unknown[],
  path: string,
  inherited: Inherited,
): Promise<Layer[]> {
  const layers: Layer[] = [];
  for (const [index, item] of list.entries()) {
    const layer = await readLayer(ctx, item, itemPath(path, index), inherited);
    if (layer !== undefined) layers.push(layer);
  }
  return layers;
}

/** Reads one layer of any kind. Undefined when the layer is broken (the problem is recorded). */
async function readLayer(
  ctx: LayerContext,
  raw: unknown,
  path: string,
  inherited: Inherited,
): Promise<Layer | undefined> {
  if (!isRecord(raw)) {
    ctx.problems.add("structure", "a layer must be an object", path);
    return undefined;
  }
  const fields = new Fields(ctx.problems, raw, path);
  const type = fields.string("type");
  const own = {
    visible: fields.bool("visible", true),
    opacity: fields.number("opacity", { min: 0, max: 1, default: 1 }),
    offsetX: fields.number("offsetx", { default: 0 }),
    offsetY: fields.number("offsety", { default: 0 }),
  };
  if (fields.number("parallaxx", { default: 1 }) !== 1 || fields.number("parallaxy", { default: 1 }) !== 1) {
    fields.fail("unsupported", "parallax is not supported: leave parallaxx and parallaxy at 1", "parallaxx");
  }
  if (fields.string("tintcolor", "") !== "") {
    fields.fail("unsupported", "a layer tint is not supported", "tintcolor");
  }
  // The values the layer is drawn with: its groups are already combined into `inherited`.
  const effective: Inherited = {
    visible: inherited.visible && own.visible,
    opacity: inherited.opacity * own.opacity,
    offsetX: inherited.offsetX + own.offsetX,
    offsetY: inherited.offsetY + own.offsetY,
  };
  const base: LayerBase = {
    id: fields.int("id", { min: 0, default: 0 }),
    name: fields.string("name", ""),
    ...effective,
    properties: readProperties(ctx.problems, fields.raw("properties"), fields.at("properties")),
  };

  switch (type) {
    case "tilelayer":
      return readTileLayer(ctx, fields, base);
    case "objectgroup":
      return readObjectLayer(ctx, fields, base);
    case "imagelayer":
      return readImageLayer(fields, base);
    case "group":
      return readGroup(ctx, fields, base, effective);
    case "":
      return undefined;
    default:
      fields.fail(
        "value",
        `the layer type "${type}" is not one of tilelayer, objectgroup, imagelayer or group`,
        "type",
      );
      return undefined;
  }
}

/** Reads a tile layer: its cells, in one block for a finite map or in chunks for an infinite map. */
async function readTileLayer(ctx: LayerContext, fields: Fields, base: LayerBase): Promise<TileLayer | undefined> {
  const encoding = fields.choice("encoding", ["csv", "base64"] as const, "csv");
  const compression = fields.choice("compression", ["", "gzip", "zlib", "zstd"] as const, "");
  const knownGid = (gid: number): boolean => findTileset(ctx.tilesets, gid) !== undefined;

  if (!ctx.infinite) {
    const width = fields.int("width", { min: 1 });
    const height = fields.int("height", { min: 1 });
    if (width !== ctx.width || height !== ctx.height) {
      fields.fail(
        "value",
        `the layer is ${width} x ${height} cells, but the map is ${ctx.width} x ${ctx.height}`,
        "width",
      );
    }
    if (!fields.has("data")) {
      fields.fail("structure", '"data" is required for a finite map', "data");
      return undefined;
    }
    const cells = await decodeCells(ctx.problems, {
      data: fields.raw("data"),
      encoding,
      compression,
      width: ctx.width,
      height: ctx.height,
      path: fields.at("data"),
      knownGid,
    });
    if (cells === undefined) return undefined;
    const chunk: TileChunk = {
      x: 0,
      y: 0,
      width: ctx.width,
      height: ctx.height,
      gids: cells.gids,
      flips: cells.flips,
    };
    return { ...base, kind: "tiles", chunks: [chunk] };
  }

  const chunks: TileChunk[] = [];
  const chunksPath = fields.at("chunks");
  for (const [index, item] of fields.array("chunks").entries()) {
    const chunk = await readChunk(ctx, item, itemPath(chunksPath, index), encoding, compression);
    if (chunk !== undefined) chunks.push(chunk);
  }
  return { ...base, kind: "tiles", chunks };
}

/** Reads one chunk of an infinite tile layer. */
async function readChunk(
  ctx: LayerContext,
  raw: unknown,
  path: string,
  encoding: "csv" | "base64",
  compression: "" | "gzip" | "zlib" | "zstd",
): Promise<TileChunk | undefined> {
  if (!isRecord(raw)) {
    ctx.problems.add("structure", "a chunk must be an object", path);
    return undefined;
  }
  const fields = new Fields(ctx.problems, raw, path);
  const x = fields.int("x");
  const y = fields.int("y");
  const width = fields.int("width", { min: 1 });
  const height = fields.int("height", { min: 1 });
  if (!fields.has("data")) {
    fields.fail("structure", '"data" is required', "data");
    return undefined;
  }
  const cells = await decodeCells(ctx.problems, {
    data: fields.raw("data"),
    encoding,
    compression,
    width,
    height,
    path: fields.at("data"),
    knownGid: (gid: number): boolean => findTileset(ctx.tilesets, gid) !== undefined,
  });
  if (cells === undefined) return undefined;
  return { x, y, width, height, gids: cells.gids, flips: cells.flips };
}

/** Reads an object layer: its objects, drawn by their y when `draworder` is topdown. */
function readObjectLayer(ctx: LayerContext, fields: Fields, base: LayerBase): ObjectLayer {
  const drawOrder = fields.choice("draworder", ["topdown", "index"] as const, "topdown");
  const objects = readObjects(ctx.problems, fields.array("objects"), fields.at("objects"), {
    knownGid: (gid: number): boolean => findTileset(ctx.tilesets, gid) !== undefined,
  });
  return { ...base, kind: "objects", drawOrder, objects };
}

/** Reads an image layer. Its picture is kept as a path; drawing it is up to the game. */
function readImageLayer(fields: Fields, base: LayerBase): ImageLayer {
  const image = fields.string("image");
  const repeatX = fields.bool("repeatx", false);
  const repeatY = fields.bool("repeaty", false);
  const transparent = fields.string("transparentcolor", "");
  return {
    ...base,
    kind: "image",
    image,
    repeatX,
    repeatY,
    transparentColor: transparent === "" ? undefined : transparent,
  };
}

/** Reads a group. Its layers get the group's values, combined with the values it inherited. */
async function readGroup(
  ctx: LayerContext,
  fields: Fields,
  base: LayerBase,
  effective: Inherited,
): Promise<Layer> {
  const layers = await readLayers(ctx, fields.array("layers", true), fields.at("layers"), effective);
  return { ...base, kind: "group", layers };
}
