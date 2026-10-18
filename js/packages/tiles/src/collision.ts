// Collision grids: which cells of a map block movement. A cell is empty, solid (the whole cell blocks), or
// partial (only some rectangles inside the cell block). A solid cell can also be one-way: it blocks a box
// that falls onto it from above, and nothing else (see movement.ts).
//
// The grid is built from the tile layers of a map, by any mix of three rules:
// - nonEmpty: every tile of the layers is solid (the usual "collision layer" of a map);
// - solidProperty: a tile whose boolean property is true is solid (for example `solid`);
// - shapes: a tile's collision group gives its solid parts. A rectangle is exact. An ellipse or a polygon is
//   approximated by its bounding box, and a rotated shape by the bounding box of its rotated corners. Points,
//   polylines, text and tile objects have no area, so they are ignored.
// A full solid cell wins over partial parts of the same cell. A cell is one-way only when every rule that
// made it solid says so (oneWayProperty).
//
// Cells of a finite map are the map's cells. An infinite map gets a grid over the bounds of its chunks.
// Cells outside the grid are empty. Layer offsets are ignored: collision uses the cells where the map
// puts them, while drawing shifts them.

import { TilesError } from "./error";
import { findLayer, resolveGid, tileLayers } from "./map";
import type { Box, MapObject, TileLayer, TileMap, TileProperty, Tileset, TileInfo } from "./types";

/** The tolerance for comparing pixel coordinates. */
export const GEOMETRY_EPSILON = 1e-7;

/** The most cells that one grid may have (4096 by 4096). A larger map needs a smaller range of layers. */
const MAX_CELLS = 1 << 24;

/** The parts of a cell that has none. */
const NO_PARTS: readonly Box[] = [];

/** What a cell holds: nothing, a whole solid tile, or some solid rectangles of it. */
export type CellKind = "empty" | "solid" | "partial";

/** Which layers and which rules build a collision grid. The rules add up: a cell is solid when any rule makes it so. */
export interface CollisionOptions {
  /**
   * The tile layers to read, by name or as objects. Default: every tile layer of the map, groups included,
   * whatever its visibility.
   */
  readonly layers?: readonly (string | TileLayer)[];
  /**
   * Every non-empty tile of the layers is solid. Default: true when neither `solidProperty` nor `shapes` is
   * given, and false otherwise.
   */
  readonly nonEmpty?: boolean;
  /** A tile whose boolean property of this name is true is solid. */
  readonly solidProperty?: string;
  /**
   * A tile whose boolean property of this name is true is one-way: a solid tile with this flag blocks only from
   * above.
   */
  readonly oneWayProperty?: string;
  /** The solid parts of each tile's collision group are used. Default false. */
  readonly shapes?: boolean;
}

/** The data that a grid is made of. Grids are made by `buildCollisionGrid`. */
interface GridData {
  readonly tileWidth: number;
  readonly tileHeight: number;
  readonly originX: number;
  readonly originY: number;
  readonly columns: number;
  readonly rows: number;
  /** 0 for an empty cell, 1 for a solid cell, 2 for a partial cell. Row by row. */
  readonly kinds: Uint8Array;
  /** 1 when the solid part of the cell is one-way. Row by row. */
  readonly oneWays: Uint8Array;
  /** The solid rectangles of each partial cell, in cell-local pixels, by cell index. */
  readonly parts: ReadonlyMap<number, readonly Box[]>;
}

/** The solid cells of a map, read by the movement and path functions. */
export class CollisionGrid {
  /** The width of a cell in pixels. */
  readonly tileWidth: number;
  /** The height of a cell in pixels. */
  readonly tileHeight: number;
  /** The column of the grid's first cell, in map cells: 0 for a finite map. */
  readonly originX: number;
  /** The row of the grid's first cell, in map cells: 0 for a finite map. */
  readonly originY: number;
  /** The number of columns of the grid. */
  readonly columns: number;
  /** The number of rows of the grid. */
  readonly rows: number;
  private readonly kinds: Uint8Array;
  private readonly oneWays: Uint8Array;
  private readonly parts: ReadonlyMap<number, readonly Box[]>;

  /** @internal Use `buildCollisionGrid`. */
  constructor(data: GridData) {
    this.tileWidth = data.tileWidth;
    this.tileHeight = data.tileHeight;
    this.originX = data.originX;
    this.originY = data.originY;
    this.columns = data.columns;
    this.rows = data.rows;
    this.kinds = data.kinds;
    this.oneWays = data.oneWays;
    this.parts = data.parts;
  }

  /** True when the cell is inside the grid. Cells outside it are empty. */
  contains(cx: number, cy: number): boolean {
    return this.indexOf(cx, cy) >= 0;
  }

