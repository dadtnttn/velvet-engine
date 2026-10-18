// The free-spot search: the nearest spot, on a grid around a point, where a new stack does not overlap the
// stacks already on the table. It serves Board.freeSpot, and it is the JavaScript form of the search that
// velvet-stacks runs for the free output placement (crates/velvet-stacks/src/placement.rs). It is plain
// arithmetic over positions, so it needs no WebAssembly and tests on its own. The rules compute in binary32,
// so every result is rounded with Math.fround, operation for operation, and both runtimes pick the same spot
// for the same table.

import type { BoardPoint, BoardRect, BoardSize } from "./types";

/** A grid cell as offsets (i, j) from the point the search starts at: the spot is `near + (i * width, j * height)`. */
type Cell = readonly [number, number];

/** Rounds a number to binary32, the precision that the rules compute in. */
const f32 = Math.fround;

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
 * The arithmetic is binary32, as in the module: the inputs are rounded to binary32, and so is every result.
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
  const width = f32(size.width);
  const height = f32(size.height);
  const start = { x: f32(near.x), y: f32(near.y) };
  const stacks = obstacles.map((stack) => ({ x: f32(stack.x), y: f32(stack.y) }));
  const area =
    bounds === undefined
      ? undefined
      : {
          x: f32(bounds.x),
          y: f32(bounds.y),
          right: f32(bounds.x + bounds.width),
          bottom: f32(bounds.y + bounds.height),
        };

  // Stacks that only touch must not overlap. The tolerance keeps them apart after rounding.
  const tolerance = f32(f32(1e-5) * Math.max(width, height));
  const clearX = f32(width - tolerance);
  const clearY = f32(height - tolerance);
  const overlaps = (spot: BoardPoint): boolean =>
    stacks.some(
      (stack) => Math.abs(f32(stack.x - spot.x)) < clearX && Math.abs(f32(stack.y - spot.y)) < clearY,
    );
  const inside = (spot: BoardPoint): boolean =>
    area === undefined ||
    (spot.x >= area.x && spot.x <= area.right && spot.y >= area.y && spot.y <= area.bottom);

  // How many steps from `start` to (x, y), counting the larger axis: a spot's ring is its number of steps.
  const steps = (x: number, y: number): number =>
    Math.max(f32(Math.abs(f32(x - start.x)) / width), f32(Math.abs(f32(y - start.y)) / height));

  // The rings to search, from firstRing to lastRing, as the module searches them.
  let firstRing = 0;
  let lastRing: number;
  if (area === undefined) {
    // An obstacle blocks only the cells within one step of it, so the rings past the farthest obstacle are free.
    let farthest = 0;
    for (const stack of stacks) farthest = Math.max(farthest, steps(stack.x, stack.y));
    lastRing = Math.ceil(farthest) + 2;
  } else {
    // Cells inside the bounds lie between the nearest point of the bounds and its far corner.
    const nearestX = Math.min(Math.max(start.x, area.x), area.right);
    const nearestY = Math.min(Math.max(start.y, area.y), area.bottom);
    const gap = steps(nearestX, nearestY);
    firstRing = Math.max(0, Math.floor(gap) - 1);
    const corners = [
      { x: area.x, y: area.y },
      { x: area.right, y: area.y },
      { x: area.x, y: area.bottom },
      { x: area.right, y: area.bottom },
    ];
    let farthest = 0;
    for (const corner of corners) farthest = Math.max(farthest, steps(corner.x, corner.y));
    lastRing = Math.ceil(farthest) + 1;
  }

  // The nearest spot so far, by squared distance; ties by the offset in y, then in x.
  let best: { spot: BoardPoint; distance: number; j: number; i: number } | undefined;
  const smallestStep = Math.min(width, height);
  for (let r = firstRing; r <= lastRing; r += 1) {
    // Every cell of this ring and of the rings after it is at least this far away, so the search can stop.
    const reach = f32(r * smallestStep);
    if (best !== undefined && f32(reach * reach) > best.distance) break;
    for (const [i, j] of ringCells(r)) {
      const dx = f32(i * width);
      const dy = f32(j * height);
      const distance = f32(f32(dx * dx) + f32(dy * dy));
      if (best !== undefined) {
        // Skip a cell that cannot win without checking it against the stacks.
        const tie = distance === best.distance && (j < best.j || (j === best.j && i < best.i));
        if (!(distance < best.distance || tie)) continue;
      }
      const spot = { x: f32(start.x + dx), y: f32(start.y + dy) };
      if (!inside(spot) || overlaps(spot)) continue;
      best = { spot, distance, j, i };
    }
  }
  return best === undefined ? null : best.spot;
}
