// Swept box movement against a collision grid. The box moves along x first, then along y, as the caller
// asks. Each axis is walked in steps no longer than one tile, so the box never skips a tile, however fast
// it goes. Within a step the contact is exact: the box stops against the first solid rectangle that it
// would reach, flush with its face. A box that already overlaps a solid rectangle in the direction of
// movement passes it, so a box that starts inside a wall is not stuck there.
//
// One-way cells block a box only when it falls onto them from above (its bottom was at or above their top).
// They never block sideways movement or upward movement. Slopes are not supported: every solid rectangle
// is a box with flat faces.

import { CollisionGrid, GEOMETRY_EPSILON } from "./collision";
import { TilesError } from "./error";
import type { Box } from "./types";

/** One side of a box. */
export type Side = "left" | "right" | "top" | "bottom";

/** A cell that stopped a box, and the side of the box that touched it. */
export interface TouchedTile {
  /** The column of the cell, in map cells. */
  readonly x: number;
  /** The row of the cell, in map cells. */
  readonly y: number;
  /** The side of the box that touched the cell. */
  readonly side: Side;
}

/** Options for `moveBox`. */
export interface MoveOptions {
  /** When true, one-way cells do not block at all, so a box can fall through them. Default false. */
  readonly ignoreOneWay?: boolean;
}

/** What a move gives back. */
export interface MoveResult {
  /** The box after the move. It stops flush against what it hit. */
  readonly box: Box;
  /**
   * For each side, whether the move stopped against something on that side. A move of zero distance still
   * reports the contacts: `moveBox(grid, box, 0, 1)` tells whether the box stands on the ground.
   */
  readonly contacts: Readonly<Record<Side, boolean>>;
  /** The cells that stopped the box, with the side they touched. Every cell at the contact distance is listed. */
  readonly tiles: readonly TouchedTile[];
}

/** The result of one step along an axis. */
interface Slide {
  /** The signed distance moved: all of the step, or up to the contact. */
  readonly travelled: number;
  /** True when a solid rectangle stopped the step. */
  readonly hit: boolean;
  /** The cells that stopped the step, at the contact distance. */
  readonly cells: readonly { readonly x: number; readonly y: number }[];
}

/**
 * Moves a box across a grid.
 * @param grid - the collision grid.
 * @param box - the box to move, with its top-left corner and size in pixels.
 * @param dx - the distance to move along x, in pixels. Positive is right.
 * @param dy - the distance to move along y, in pixels. Positive is down.
 * @param options - see {@link MoveOptions}.
 * @returns the box after the move, its contacts, and the cells that stopped it.
 * @throws TilesError with code "bad-input" when a distance or a box value is not a finite number, or when a size
 * is negative.
 */
export function moveBox(
  grid: CollisionGrid,
  box: Box,
  dx: number,
  dy: number,
  options: MoveOptions = {},
): MoveResult {
  checkBox(box);
  if (!Number.isFinite(dx) || !Number.isFinite(dy)) {
    throw new TilesError("bad-input", `the distances must be finite numbers, got ${dx} and ${dy}`);
  }
  const ignoreOneWay = options.ignoreOneWay ?? false;
  const contacts = { left: false, right: false, top: false, bottom: false };
  const tiles: TouchedTile[] = [];
  let current: Box = { x: box.x, y: box.y, width: box.width, height: box.height };

  current = walk(grid, current, "x", dx, ignoreOneWay, contacts, tiles);
  current = walk(grid, current, "y", dy, ignoreOneWay, contacts, tiles);
  return { box: current, contacts, tiles };
}

/**
 * Walks a box along one axis in steps of at most one tile, stopping at the first contact.
 * @returns the box after the walk.
 */
function walk(
  grid: CollisionGrid,
  start: Box,
  axis: "x" | "y",
  distance: number,
  ignoreOneWay: boolean,
  contacts: Record<Side, boolean>,
  tiles: TouchedTile[],
): Box {
  const total = Math.abs(distance);
  const direction = Math.sign(distance);
  if (total === 0) return start;
  const stepLength = axis === "x" ? grid.tileWidth : grid.tileHeight;
  let moved = 0;
  let box = start;
  while (moved < total) {
    const step = Math.min(stepLength, total - moved);
    const slide = slideStep(grid, box, axis, direction * step, ignoreOneWay);
    if (slide.hit) {
      box = axis === "x" ? { ...box, x: box.x + slide.travelled } : { ...box, y: box.y + slide.travelled };
      const side = contactSide(axis, direction);
      contacts[side] = true;
      for (const cell of slide.cells) tiles.push({ x: cell.x, y: cell.y, side });
      return box;
    }
    moved += step;
    // Positions come from the start, so that many small steps do not drift.
    box = axis === "x" ? { ...start, x: start.x + direction * moved } : { ...start, y: start.y + direction * moved };
  }
  return box;
}

/** The side of the box that a move along an axis touches when it stops. */
function contactSide(axis: "x" | "y", direction: number): Side {
  if (axis === "x") return direction > 0 ? "right" : "left";
  return direction > 0 ? "bottom" : "top";
}