  /** What the cell holds. Cells outside the grid are empty. */
  kindAt(cx: number, cy: number): CellKind {
    const index = this.indexOf(cx, cy);
    if (index < 0) return "empty";
    const kind = this.kinds[index];
    if (kind === 1) return "solid";
    if (kind === 2) return "partial";
    return "empty";
  }

  /** True when the solid part of the cell is one-way. False for an empty cell. */
  isOneWayAt(cx: number, cy: number): boolean {
    const index = this.indexOf(cx, cy);
    return index >= 0 && this.oneWays[index] === 1 && this.kinds[index] !== 0;
  }

  /** The solid rectangles of a partial cell, in pixels from the cell's top-left corner. Empty for other cells. */
  partsAt(cx: number, cy: number): readonly Box[] {
    const index = this.indexOf(cx, cy);
    if (index < 0) return NO_PARTS;
    return this.parts.get(index) ?? NO_PARTS;
  }

  /** The index of a cell in the arrays, or -1 when it is outside the grid. */
  indexOf(cx: number, cy: number): number {
    const column = cx - this.originX;
    const row = cy - this.originY;
    if (column < 0 || row < 0 || column >= this.columns || row >= this.rows) return -1;
    return row * this.columns + column;
  }
}

/**
 * Builds the collision grid of a map.
 * @param map - the loaded map.
 * @param options - the layers and the rules. With none, every tile of every tile layer is solid.
 * @returns the grid.
 * @throws TilesError with code "bad-input" when a named layer does not exist or is not a tile layer, or when the
 * grid would be larger than 16 777 216 cells.
 */
export function buildCollisionGrid(map: TileMap, options: CollisionOptions = {}): CollisionGrid {
  const layers = selectLayers(map, options.layers);
  const shapes = options.shapes ?? false;
  const nonEmpty = options.nonEmpty ?? (options.solidProperty === undefined && !shapes);
  const bounds = gridBounds(map, layers);
  const size = bounds.columns * bounds.rows;
  if (size > MAX_CELLS) {
    throw new TilesError("bad-input", `the collision grid would have ${size} cells, more than ${MAX_CELLS}`);
  }

  const kinds = new Uint8Array(size);
  const oneWays = new Uint8Array(size);
  const parts = new Map<number, Box[]>();

  /** Marks a cell whole-solid. A cell stays one-way only when every rule that made it solid is one-way. */
  const markSolid = (index: number, oneWay: boolean): void => {
    oneWays[index] = (kinds[index] === 0 ? 1 : oneWays[index] ?? 0) & (oneWay ? 1 : 0);
    kinds[index] = 1;
    parts.delete(index);
  };
  /** Adds solid rectangles to a cell, unless the cell is already whole-solid. */
  const addParts = (index: number, rects: Box[], oneWay: boolean): void => {
    if (kinds[index] === 1) {
      oneWays[index] = (oneWays[index] ?? 0) & (oneWay ? 1 : 0);
      return;
    }
    oneWays[index] = (kinds[index] === 0 ? 1 : oneWays[index] ?? 0) & (oneWay ? 1 : 0);
    kinds[index] = 2;
    const existing = parts.get(index);
    if (existing === undefined) parts.set(index, rects);
    else existing.push(...rects);
  };

  for (const layer of layers) {
    for (const chunk of layer.chunks) {
      for (let row = 0; row < chunk.height; row++) {
        for (let column = 0; column < chunk.width; column++) {
          const cell = row * chunk.width + column;
          const gid = chunk.gids[cell] ?? 0;
          if (gid === 0) continue;
          const ref = resolveGid(map, gid);
          if (ref === undefined) continue;
          const cx = chunk.x + column;
          const cy = chunk.y + row;
          const index = (cy - bounds.originY) * bounds.columns + (cx - bounds.originX);
          const oneWay = isTrue(ref.info?.properties, options.oneWayProperty);
          const solid =
            nonEmpty || isTrue(ref.info?.properties, options.solidProperty);
          if (solid) {
            markSolid(index, oneWay);
          } else if (shapes && ref.info !== undefined && ref.info.collision.length > 0) {
            const rects = cellParts(ref.tileset, ref.info, map.tileWidth, map.tileHeight);
            if (rects === "whole") markSolid(index, oneWay);
            else if (rects.length > 0) addParts(index, rects, oneWay);
          }
        }
      }
    }
  }

  return new CollisionGrid({
    tileWidth: map.tileWidth,
    tileHeight: map.tileHeight,
    originX: bounds.originX,
    originY: bounds.originY,
    columns: bounds.columns,
    rows: bounds.rows,
    kinds,
    oneWays,
    parts,
  });
}

