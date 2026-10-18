// Tests of the free-spot search (spot.ts), without WebAssembly: the nearest free grid spot, the rule for
// touching stacks, the tie rule, the bounds, and a brute-force check of the ring search on random tables.

import { describe, expect, it } from "vitest";
import { findFreeSpot } from "./spot";
import type { BoardPoint, BoardSize } from "./types";

const UNIT: BoardSize = { width: 1, height: 1 };

/**
 * Whether two stacks of `size` at these positions overlap: they differ by less than the size on both axes. The
 * tolerance is the one that findFreeSpot uses, so that touching stacks stay apart after rounding.
 */
function overlapping(a: BoardPoint, b: BoardPoint, size: BoardSize): boolean {
  const tolerance = 1e-5 * Math.max(size.width, size.height);
  return Math.abs(a.x - b.x) < size.width - tolerance && Math.abs(a.y - b.y) < size.height - tolerance;
}

/** A small deterministic generator, so that the random tables are the same on every run. */
function generator(seed: number): () => number {
  let state = seed;
  return () => {
    state = (state * 1103515245 + 12345) % 2147483648;
    return state / 2147483648;
  };
}

describe("findFreeSpot", () => {
  it("returns the starting point when nothing is in the way", () => {
    expect(findFreeSpot([], { x: 2, y: 3 }, UNIT)).toEqual({ x: 2, y: 3 });
  });

  it("does not count a stack that only touches the new one as an overlap", () => {
    expect(findFreeSpot([{ x: 1, y: 0 }], { x: 0, y: 0 }, UNIT)).toEqual({ x: 0, y: 0 });
    expect(findFreeSpot([{ x: 0, y: 1 }], { x: 0, y: 0 }, UNIT)).toEqual({ x: 0, y: 0 });
  });

  it("moves to the nearest free spot, and breaks a tie by the smaller y, then the smaller x", () => {
    // Four spots are one step away and all are free: (0, -1) comes first, as its y is the smallest.
    expect(findFreeSpot([{ x: 0, y: 0 }], { x: 0, y: 0 }, UNIT)).toEqual({ x: 0, y: -1 });
  });

  it("finds a spot that a stack covers only partly", () => {
    // (0, 0) is covered by the stack at (0.5, 0); (0, -1) only touches it.
    expect(findFreeSpot([{ x: 0.5, y: 0 }], { x: 0, y: 0 }, UNIT)).toEqual({ x: 0, y: -1 });
  });

  it("uses the two sides of the size separately", () => {
    // The steps are 2 across and 1 down: (2, 0) is free too, but (0, -1) is nearer.
    expect(findFreeSpot([{ x: 0, y: 0 }], { x: 0, y: 0 }, { width: 2, height: 1 })).toEqual({ x: 0, y: -1 });
  });

  it("keeps the spot inside the bounds, edges included, and searches from the nearest point of them", () => {
    const bounds = { x: 0, y: 0, width: 10, height: 10 };
    expect(findFreeSpot([], { x: -5, y: -5 }, UNIT, bounds)).toEqual({ x: 0, y: 0 });
    expect(findFreeSpot([], { x: 10, y: 10 }, UNIT, bounds)).toEqual({ x: 10, y: 10 });
    expect(findFreeSpot([{ x: 10, y: 10 }], { x: 10, y: 10 }, UNIT, bounds)).toEqual({ x: 10, y: 9 });
  });

  it("returns null when every spot inside the bounds is taken, and finds the one spot of empty bounds", () => {
    const bounds = { x: 0, y: 0, width: 0, height: 0 };
    expect(findFreeSpot([{ x: 0, y: 0 }], { x: 0, y: 0 }, UNIT, bounds)).toBeNull();
    // The only spot inside the bounds is (0, 0), five steps from the starting point.
    expect(findFreeSpot([], { x: 5, y: 5 }, UNIT, bounds)).toEqual({ x: 0, y: 0 });
  });

  it("finds the same spot as a scan of every grid spot, on random tables", () => {
    const random = generator(7);
    for (let table = 0; table < 40; table += 1) {
      const size: BoardSize = { width: 1 + random(), height: 0.5 + random() };
      const near: BoardPoint = { x: random() * 4 - 2, y: random() * 4 - 2 };
      // Some stacks sit on the grid (so that they touch the spots next to them), the others anywhere.
      const obstacles: BoardPoint[] = Array.from({ length: 1 + Math.floor(random() * 25) }, () =>
        random() < 0.5
          ? {
              x: near.x + (Math.floor(random() * 11) - 5) * size.width,
              y: near.y + (Math.floor(random() * 11) - 5) * size.height,
            }
          : { x: random() * 12 - 6, y: random() * 12 - 6 },
      );

      // The scan: every spot in a window, the free ones only, by the same rule as the search: the squared
      // distance from `near`, then the offset in y, then the offset in x.
      let expected: BoardPoint | null = null;
      let bestKey: [number, number, number] | null = null;
      for (let j = -40; j <= 40; j += 1) {
        for (let i = -40; i <= 40; i += 1) {
          const dx = i * size.width;
          const dy = j * size.height;
          const spot = { x: near.x + dx, y: near.y + dy };
          if (obstacles.some((stack) => overlapping(stack, spot, size))) continue;
          const key: [number, number, number] = [dx * dx + dy * dy, j, i];
          const better =
            bestKey === null ||
            key[0] < bestKey[0] ||
            (key[0] === bestKey[0] && (key[1] < bestKey[1] || (key[1] === bestKey[1] && key[2] < bestKey[2])));
          if (better) {
            bestKey = key;
            expected = spot;
          }
        }
      }
      expect(findFreeSpot(obstacles, near, size), `table ${table}`).toEqual(expected);
    }
  });
});
