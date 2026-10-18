// Board: the table of cards, played through velvet-stacks. A board plays with a catalog. Each action
// queues events, and `tick` advances the jobs and returns every event since the last call.

import { loadVelvetWasm, type VelvetWasm } from "@velvet/wasm";
import { CardsError } from "./error";
import type { BoardEvent, BoardSave, Catalog, CardView, Seed, StackView } from "./types";
import { attempt, decode } from "./wire";

/** The module's board, as its class types it. */
type NativeBoard = InstanceType<VelvetWasm["StacksBoard"]>;

/** The largest 64-bit seed: 2^64 - 1. */
const MAX_SEED = (1n << 64n) - 1n;

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
   * @returns the board.
   * @throws {CardsError} with code `bad-input` for a seed that is not valid, and the code of the
   *   rule that broke for a catalog that is not valid (see `loadCatalog`).
   */
  static async create(catalog: Catalog, seed: Seed): Promise<Board> {
    const text = seedText(seed);
    const wasm = await loadVelvetWasm();
    const native = attempt(() => new wasm.StacksBoard(JSON.stringify(catalog), text));
    return new Board(native);
  }

  /**
   * Restores a board from a save and the catalog it was played with. Stacks are matched again, so a
   * job that no longer fits the catalog cannot run.
   *
   * @param catalog - the catalog of the save.
   * @param save - the save document, as `save` returns it.
   * @returns the restored board.
   * @throws {CardsError} with code `invalid-save` for a save that is inconsistent, `unknown-card` or
   *   `unknown-recipe` for a save that names what the catalog does not have, `bad-id` for ids that a
   *   JavaScript number cannot hold, and `bad-input` for a save that is not of the expected shape.
   */
  static async load(catalog: Catalog, save: BoardSave): Promise<Board> {
    const wasm = await loadVelvetWasm();
    const native = attempt(() =>
      wasm.StacksBoard.load(JSON.stringify(catalog), JSON.stringify(save)),
    );
    return new Board(native);
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
