// The free-spot search behind Board.freeSpot: the nearest spot, on a grid around a point, where a new
// stack does not overlap the stacks already on the table. It is plain arithmetic over positions, so it
// needs no WebAssembly and tests on its own.

import type { BoardPoint, BoardRect, BoardSize } from "./types";

/** A grid cell as offsets (i, j) from the point the search starts at: the spot is `near + (i * width, j * height)`. */
type Cell = readonly [number, number];

/**
 * The cells of ring `r`: every cell whose larger offset, in absolute value, is `r`. Ring 0 is the
 * starting point itself. The rings are visited outward, which makes the search a spiral.
 * @param r - the ring, from 0 up.
 * @returns the cells of the ring (8 r of them, or 1 for ring 0), in no particular order.
 */
function ringCells(r: number): Cell[] {
  if (r === 0) return [[0, 0]];
  const cells: Cell[] = [];
  for (let i = -r; i <= r; i += 1) {
    cells.push([i, -r], [i, r]);
  }
  for (let j = -r + 1; j <= r - 1; j += 1) {
    cells.push([-r, j], [r, j]);
  }
  return cells;
}

/**
 * The nearest spot to `near` where a stack of `size` lies free. Every stack, the new one included, covers
 * the same `size`, starting at its position. Two stacks overlap when they share an area: their positions
 * differ by less than the size on both axes. Stacks that only touch do not overlap.
 *
 * The search tries the spots on a grid of `size` steps around `near`, ring by ring outward, and returns the
 * tried spot nearest to `near`. Ties go to the spot with the smaller offset in y, then in x, so the same
 * table always gives the same answer. It is the nearest grid spot, not the nearest position in the plane.
 *
 * @param obstacles - where the stacks on the table are (their bottom cards).
 * @param near - the point the search starts from.
 * @param size - the footprint of a stack, in board units; both sides above zero.
 * @param bounds - when given, the spot's position must lie inside this rectangle (its edges included).
 * @returns the spot, or null when every spot inside `bounds` is taken.
 */
export function findFreeSpot(
  obstacles: readonly BoardPoint[],
  near: BoardPoint,
  size: BoardSize,
  bounds?: BoardRect,
): BoardPoint | null {
  const { width, height } = size;
  // Positions are binary32 numbers on the Rust side, so two stacks that touch can differ by a little
  // rounding. The tolerance keeps touching stacks from counting as overlapping.
  const tolerance = 1e-5 * Math.max(width, height);
  const overlaps = (spot: BoardPoint): boolean =>
    obstacles.some(
      (stack) =>
        Math.abs(stack.x - spot.x) < width - tolerance && Math.abs(stack.y - spot.y) < height - tolerance,
    );
  const inside = (spot: BoardPoint): boolean =>
    bounds === undefined ||
    (spot.x >= bounds.x &&
      spot.x <= bounds.x + bounds.width &&
      spot.y >= bounds.y &&
      spot.y <= bounds.y + bounds.height);

  // The rings to search, from firstRing to lastRing. A point's ring is how many steps away it is, counting
  // the larger axis; a cell in ring r is at least r * min(width, height) away from `near`.
  const cellDistance = (x: number, y: number): number =>
    Math.max(Math.abs(x - near.x) / width, Math.abs(y - near.y) / height);
  let firstRing = 0;
  let lastRing: number;
  if (bounds === undefined) {
    // An obstacle blocks only the cells within one step of it, so the rings past the farthest obstacle are free.
    let farthest = 0;
    for (const stack of obstacles) farthest = Math.max(farthest, cellDistance(stack.x, stack.y));
    lastRing = Math.ceil(farthest) + 2;
  } else {
    // Cells inside the bounds lie between the nearest point of the bounds and its far corner.
    const nearestX = Math.min(Math.max(near.x, bounds.x), bounds.x + bounds.width);
    const nearestY = Math.min(Math.max(near.y, bounds.y), bounds.y + bounds.height);
    const gap = Math.max(Math.abs(nearestX - near.x) / width, Math.abs(nearestY - near.y) / height);
    firstRing = Math.max(0, Math.floor(gap) - 1);
    const corners: BoardPoint[] = [
      { x: bounds.x, y: bounds.y },
      { x: bounds.x + bounds.width, y: bounds.y },
      { x: bounds.x, y: bounds.y + bounds.height },
      { x: bounds.x + bounds.width, y: bounds.y + bounds.height },
    ];
    let farthest = 0;
    for (const corner of corners) farthest = Math.max(farthest, cellDistance(corner.x, corner.y));
    lastRing = Math.ceil(farthest) + 1;
  }

  // The nearest spot so far, by squared distance; ties by the offset in y, then in x.
  let best: { spot: BoardPoint; distance: number; j: number; i: number } | undefined;
  const smallestStep = Math.min(width, height);
  for (let r = firstRing; r <= lastRing; r += 1) {
    // Every cell of this ring and of the rings after it is at least this far away, so the search can stop.
    const reach = r * smallestStep;
    if (best !== undefined && reach * reach > best.distance) break;
    for (const [i, j] of ringCells(r)) {
      const dx = i * width;
      const dy = j * height;
      const distance = dx * dx + dy * dy;
      if (best !== undefined) {
        // Skip a cell that cannot win without checking it against the stacks.
        const tie = distance === best.distance && (j < best.j || (j === best.j && i < best.i));
        if (!(distance < best.distance || tie)) continue;
      }
      const spot = { x: near.x + dx, y: near.y + dy };
      if (!inside(spot) || overlaps(spot)) continue;
      best = { spot, distance, j, i };
    }
  }
  return best === undefined ? null : best.spot;
}
