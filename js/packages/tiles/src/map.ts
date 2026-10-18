// Helpers for reading a loaded map: find a layer by its name, list the tile layers, and find the tile
// that a global id names. They only read; the map is plain data.

import { findTileset } from "./tileset";
import type { Layer, TileLayer, TileMap, TileRef } from "./types";

/**
 * Finds the first layer with a name. Groups are searched too, depth first, in the order of the file.
 * @returns the layer, or undefined when no layer has the name.
 */
export function findLayer(map: TileMap, name: string): Layer | undefined {
  return findIn(map.layers, name);
}

/** Lists the tile layers of a map, groups included, in the order of the file. */
export function tileLayers(map: TileMap): TileLayer[] {
  const found: TileLayer[] = [];
  collectTileLayers(map.layers, found);
  return found;
}

/**
 * Finds the tile that a global id names: its tileset, its local id, and what the tileset says about it.
 * @param map - the map that the id belongs to.
 * @param gid - a global id with no flip flags, as in `TileChunk.gids`.
 * @returns the reference, or undefined for 0 and for an id that no tileset owns.
 */
export function resolveGid(map: TileMap, gid: number): TileRef | undefined {
  const tileset = findTileset(map.tilesets, gid);
  if (tileset === undefined) return undefined;
  const localId = gid - tileset.firstGid;
  return { tileset, localId, info: tileset.tiles.get(localId) };
}

/** Searches a list of layers, and then the layers of its groups. */
function findIn(layers: readonly Layer[], name: string): Layer | undefined {
  for (const layer of layers) {
    if (layer.name === name) return layer;
    if (layer.kind === "group") {
      const found = findIn(layer.layers, name);
      if (found !== undefined) return found;
    }
  }
  return undefined;
}

/** Adds the tile layers of a list, and of its groups, to `found`. */
function collectTileLayers(layers: readonly Layer[], found: TileLayer[]): void {
  for (const layer of layers) {
    if (layer.kind === "tiles") found.push(layer);
    else if (layer.kind === "group") collectTileLayers(layer.layers, found);
  }
}
