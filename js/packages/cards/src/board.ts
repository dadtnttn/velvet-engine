// Board: the table of cards, played through velvet-stacks. A board plays with a catalog. Each action
// queues events, and `tick` advances the jobs and returns every event since the last call.

import { loadVelvetWasm, type VelvetWasm } from "@velvet/wasm";
import { CardsError } from "./error";
import { findFreeSpot } from "./spot";
import type {
  BoardEvent,
  BoardOptions,
  BoardPoint,
  BoardRect,
  BoardSave,
  BoardSize,
  Catalog,
  CardView,
  FreeSpotOptions,
  OutputPlacement,
  Seed,
  StackView,
} from "./types";
import { attempt, decode } from "./wire";

/** The module's board, as its class types it. */
type NativeBoard = InstanceType<VelvetWasm["StacksBoard"]>;

/** The largest 64-bit seed: 2^64 - 1. */
const MAX_SEED = (1n << 64n) - 1n;

/** Rounds to binary32, the precision the module keeps positions and sizes in. */
const f32 = Math.fround;

/**
 * An output placement as the module reads it: a footprint `[width, height]` and bounds `[x, y, width,
 * height]`, as arrays of numbers. The module writes it the same way (see docs/formats/STACKS.md).
 */
type WirePlacement =
  | { readonly kind: "row" }
  | {
      readonly kind: "free";
      readonly footprint: readonly [number, number];
      readonly bounds?: readonly [number, number, number, number];
    };

/**
 * The decimal text that a seed crosses the boundary as. A number must be a safe integer, because
 * a larger one has already lost the bits that its digits would name; a bigint or a string keeps
 * all 64 bits.
 * @throws {CardsError} with code `bad-input` for a number that is not a safe whole number from 0 up,
 *   or a bigint outside 0 to 2^64 - 1.
 */
function seedText(seed: Seed): string {
  if (typeof seed === "string") return seed;
  if (typeof seed === "number") {
    if (!Number.isSafeInteger(seed) || seed < 0) {
      throw new CardsError(
        "bad-input",
        `a numeric seed must be a whole number from 0 to 2^53 - 1, not ${seed}: pass a bigint or a decimal string for larger seeds`,
      );
    }
    return String(seed);
  }
  if (seed < 0n || seed > MAX_SEED) {
    throw new CardsError("bad-input", `a seed must be from 0 to 2^64 - 1, not ${seed}`);
  }
  return seed.toString();
}

/**
 * Refuses a position that the module cannot hold. The module keeps positions as binary32 numbers,
 * so a number too large for that is refused like NaN.
 * @throws {CardsError} with code `bad-input` for a position that is not two finite numbers.
 */
function checkPosition(x: number, y: number): void {
  if (!Number.isFinite(Math.fround(x)) || !Number.isFinite(Math.fround(y))) {
    throw new CardsError("bad-input", `a position must be two finite numbers, not (${x}, ${y})`);
  }
}

/**
 * Refuses a size that a stack cannot have: both sides must be finite binary32 numbers above zero.
 * @throws {CardsError} with code `bad-input`.
 */
function checkSize(size: BoardSize): void {
  const width = f32(size.width);
  const height = f32(size.height);
  if (!(Number.isFinite(width) && width > 0 && Number.isFinite(height) && height > 0)) {
    throw new CardsError("bad-input", `a size must be two finite numbers above zero, not (${size.width}, ${size.height})`);
  }
}

/**
 * Refuses a rectangle of bounds whose corner or sides are not finite binary32 numbers, whose sides are
 * negative, or whose far edges overflow.
 * @throws {CardsError} with code `bad-input`.
 */
function checkBounds(bounds: BoardRect): void {
  const fx = f32(bounds.x);
  const fy = f32(bounds.y);
  const fw = f32(bounds.width);
  const fh = f32(bounds.height);
  const finite = [fx, fy, fw, fh, f32(fx + fw), f32(fy + fh)].every((value) => Number.isFinite(value));
  if (!finite || fw < 0 || fh < 0) {
    throw new CardsError(
      "bad-input",
      `bounds must be a finite rectangle with sides of zero or more, not ${JSON.stringify(bounds)}`,
    );
  }
}

/** True when `value` is a number that binary32 holds as a finite number. */
function isBinary32(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(f32(value));
}

/**
 * Turns a placement into the document the module reads. Every number in it must be a finite binary32
 * number. The rules about sizes (a side above zero, a bounds side of zero or more) are the module's: it
 * refuses them with `bad-placement`.
 * @throws {CardsError} with code `bad-input` for a placement that is not of the expected shape, or a
 *   number in it that is not finite.
 */
