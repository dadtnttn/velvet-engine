// Tilesets: a picture cut into a grid of tiles, read from an embedded tileset of a map or from a .tsj
// file. Each tile may carry properties, a collision group (its object group) and an animation. Image
// collection tilesets (one picture per tile) are refused. Also here: the search of a global id among
// the tilesets, and the frame that an animated tile shows at a time.

import { isRecord, Fields, fieldPath, itemPath, readItems, type Problems } from "./reader";
import { readObjects } from "./objects";
import { readProperties } from "./properties";
import type { TileFrame, TileInfo, Tileset } from "./types";

/**
 * Reads a tileset from its JSON object.
 * @param problems - where the problems go.
 * @param source - the tileset's JSON object (an embedded tileset, or the content of a .tsj file).
 * @param path - the path of the object, which prefixes the paths of its problems.
 * @param firstGid - the global id of its first tile, from the map.
 * @returns the tileset, with no `source`: the map's reader sets it for an external tileset. Undefined only when the
 * tileset is an image collection, which is refused.
 */
export function readTileset(
  problems: Problems,
  source: Record<string, unknown>,
  path: string,
  firstGid: number,
): Tileset | undefined {
  const fields = new Fields(problems, source, path);
  const name = fields.string("name", "");
  if (name === "") fields.fail("value", "a tileset needs a name", "name");
  const tileWidth = fields.int("tilewidth", { min: 1 });
  const tileHeight = fields.int("tileheight", { min: 1 });
  if (!fields.has("image")) {
    fields.fail("unsupported", "image collection tilesets are not supported: use one picture per tileset", "image");
    return undefined;
  }
  const image = fields.string("image", "");
  if (image === "") fields.fail("value", "the tileset's picture needs a path", "image");
  const columns = fields.int("columns", { min: 1 });
  const tileCount = fields.int("tilecount", { min: 1 });
  const imageWidth = fields.int("imagewidth", { min: 0, default: 0 });
  const imageHeight = fields.int("imageheight", { min: 0, default: 0 });
  const margin = fields.int("margin", { min: 0, default: 0 });
  const spacing = fields.int("spacing", { min: 0, default: 0 });
  const offset = fields.object("tileoffset");
  const offsetX = offset?.int("x", { default: 0 }) ?? 0;
  const offsetY = offset?.int("y", { default: 0 }) ?? 0;
  const properties = readProperties(problems, fields.raw("properties"), fields.at("properties"));

  const tiles = new Map<number, TileInfo>();
  const tilesPath = fields.at("tiles");
  fields.array("tiles").forEach((item, index) => {
    const itemAt = itemPath(tilesPath, index);
    if (!isRecord(item)) {
      problems.add("structure", "a tile must be an object", itemAt);
      return;
    }
    const info = readTile(new Fields(problems, item, itemAt), tileCount);
    if (info === undefined) return;
    if (tiles.has(info.id)) {
      problems.add("duplicate", `the tile ${info.id} is listed twice`, fieldPath(itemAt, "id"));
      return;
    }
    tiles.set(info.id, info);
  });

  return {
    name,
    firstGid,
    tileCount,
    tileWidth,
    tileHeight,
    columns,
    image,
    imageWidth,
    imageHeight,
    margin,
    spacing,
    offsetX,
    offsetY,
    properties,
    tiles,
    source: undefined,
  };
}

/**
 * Reads one entry of a tileset's `tiles` list. An entry that says nothing (no type, properties, collision
 * group or animation) is not kept.
 * @returns the tile, or undefined when it says nothing or is an image tile (refused).
 */
function readTile(fields: Fields, tileCount: number): TileInfo | undefined {
  if (fields.has("image")) {
    fields.fail("unsupported", "image collection tiles are not supported", "image");
    return undefined;
  }
  const lastId = Math.max(0, tileCount - 1);
  const id = fields.int("id", { min: 0, max: lastId });
  const type = fields.has("type") ? fields.string("type") : fields.string("class", "");
  const properties = readProperties(fields.recorder, fields.raw("properties"), fields.at("properties"));
  const group = fields.object("objectgroup");
  const collision =
    group === undefined ? [] : readObjects(fields.recorder, group.array("objects"), group.at("objects"));
  const animation: TileFrame[] = readItems(
    fields.recorder,
    fields.array("animation"),
    fields.at("animation"),
    "an animation frame",
    (frame) => ({
      tileId: frame.int("tileid", { min: 0, max: lastId }),
      duration: frame.int("duration", { min: 1 }),
    }),
  );
  if (type === "" && Object.keys(properties).length === 0 && collision.length === 0 && animation.length === 0) {
    return undefined;
  }
  return { id, type, properties, collision, animation };
}

/**
 * Finds the tileset that owns a global id.
 * @param tilesets - the tilesets of a map, in order of their first id.
 * @param gid - a global id with no flip flags.
 * @returns the tileset, or undefined for 0 or for an id that no tileset owns.
 */
export function findTileset(tilesets: readonly Tileset[], gid: number): Tileset | undefined {
  // The last tileset whose first id is not above gid is the only candidate.
  let low = 0;
  let high = tilesets.length - 1;
  let found: Tileset | undefined;
  while (low <= high) {
    const middle = (low + high) >> 1;
    const candidate = tilesets[middle];
    if (candidate === undefined) break;
    if (candidate.firstGid <= gid) {
      found = candidate;
      low = middle + 1;
    } else {
      high = middle - 1;
    }
  }
  if (found === undefined || gid < found.firstGid || gid >= found.firstGid + found.tileCount) return undefined;
  return found;
}

/**
 * The local tile id to show at a time: an animated tile shows its frames in turn, and loops; any other tile
 * shows itself.
 * @param tileset - the tileset of the tile.
 * @param localId - the tile's local id.
 * @param timeMs - the time in milliseconds. Any finite number; the animation loops over its total length.
 */
export function animatedLocalId(tileset: Tileset, localId: number, timeMs: number): number {
  const frames = tileset.tiles.get(localId)?.animation;
  if (frames === undefined || frames.length === 0) return localId;
  let total = 0;
  for (const frame of frames) total += frame.duration;
  let time = timeMs % total;
  if (time < 0) time += total;
  let edge = 0;
  for (const frame of frames) {
    edge += frame.duration;
    if (time < edge) return frame.tileId;
  }
  return localId;
}
