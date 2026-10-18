// Tests of the stack layout: the spacing of stacks, the table's limits, and the search for free places.

import { describe, expect, it } from "vitest";
import {
  CARD,
  JOB_BAR,
  MIN_STACK_STEP,
  STACK_STEP,
  TABLE,
  TABLE_ORIGIN,
  UNIT,
  cardTopOffset,
  findFreeSpot,
  overlaps,
  placeStack,
  stackBodyHeight,
  stackBox,
  stackFrameHeight,
  stackStep,
  toPixels,
  toTable,
  toUnits,
  type Box,
} from "./layout";

/** The frame height of a single card: its own height and the strip for its bar. */
const ONE_CARD_FRAME = CARD.height + JOB_BAR.gap + JOB_BAR.height;

describe("stack spacing", () => {
  it("keeps the full step for a stack of one or two cards", () => {
    expect(stackStep(1)).toBe(STACK_STEP);
    expect(stackStep(2)).toBe(STACK_STEP);
  });

  it("squeezes a tall stack, but never below the smallest step", () => {
    const tall = stackStep(30);
    expect(tall).toBeLessThan(STACK_STEP);
    expect(tall).toBeGreaterThanOrEqual(MIN_STACK_STEP);
    expect(stackStep(1000)).toBe(MIN_STACK_STEP);
  });

  it("places each card of a stack one step below the card under it", () => {
    expect(cardTopOffset(0, 3)).toBe(0);
    expect(cardTopOffset(1, 3)).toBe(STACK_STEP);
    expect(cardTopOffset(2, 3)).toBe(2 * STACK_STEP);
  });

  it("gives the height of a stack from its bottom card to its top card", () => {
    expect(stackBodyHeight(0)).toBe(0);
    expect(stackBodyHeight(1)).toBe(CARD.height);
    expect(stackBodyHeight(3)).toBe(CARD.height + 2 * STACK_STEP);
    expect(stackFrameHeight(1)).toBe(ONE_CARD_FRAME);
  });
});

describe("the table's limits", () => {
  it("keeps a stack's whole frame on the table", () => {
    const max = placeStack(1e9, 1e9, 3);
    const box = stackBox(max.x, max.y, 3);
    // Board units are binary fractions of pixels, so allow a rounding error of a millionth of a pixel.
    const slack = 1e-6;
    expect(box.left + box.width).toBeLessThanOrEqual(TABLE.width + slack);
    expect(box.top + box.height).toBeLessThanOrEqual(TABLE.height + slack);
  });

  it("keeps the top-left corner at or after the table's origin", () => {
    expect(placeStack(-50, -10, 1)).toEqual({ x: 0, y: 0 });
  });

  it("places a tall stack at the top of the table when it cannot fit lower", () => {
    expect(placeStack(0, 500, 30).y).toBe(0);
  });

  it("converts between pixels and board units", () => {
    expect(toUnits(toPixels(2.5))).toBe(2.5);
    expect(toPixels(1)).toBe(UNIT);
  });

  it("converts a logical point into table pixels", () => {
    expect(toTable({ x: TABLE_ORIGIN.left + 10, y: TABLE_ORIGIN.top + 20 })).toEqual({ x: 10, y: 20 });
  });
});

describe("overlaps", () => {
  const a: Box = { left: 0, top: 0, width: 100, height: 100 };

  it("finds boxes that share an area", () => {
    expect(overlaps(a, { left: 50, top: 50, width: 100, height: 100 })).toBe(true);
  });

  it("treats boxes that only touch as not overlapping", () => {
    expect(overlaps(a, { left: 100, top: 0, width: 100, height: 100 })).toBe(false);
  });

  it("counts a margin around the first box", () => {
    expect(overlaps(a, { left: 105, top: 0, width: 100, height: 100 }, 8)).toBe(true);
    expect(overlaps(a, { left: 115, top: 0, width: 100, height: 100 }, 8)).toBe(false);
  });
});

describe("findFreeSpot", () => {
  it("returns the wanted place when nothing is in the way", () => {
    expect(findFreeSpot({ x: 2, y: 1 }, 1, [])).toEqual({ x: 2, y: 1 });
  });

  it("moves a stack off a place that is taken, without overlapping the taken one", () => {
    const taken = stackBox(2, 1, 1);
    const spot = findFreeSpot({ x: 2, y: 1 }, 1, [taken]);
    expect(spot).not.toBeNull();
    if (spot === null) return;
    expect(overlaps(stackBox(spot.x, spot.y, 1), taken, 8)).toBe(false);
  });

  it("is deterministic: the same table gives the same spot", () => {
    const taken = [stackBox(2, 1, 1), stackBox(3, 1, 1)];
    expect(findFreeSpot({ x: 2, y: 1 }, 1, taken)).toEqual(findFreeSpot({ x: 2, y: 1 }, 1, taken));
  });

  it("gives up with null when no place on the table is free", () => {
    const everything: Box = { left: 0, top: 0, width: TABLE.width, height: TABLE.height };
    expect(findFreeSpot({ x: 0, y: 0 }, 1, [everything])).toBeNull();
  });
});