function toWire(placement: OutputPlacement): WirePlacement {
  if (placement.kind === "row") return { kind: "row" };
  const { footprint, bounds } = placement;
  const numbers: unknown[] = [footprint?.width, footprint?.height];
  if (bounds !== undefined) numbers.push(bounds.x, bounds.y, bounds.width, bounds.height);
  if (placement.kind !== "free" || !numbers.every(isBinary32)) {
    throw new CardsError(
      "bad-input",
      "an output placement is a row, or a free placement whose footprint and bounds are finite numbers, " +
        `not ${JSON.stringify(placement)}`,
    );
  }
  const wireFootprint: [number, number] = [f32(footprint.width), f32(footprint.height)];
  if (bounds === undefined) return { kind: "free", footprint: wireFootprint };
  const wireBounds: [number, number, number, number] = [
    f32(bounds.x),
    f32(bounds.y),
    f32(bounds.width),
    f32(bounds.height),
  ];
  return { kind: "free", footprint: wireFootprint, bounds: wireBounds };
}

/** The placement that the module wrote, in the shape the cards API uses. */
function fromWire(wire: WirePlacement): OutputPlacement {
  if (wire.kind === "row") return { kind: "row" };
  const [width, height] = wire.footprint;
  const footprint = { width, height };
  if (wire.bounds === undefined) return { kind: "free", footprint };
  const [x, y, w, h] = wire.bounds;
  return { kind: "free", footprint, bounds: { x, y, width: w, height: h } };
}

/**
 * Applies the options of `Board.create` or `Board.load` to a new board. A bad option frees the board and
 * throws, so that nothing is left behind.
 * @throws {CardsError} with the code of the option that is not valid (`bad-placement`, for one).
 */
function withOptions(board: Board, options: BoardOptions): Board {
  if (options.outputPlacement !== undefined) {
    try {
      board.setOutputPlacement(options.outputPlacement);
    } catch (error) {
      board.free();
      throw error;
    }
  }
  return board;
}

/**
 * A board of cards. Make one with `Board.create` (a new board) or `Board.load` (the board of a
 * save), then act on it: spawn, move, drop, split and remove cards, and call `tick` as time passes.
 * Card and stack ids are numbers that count from 1. Call `free` when a board is no longer needed.
 *
 * Read the events of an action before saving: `save` does not hold queued events.
 *
 * @example
 * const board = await Board.create(catalog, 42);
 * const forest = board.spawn("forest", 2, 3);
 * const worker = board.spawn("worker", 0, 0);
 * board.dropOn(worker, forest);
 * for (const event of board.tick(1 / 60)) {
 *   if (event.event === "card_spawned") playSound("pop");
 * }
 */
export class Board {
  #native: NativeBoard | undefined;

  /** @param native - the module's board. Use `Board.create` or `Board.load` to make one. */
  private constructor(native: NativeBoard) {
    this.#native = native;
  }

  /**
   * Makes an empty board that plays with `catalog`. Equal seeds and equal actions give equal events.
   *
   * @param catalog - a catalog from `loadCatalog`.
   * @param seed - the seed of the chance rolls: a safe whole number, a bigint of up to 64 bits, or
   *   a string of decimal digits of up to 64 bits.
   * @param options - the output placement, when it is not the row (see `setOutputPlacement`).
   * @returns the board.
   * @throws {CardsError} with code `bad-input` for a seed that is not valid, the code of the
   *   rule that broke for a catalog that is not valid (see `loadCatalog`), and `bad-placement` for an
   *   output placement that cannot hold a stack.
   */
  static async create(catalog: Catalog, seed: Seed, options: BoardOptions = {}): Promise<Board> {
    const text = seedText(seed);
    const wasm = await loadVelvetWasm();
    const native = attempt(() => new wasm.StacksBoard(JSON.stringify(catalog), text));
    return withOptions(new Board(native), options);
  }

  /**
   * Restores a board from a save and the catalog it was played with. Stacks are matched again, so a
   * job that no longer fits the catalog cannot run. A save does not hold the output placement, so a
   * board starts with the row: pass `options` to set the placement the game uses.
   *
   * @param catalog - the catalog of the save.
   * @param save - the save document, as `save` returns it.
   * @param options - the output placement, when it is not the row (see `setOutputPlacement`).
   * @returns the restored board.
   * @throws {CardsError} with code `invalid-save` for a save that is inconsistent, `unknown-card` or
   *   `unknown-recipe` for a save that names what the catalog does not have, `bad-id` for ids that a
   *   JavaScript number cannot hold, `bad-input` for a save that is not of the expected shape, and
   *   `bad-placement` for an output placement that cannot hold a stack.
   */
  static async load(catalog: Catalog, save: BoardSave, options: BoardOptions = {}): Promise<Board> {
    const wasm = await loadVelvetWasm();
    const native = attempt(() =>
      wasm.StacksBoard.load(JSON.stringify(catalog), JSON.stringify(save)),
    );
    return withOptions(new Board(native), options);
  }