/**
 * One step of a box along one axis, of at most one tile: the swept area is the cells that the box passes
 * through, and the nearest solid rectangle in that area decides where the box stops.
 */
function slideStep(
  grid: CollisionGrid,
  box: Box,
  axis: "x" | "y",
  distance: number,
  ignoreOneWay: boolean,
): Slide {
  const direction = Math.sign(distance);
  const amount = Math.abs(distance);
  const tileWidth = grid.tileWidth;
  const tileHeight = grid.tileHeight;

  // The range of the swept area along the moving axis, and across it.
  const alongStart = axis === "x" ? box.x : box.y;
  const alongLength = axis === "x" ? box.width : box.height;
  const sweepStart = direction > 0 ? alongStart : alongStart - amount;
  const sweepEnd = (direction > 0 ? alongStart + amount : alongStart) + alongLength;
  const acrossStart = axis === "x" ? box.y : box.x;
  const acrossEnd = acrossStart + (axis === "x" ? box.height : box.width);

  const cellAlong = axis === "x" ? tileWidth : tileHeight;
  const cellAcross = axis === "x" ? tileHeight : tileWidth;
  const firstAlong = Math.floor(sweepStart / cellAlong);
  const lastAlong = Math.max(firstAlong, Math.ceil(sweepEnd / cellAlong) - 1);
  const firstAcross = Math.floor(acrossStart / cellAcross);
  const lastAcross = Math.max(firstAcross, Math.ceil(acrossEnd / cellAcross) - 1);

  let best = Infinity;
  let cells: { x: number; y: number }[] = [];
  const consider = (move: number, column: number, row: number): void => {
    if (move < best - GEOMETRY_EPSILON) {
      best = move;
      cells = [{ x: column, y: row }];
    } else if (move <= best + GEOMETRY_EPSILON) {
      if (!cells.some((cell) => cell.x === column && cell.y === row)) cells.push({ x: column, y: row });
    }
  };

  for (let row = firstAcross; row <= lastAcross; row++) {
    for (let along = firstAlong; along <= lastAlong; along++) {
      const column = axis === "x" ? along : row;
      const cellRow = axis === "x" ? row : along;
      const kind = grid.kindAt(column, cellRow);
      if (kind === "empty") continue;
      const oneWay = grid.isOneWayAt(column, cellRow);
      if (oneWay && (axis === "x" || direction < 0 || ignoreOneWay)) continue;
      if (kind === "solid") {
        const rect = { x: column * tileWidth, y: cellRow * tileHeight, width: tileWidth, height: tileHeight };
        const move = contactDistance(box, rect, axis, direction, amount);
        if (move !== undefined) consider(move, column, cellRow);
        continue;
      }
      for (const part of grid.partsAt(column, cellRow)) {
        const rect = {
          x: column * tileWidth + part.x,
          y: cellRow * tileHeight + part.y,
          width: part.width,
          height: part.height,
        };
        const move = contactDistance(box, rect, axis, direction, amount);
        if (move !== undefined) consider(move, column, cellRow);
      }
    }
  }

  if (best === Infinity) return { travelled: distance, hit: false, cells: [] };
  return { travelled: direction * best, hit: true, cells };
}

/**
 * How far a box can move along an axis before it touches a rectangle, or undefined when it does not reach it
 * within `amount` or when they do not share the cross-axis extent.
 */
function contactDistance(
  box: Box,
  rect: Box,
  axis: "x" | "y",
  direction: number,
  amount: number,
): number | undefined {
  // The rectangles must overlap across the axis, by more than a touch.
  const overlapsAcross =
    axis === "x"
      ? box.y + box.height > rect.y + GEOMETRY_EPSILON && box.y < rect.y + rect.height - GEOMETRY_EPSILON
      : box.x + box.width > rect.x + GEOMETRY_EPSILON && box.x < rect.x + rect.width - GEOMETRY_EPSILON;
  if (!overlapsAcross) return undefined;

  // The gap between the box's leading face and the rectangle's near face.
  let gap: number;
  if (axis === "x") {
    gap = direction > 0 ? rect.x - (box.x + box.width) : box.x - (rect.x + rect.width);
  } else {
    gap = direction > 0 ? rect.y - (box.y + box.height) : box.y - (rect.y + rect.height);
  }
  // A negative gap means the box already overlaps the rectangle: it is not stopped by what it is inside.
  if (gap < -GEOMETRY_EPSILON) return undefined;
  const move = Math.max(0, gap);
  return move <= amount + GEOMETRY_EPSILON ? move : undefined;
}

/** Throws unless the box has finite coordinates and sizes that are not negative. */
function checkBox(box: Box): void {
  const values = [box.x, box.y, box.width, box.height];
  if (!values.every((value) => Number.isFinite(value))) {
    throw new TilesError("bad-input", "the box must have finite coordinates and sizes");
  }
  if (box.width < 0 || box.height < 0) {
    throw new TilesError("bad-input", "the box must not have a negative size");
  }
}
