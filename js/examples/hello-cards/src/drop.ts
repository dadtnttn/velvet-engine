// The drop decision: what a released stack does. The game asks this, and then makes the call that
// the answer names: `dropOn` the card under the pointer, or leave the stack where it was put.
//
// Points and rectangles are in logical pixels, the same space as Stage.toLogical and Stage.rectOf.

import type { Point } from "./layout";

/** A rectangle in logical pixels. */
export interface LogicalRect {
  readonly left: number;
  readonly top: number;
  readonly width: number;
  readonly height: number;
}

/** A card that a drop may land on: its id, and where it is drawn. */
export interface DropTarget {
  readonly card: number;
  readonly rect: LogicalRect;
}

/**
 * What to do with a released stack. `drop` means `board.dropOn(dragged, onto)`, and `place` means
 * the stack stays where the pointer left it, on the table.
 */
export type DropDecision = { readonly kind: "drop"; readonly onto: number } | { readonly kind: "place" };

/** True when `point` lies inside `rect`. The left and top edges are inside, the right and bottom are not. */
export function contains(rect: LogicalRect, point: Point): boolean {
  return (
    point.x >= rect.left &&
    point.x < rect.left + rect.width &&
    point.y >= rect.top &&
    point.y < rect.top + rect.height
  );
}

/**
 * Decides a release. `targets` lists the cards that a drop may land on, from the bottom of the
 * stacking order to the top, so that the card drawn on top wins where two overlap. A stack that is
 * released over no target is placed.
 *
 * @param point - where the pointer was released, in logical pixels.
 * @param targets - the candidate cards, bottom first. The dragged stack's own cards are not among them.
 */
export function decideDrop(point: Point, targets: readonly DropTarget[]): DropDecision {
  for (let index = targets.length - 1; index >= 0; index -= 1) {
    const target = targets[index];
    if (target !== undefined && contains(target.rect, point)) {
      return { kind: "drop", onto: target.card };
    }
  }
  return { kind: "place" };
}

/**
 * True when a press and its release are one click: the pointer moved less than `slop` logical
 * pixels. A click on a stacked card must not split the stack, so the game takes it back.
 */
export function isClick(from: Point, to: Point, slop = 6): boolean {
  return Math.hypot(to.x - from.x, to.y - from.y) < slop;
}
