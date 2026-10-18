// Tests of the drop decision: which card a release lands on, and which releases count as clicks.

import { describe, expect, it } from "vitest";
import { decideDrop, isClick, type DropTarget } from "./drop";

/** A target card at a rectangle, in logical pixels. */
function target(card: number, left: number, top: number, width = 112, height = 150): DropTarget {
  return { card, rect: { left, top, width, height } };
}

describe("decideDrop", () => {
  it("places the stack when no card is under the pointer", () => {
    expect(decideDrop({ x: 500, y: 500 }, [target(1, 0, 0)])).toEqual({ kind: "place" });
    expect(decideDrop({ x: 500, y: 500 }, [])).toEqual({ kind: "place" });
  });

  it("drops the stack onto the card under the pointer", () => {
    expect(decideDrop({ x: 20, y: 20 }, [target(7, 0, 0)])).toEqual({ kind: "drop", onto: 7 });
  });

  it("chooses the card drawn on top where two cards overlap", () => {
    const bottom = target(1, 0, 0);
    const top = target(2, 10, 10);
    expect(decideDrop({ x: 50, y: 50 }, [bottom, top])).toEqual({ kind: "drop", onto: 2 });
  });

  it("counts the left and top edges as inside, and the right and bottom edges as outside", () => {
    expect(decideDrop({ x: 0, y: 0 }, [target(1, 0, 0)])).toEqual({ kind: "drop", onto: 1 });
    expect(decideDrop({ x: 112, y: 10 }, [target(1, 0, 0)])).toEqual({ kind: "place" });
    expect(decideDrop({ x: 10, y: 150 }, [target(1, 0, 0)])).toEqual({ kind: "place" });
  });

  it("falls through to a card below when the top card is not under the pointer", () => {
    const bottom = target(1, 0, 0);
    const top = target(2, 200, 200);
    expect(decideDrop({ x: 20, y: 20 }, [bottom, top])).toEqual({ kind: "drop", onto: 1 });
  });
});

describe("isClick", () => {
  it("counts a release near the press as a click", () => {
    expect(isClick({ x: 100, y: 100 }, { x: 102, y: 103 })).toBe(true);
  });

  it("counts a release far from the press as a drag", () => {
    expect(isClick({ x: 100, y: 100 }, { x: 140, y: 100 })).toBe(false);
  });

  it("uses the slop given, so that a stricter test is possible", () => {
    expect(isClick({ x: 0, y: 0 }, { x: 3, y: 0 }, 2)).toBe(false);
  });
});
