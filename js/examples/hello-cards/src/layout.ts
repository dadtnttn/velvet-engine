// Stack layout: where cards and stacks sit on the table.
//
// The engine places stacks in board units. One unit is UNIT logical pixels, and a card is about one
// unit wide, so a copy that the engine puts 1.5 units away lands beside its source. The view draws
// cards in table pixels (the table's own top-left corner is the origin). Every function here is
// pure, so the maths is tested without a browser.

/** Logical pixels in one board unit. */
export const UNIT = 160;

/** The table where cards lie, in its own pixels. */
export const TABLE = { width: 1840, height: 880 } as const;

/** Where the table's top-left corner sits in the logical surface. */
export const TABLE_ORIGIN = { left: 40, top: 120 } as const;

/** A card, in logical pixels. */
export const CARD = { width: 112, height: 150 } as const;

/** The vertical offset between the cards of a stack, in pixels: each card's name stays visible. */
export const STACK_STEP = 34;

/** The smallest offset that a very tall stack shrinks to. */
export const MIN_STACK_STEP = 16;

/** The strip under a stack that holds its progress bar: a gap, then the bar. */
export const JOB_BAR = { gap: 8, height: 12 } as const;

/** Room kept under the cards of a stack for its progress bar. */
const BAR_ROOM = JOB_BAR.gap + JOB_BAR.height;

/** A point. Its unit depends on the function that gives it: pixels or board units. */
export interface Point {
  readonly x: number;
  readonly y: number;
}

/** A box in table pixels. */
export interface Box {
  readonly left: number;
  readonly top: number;
  readonly width: number;
  readonly height: number;
}

/** Keeps `value` between `low` and `high`. `high` must not be below `low`. */
function clamp(value: number, low: number, high: number): number {
  return Math.min(Math.max(value, low), high);
}

/** Logical pixels to board units. */
export function toUnits(pixels: number): number {
  return pixels / UNIT;
}

/** Board units to logical pixels. */
export function toPixels(units: number): number {
  return units * UNIT;
}

/** A point of the logical surface, converted into table pixels. */
export function toTable(point: Point): Point {
  return { x: point.x - TABLE_ORIGIN.left, y: point.y - TABLE_ORIGIN.top };
}

/**
 * The vertical offset between two cards of a stack of `count` cards. It is STACK_STEP, but a tall
 * stack squeezes its cards together so that the stack still fits on the table.
 */
export function stackStep(count: number): number {
  if (count <= 1) return STACK_STEP;
  const room = (TABLE.height - BAR_ROOM - CARD.height) / (count - 1);
  return clamp(room, MIN_STACK_STEP, STACK_STEP);
}

/** The height from the top of a stack's bottom card to the bottom of its top card. */
export function stackBodyHeight(count: number): number {
  if (count <= 0) return 0;
  return CARD.height + (count - 1) * stackStep(count);
}

/** The height of a stack's frame: its cards, and the room for its progress bar. */
export function stackFrameHeight(count: number): number {
  return stackBodyHeight(count) + BAR_ROOM;
}

/** The top offset of card `index` inside its stack. Index 0 is the bottom card. */
export function cardTopOffset(index: number, count: number): number {
  return index * stackStep(count);
}

/**
 * Where the bottom card of a stack of `count` cards should go, for a wanted top-left corner in
 * table pixels. The whole frame of the stack is kept on the table. The result is in board units.
 */
export function placeStack(left: number, top: number, count: number): Point {
  const maxLeft = TABLE.width - CARD.width;
  const maxTop = Math.max(0, TABLE.height - stackFrameHeight(count));
  return {
    x: toUnits(clamp(left, 0, maxLeft)),
    y: toUnits(clamp(top, 0, maxTop)),
  };
}

/** The frame of a stack whose bottom card is at board position (`x`, `y`), in table pixels. */
export function stackBox(x: number, y: number, count: number): Box {
  return {
    left: toPixels(x),
    top: toPixels(y),
    width: CARD.width,
    height: stackFrameHeight(count),
  };
}

/** True when two boxes overlap, after `margin` pixels are added around `a`. */
export function overlaps(a: Box, b: Box, margin = 0): boolean {
  return (
    a.left - margin < b.left + b.width &&
    b.left < a.left + a.width + margin &&
    a.top - margin < b.top + b.height &&
    b.top < a.top + a.height + margin
  );
}

/** The grid that free spots are searched on, in board units. */
const SEARCH_GRID = 0.25;

/**
 * The free position nearest to `wanted` (board units) for a new stack of `count` cards, whose
 * frame does not overlap any of `others` (table pixels, with `margin` pixels of air). The search is
 * over a fixed grid, and ties are broken by position, so the same table always gives the same
 * answer. Returns null when the table has no free place.
 */
export function findFreeSpot(
  wanted: Point,
  count: number,
  others: readonly Box[],
  margin = 8,
): Point | null {
  const start = placeStack(toPixels(wanted.x), toPixels(wanted.y), count);
  const columns = Math.floor(toUnits(TABLE.width - CARD.width) / SEARCH_GRID);
  const rows = Math.floor(toUnits(Math.max(0, TABLE.height - stackFrameHeight(count))) / SEARCH_GRID);
  const candidates: Point[] = [];
  for (let column = 0; column <= columns; column += 1) {
    for (let row = 0; row <= rows; row += 1) {
      candidates.push({ x: column * SEARCH_GRID, y: row * SEARCH_GRID });
    }
  }
  const distance = (point: Point): number => (point.x - start.x) ** 2 + (point.y - start.y) ** 2;
  candidates.sort((a, b) => distance(a) - distance(b) || a.y - b.y || a.x - b.x);
  for (const candidate of candidates) {
    const box = stackBox(candidate.x, candidate.y, count);
    if (others.every((other) => !overlaps(box, other, margin))) return candidate;
  }
  return null;
}