/** Resolves the layers named in the options, or every tile layer. */
function selectLayers(map: TileMap, list: CollisionOptions["layers"]): TileLayer[] {
  if (list === undefined) return tileLayers(map);
  return list.map((entry) => {
    if (typeof entry !== "string") {
      if (entry.kind !== "tiles") throw new TilesError("bad-input", `the layer "${entry.name}" is not a tile layer`);
      return entry;
    }
    const layer = findLayer(map, entry);
    if (layer === undefined) throw new TilesError("bad-input", `there is no layer named "${entry}"`);
    if (layer.kind !== "tiles") throw new TilesError("bad-input", `the layer "${entry}" is not a tile layer`);
    return layer;
  });
}

/** The origin and size of the grid: the whole map, or the bounds of the chunks of an infinite map. */
function gridBounds(
  map: TileMap,
  layers: readonly TileLayer[],
): { originX: number; originY: number; columns: number; rows: number } {
  if (!map.infinite) return { originX: 0, originY: 0, columns: map.width, rows: map.height };
  let left = Infinity;
  let top = Infinity;
  let right = -Infinity;
  let bottom = -Infinity;
  for (const layer of layers) {
    for (const chunk of layer.chunks) {
      left = Math.min(left, chunk.x);
      top = Math.min(top, chunk.y);
      right = Math.max(right, chunk.x + chunk.width);
      bottom = Math.max(bottom, chunk.y + chunk.height);
    }
  }
  if (left > right) return { originX: 0, originY: 0, columns: 0, rows: 0 };
  return { originX: left, originY: top, columns: right - left, rows: bottom - top };
}

/** True when the property of this name is a boolean that is true. */
function isTrue(properties: Readonly<Record<string, TileProperty>> | undefined, name: string | undefined): boolean {
  if (properties === undefined || name === undefined) return false;
  const property = properties[name];
  return property?.type === "bool" && property.value;
}

/**
 * The solid parts of a tile in its cell, in cell-local pixels, clipped to the cell. The tile image is drawn
 * with its bottom-left corner on the cell's bottom-left corner, shifted by the tileset's offset (as drawing
 * does), so its shapes are placed the same way.
 * @returns "whole" when one part covers the whole cell, or the rectangles (possibly none).
 */
function cellParts(tileset: Tileset, info: TileInfo, cellWidth: number, cellHeight: number): Box[] | "whole" {
  const originX = tileset.offsetX;
  const originY = cellHeight - tileset.tileHeight + tileset.offsetY;
  const rects: Box[] = [];
  for (const object of info.collision) {
    const bounds = shapeBounds(object);
    if (bounds === undefined) continue;
    const left = clamp(originX + bounds.x, 0, cellWidth);
    const right = clamp(originX + bounds.x + bounds.width, 0, cellWidth);
    const top = clamp(originY + bounds.y, 0, cellHeight);
    const bottom = clamp(originY + bounds.y + bounds.height, 0, cellHeight);
    if (right - left <= GEOMETRY_EPSILON || bottom - top <= GEOMETRY_EPSILON) continue;
    const coversCell =
      left <= GEOMETRY_EPSILON &&
      top <= GEOMETRY_EPSILON &&
      right >= cellWidth - GEOMETRY_EPSILON &&
      bottom >= cellHeight - GEOMETRY_EPSILON;
    if (coversCell) {
      return "whole";
    }
    rects.push({ x: left, y: top, width: right - left, height: bottom - top });
  }
  return rects;
}

/**
 * The bounding box of an object's area, in the tile image's pixels. A rectangle, an ellipse and a polygon have
 * an area; the rest have none. A rotated shape is rotated around its pivot, the top-left corner, and its box
 * is the box of the rotated corners.
 */
function shapeBounds(object: MapObject): Box | undefined {
  switch (object.shape) {
    case "rectangle":
    case "ellipse": {
      const corners = [
        { x: 0, y: 0 },
        { x: object.width, y: 0 },
        { x: object.width, y: object.height },
        { x: 0, y: object.height },
      ];
      return boundsOf(corners, object.x, object.y, object.rotation);
    }
    case "polygon":
      return boundsOf(object.points, object.x, object.y, object.rotation);
    default:
      return undefined;
  }
}

/** The box of a set of points, placed at (`originX`, `originY`) and rotated by `degrees` around that point. */
function boundsOf(
  points: readonly { x: number; y: number }[],
  originX: number,
  originY: number,
  degrees: number,
): Box | undefined {
  if (points.length === 0) return undefined;
  const radians = (degrees * Math.PI) / 180;
  const cos = Math.cos(radians);
  const sin = Math.sin(radians);
  let left = Infinity;
  let top = Infinity;
  let right = -Infinity;
  let bottom = -Infinity;
  for (const point of points) {
    const x = originX + point.x * cos - point.y * sin;
    const y = originY + point.x * sin + point.y * cos;
    left = Math.min(left, x);
    top = Math.min(top, y);
    right = Math.max(right, x);
    bottom = Math.max(bottom, y);
  }
  return { x: left, y: top, width: right - left, height: bottom - top };
}

/** Limits a number to a range. */
function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}
