// Tests of the free-spot search (spot.ts), without WebAssembly: the nearest free grid spot, the rule for
// touching stacks, the tie rule, the bounds, the binary32 rounding, and a brute-force check of the ring
// search on random tables.

import { describe, expect, it } from "vitest";
import { findFreeSpot } from "./spot";
import { generator } from "./test-support";
import type { BoardPoint, BoardSize } from "./types";

const UNIT: BoardSize = { width: 1, height: 1 };

/** Rounds to binary32, as the rules do. */
const f32 = Math.fround;

/**
 * Whether two stacks of `size` at these positions overlap: they differ by less than the size on both axes. The
 * arithmetic is binary32, as in findFreeSpot, so that touching stacks stay apart after rounding.
 */
function overlapping(a: BoardPoint, b: BoardPoint, size: BoardSize): boolean {
  const tolerance = f32(f32(1e-5) * Math.max(size.width, size.height));
  return (
    Math.abs(f32(a.x - b.x)) < f32(size.width - tolerance) &&
    Math.abs(f32(a.y - b.y)) < f32(size.height - tolerance)
  );
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

  it("looks past the first rings when no bounds are given, and finds nothing inside bounds that are taken", () => {
    // The bush and eight stones fill rings 0 and 1 around (1, 0): the nearest free spots are two steps out, and
    // (1, -2) is the one with the smallest y.
    const table: BoardPoint[] = [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 1, y: -1 },
      { x: 2, y: -1 },
      { x: 0, y: -1 },
      { x: 2, y: 0 },
      { x: 0, y: 1 },
      { x: 1, y: 1 },
      { x: 2, y: 1 },
    ];
    expect(findFreeSpot(table, { x: 1, y: 0 }, UNIT)).toEqual({ x: 1, y: -2 });
    expect(findFreeSpot(table, { x: 1, y: 0 }, UNIT, { x: 0, y: 0, width: 0, height: 0 })).toBeNull();
  });

  it("ends for a footprint so small that its squares underflow, at the spot velvet-stacks picks", () => {
    // The squared distances of the rings round to zero here, so the stopping rule never fires: the cap ends it.
    // The same spot as the Rust test in placement.rs, binary32 value for binary32 value.
    expect(findFreeSpot([{ x: 1000, y: 0 }], { x: 1e-30, y: 0 }, { width: 1e-30, height: 1e-30 })).toEqual({
      x: f32(-9.99e-28),
      y: f32(-1e-27),
    });
  });

  it("rounds its inputs and its results to binary32, as the module does", () => {
    // 0.1 is not a binary32 number: the spot is the binary32 value nearest to it.
    expect(findFreeSpot([], { x: 0.1, y: 0 }, UNIT)).toEqual({ x: f32(0.1), y: 0 });
    // A stack of width 0.3 steps to f32(0.3) from the origin; the left and right spots tie, and the left
    // one has the smaller offset in x.
    expect(findFreeSpot([{ x: 0, y: 0 }], { x: 0, y: 0 }, { width: 0.3, height: 1 })).toEqual({
      x: -f32(0.3),
      y: 0,
    });
  });

  it("finds the same spot as a scan of every grid spot, on random tables", () => {
    const random = generator(7);
    for (let table = 0; table < 40; table += 1) {
      const size: BoardSize = { width: f32(1 + random()), height: f32(0.5 + random()) };
      const near: BoardPoint = { x: f32(random() * 4 - 2), y: f32(random() * 4 - 2) };
      // Some stacks sit on the grid (so that they touch the spots next to them), the others anywhere.
      const obstacles: BoardPoint[] = Array.from({ length: 1 + Math.floor(random() * 25) }, () =>
        random() < 0.5
          ? {
              x: f32(near.x + f32((Math.floor(random() * 11) - 5) * size.width)),
              y: f32(near.y + f32((Math.floor(random() * 11) - 5) * size.height)),
            }
          : { x: f32(random() * 12 - 6), y: f32(random() * 12 - 6) },
      );

      // The scan: every spot in a window, the free ones only, by the same rule as the search: the squared
      // distance from `near`, then the offset in y, then the offset in x. Every step is rounded to binary32.
      let expected: BoardPoint | null = null;
      let bestKey: [number, number, number] | null = null;
      for (let j = -40; j <= 40; j += 1) {
        for (let i = -40; i <= 40; i += 1) {
          const dx = f32(i * size.width);
          const dy = f32(j * size.height);
          const spot = { x: f32(near.x + dx), y: f32(near.y + dy) };
          if (obstacles.some((stack) => overlapping(stack, spot, size))) continue;
          const key: [number, number, number] = [f32(f32(dx * dx) + f32(dy * dy)), j, i];
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