  /** The module's board; throws `disposed` once `free` has been called. */
  #live(): NativeBoard {
    if (this.#native === undefined) {
      throw new CardsError("disposed", "this board was freed with free() and can no longer be used");
    }
    return this.#native;
  }

  /**
   * Creates a card of definition `def` at (`x`, `y`), in a stack of its own, and returns its id.
   * Queues the card's events.
   *
   * @param def - the card's definition in the catalog.
   * @param x - the horizontal position, in board units.
   * @param y - the vertical position, in board units.
   * @returns the new card's id.
   * @throws {CardsError} with code `unknown-card` when the catalog has no card `def`, and
   *   `bad-input` when a position is not a finite number.
   */
  spawn(def: string, x: number, y: number): number {
    return decode<{ card: number }>(this.#live().spawn(def, x, y)).card;
  }

  /**
   * Moves the whole stack of `card` so that its bottom card sits at (`x`, `y`). Queues no events,
   * and never starts or cancels a job.
   *
   * @throws {CardsError} with code `no-such-card` when no card has that id, and `bad-id` or
   *   `bad-input` for an argument that is not valid.
   */
  moveStack(card: number, x: number, y: number): void {
    decode<null>(this.#live().moveStack(card, x, y));
  }

  /**
   * Drops the stack from `card` upward onto the stack that holds `onto`. The card on top of that
   * stack decides, by its stack rule. A refused drop changes nothing and queues no events.
   *
   * @throws {CardsError} with code `drop-refused` when the top card does not accept the dragged
   *   card, `stack-full` when the stack would grow past its limit, `same-stack` when both cards are
   *   on one stack, and `no-such-card` or `bad-id` for ids that do not name a card on the table.
   */
  dropOn(card: number, onto: number): void {
    decode<null>(this.#live().dropOn(card, onto));
  }

  /**
   * Takes `card` and the cards above it into a new stack, and returns the new stack's id. Splitting
   * the bottom card changes nothing and returns the stack the card is in.
   *
   * By default the new stack stays where the old one is: its bottom card keeps the old position, so
   * the new stack lies on top of the old one. Pass `at` to put it somewhere else in the same call,
   * with no extra events. When `card` is the bottom card nothing is split, and the stack that holds
   * it moves to `at` as a whole.
   *
   * @param card - the card to take, and the cards above it.
   * @param at - where the bottom card of the new stack goes, in board units. Default: the position
   *   of the old stack.
   * @returns the id of the new stack, or of the stack that holds `card` when it is the bottom card.
   * @throws {CardsError} with code `no-such-card` or `bad-id` for an id that is not valid, and
   *   `bad-input` for a position that is not two finite numbers. A refused call changes nothing.
   */
  split(card: number, at?: { readonly x: number; readonly y: number }): number {
    if (at !== undefined) checkPosition(at.x, at.y);
    const stack = decode<{ stack: number }>(this.#live().split(card)).stack;
    if (at !== undefined) this.moveStack(card, at.x, at.y);
    return stack;
  }

  /**
   * Takes `card` off the table. The cards above it stay in their stack.
   *
   * @throws {CardsError} with code `no-such-card` or `bad-id` for an id that is not valid.
   */
  remove(card: number): void {
    decode<null>(this.#live().remove(card));
  }

  /**
   * Advances time by `dt` seconds, finishes the jobs that are due, and returns the events in the
   * order they happened: those queued since the last `tick` or `takeEvents` (from actions), then
   * those of this tick. A negative or non-finite `dt` counts as 0.
   *
   * @param dt - the seconds that went by.
   * @returns the events.
   */
  tick(dt: number): BoardEvent[] {
    return decode<BoardEvent[]>(this.#live().tick(dt));
  }

  /**
   * Returns and clears the events that actions have queued since the last `tick` or `takeEvents`.
   * Call it before `save`, so that no event is lost.
   */
  takeEvents(): BoardEvent[] {
    return decode<BoardEvent[]>(this.#live().takeEvents());
  }

  /**
   * The progress, from 0 to 1, of the job on the stack that holds `card`. Null when that stack runs
   * no job, or when no card has that id.
   *
   * @throws {CardsError} with code `bad-id` for an id that is not valid.
   */
  progress(card: number): number | null {
    return decode<number | null>(this.#live().progress(card));
  }

  /**
   * The recipe that runs on the stack that holds `card`, or null when none runs.
   *
   * @throws {CardsError} with code `bad-id` for an id that is not valid.
   */
  runningRecipe(card: number): string | null {
    return decode<string | null>(this.#live().runningRecipe(card));
  }

  /**
   * Every card on the table, sorted by id. Use it to draw the table.
   */
  cards(): CardView[] {
    return decode<CardView[]>(this.#live().cards());
  }

  /**
   * Every stack on the table, sorted by id, with the cards bottom to top, the position of the bottom
   * card and the job it runs. Use it to draw the table.
   */
  stacks(): StackView[] {
    return decode<StackView[]>(this.#live().stacks());
  }

  /**
   * The nearest spot to `near` where a new stack of `size` does not overlap the stacks on the table. Each stack
   * covers `size` from its position, so two stacks overlap when their positions differ by less than `size` on
   * both axes; stacks that only touch do not overlap. The search tries the spots on a grid of `size` steps around
   * `near`, ring by ring outward, and returns the nearest one that is free (ties go to the smaller y, then x).
   * It changes nothing and queues no events.
   *
   * @param near - where the search starts, in board units.
   * @param size - the footprint of a stack, in board units. Every stack, the new one included, takes this size.
   * @param options - `bounds`, a rectangle the spot's position must lie in; and `except`, a card whose stack is
   *   not an obstacle (the stack being moved).
   * @returns the spot, or null when every spot inside `bounds` is taken.
   * @throws {CardsError} with code `bad-input` for a point, size or bounds that is not valid, and `no-such-card`
   *   when `except` names no card on the table.
   *
   * @example
   * const table = { x: 0, y: 0, width: 8, height: 6 };
   * const spot = board.freeSpot({ x: 2, y: 3 }, { width: 1, height: 1 }, { bounds: table });
   * if (spot !== null) board.moveStack(card, spot.x, spot.y);
   */
  freeSpot(near: BoardPoint, size: BoardSize, options: FreeSpotOptions = {}): BoardPoint | null {
    checkPosition(near.x, near.y);
    checkSize(size);
    if (options.bounds !== undefined) checkBounds(options.bounds);
    const stacks = this.stacks();
    const except = options.except;
    let obstacles = stacks;
    if (except !== undefined) {
      const holder = stacks.find((stack) => stack.cards.includes(except));
      if (holder === undefined) {
        throw new CardsError("no-such-card", `no card on the table has the id ${except}`);
      }
      obstacles = stacks.filter((stack) => stack.id !== holder.id);
    }
    return findFreeSpot(obstacles, near, size, options.bounds);
  }

  /**
   * Sets where the outputs of finished jobs land from now on: a row to the right of the stack (the
   * default), or the nearest free spot of a grid (`free`). It changes no card, stack or job, and queues no
   * events. The placement is not part of a save: a game sets it again after `Board.load`, or passes it to
   * `Board.load` as an option.
   *
   * @param placement - the placement. For `free`, `footprint` is the size of a stack, and `bounds` (when
   *   given) the rectangle that a spot must lie in, edges included.
   * @throws {CardsError} with code `bad-input` for a number that is not finite (or not a placement of the
   *   expected shape), and `bad-placement` for a footprint side that is not above zero, or a bounds side
   *   that is negative. A refused placement changes nothing.
   *
   * @example
   * board.setOutputPlacement({ kind: "free", footprint: { width: 1, height: 1 }, bounds: table });
   */
  setOutputPlacement(placement: OutputPlacement): void {
    decode<null>(this.#live().setOutputPlacement(JSON.stringify(toWire(placement))));
  }

  /**
   * The placement that finished jobs use now: `{ kind: "row" }` unless a game set another.
   *
   * @throws {CardsError} with code `disposed` once `free` has been called.
   */
  outputPlacement(): OutputPlacement {
    return fromWire(decode<WirePlacement>(this.#live().outputPlacement()));
  }

  /**
   * The board's state as a save document. Queued events are not in it.
   *
   * @throws {CardsError} with code `invalid-save` or `bad-id` when the board cannot be written
   *   exactly (a position or time overflowed binary32, or an id is too large for a number).
   */
  save(): BoardSave {
    return decode<BoardSave>(this.#live().save());
  }

  /**
   * Frees the memory of the board in the module. Calling it again does nothing. Any other call
   * afterwards throws `disposed`.
   */
  free(): void {
    this.#native?.free();
    this.#native = undefined;
  }
}
