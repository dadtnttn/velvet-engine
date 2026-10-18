// Conversation: a dialogue in progress, played box by box.

import type { GuionConversation as NativeConversation } from "@velvet/wasm";
import { GuionError } from "./error";
import type { GuionVars, Shown } from "./types";
import { decode } from "./wire";

/** The largest option index that the module takes (its indexes are 32-bit). */
const MAX_OPTION_INDEX = 0xffff_ffff;

/**
 * A conversation in progress: where the dialogue is, and the story's variables. Make one with
 * `GuionLibrary.start`. Each method returns what the box shows after it, so a game can draw the
 * box from the returned value alone.
 *
 * @example
 * let shown = conversation.current();
 * while (shown.type !== "end" && shown.type !== "vn") {
 *   shown = shown.type === "choice" ? conversation.choose(pickedIndex) : conversation.next();
 * }
 */
export class Conversation {
  readonly #native: NativeConversation;

  /**
   * Wraps a conversation of the module. Use `GuionLibrary.start` to make one.
   * @param native - the module's conversation object.
   */
  constructor(native: NativeConversation) {
    this.#native = native;
  }

  /** What the box shows now. Changes nothing. */
  current(): Shown {
    return decode<Shown>(this.#native.current());
  }

  /**
   * Moves past the line on screen, and returns the box that follows. On a choice, at the end or at
   * a visual-novel hand-over it changes nothing and returns the same box.
   */
  next(): Shown {
    return decode<Shown>(this.#native.next());
  }

  /**
   * Takes option `index` of the choice on screen (options are counted from 0), and returns the box
   * after it.
   *
   * @param index - the option to take.
   * @returns the box that follows the option.
   * @throws {GuionError} with code `not-a-choice` when the box is not a choice, `no-such-option`
   *   when there is no option with that index, and `unavailable` when its condition does not hold.
   */
  choose(index: number): Shown {
    if (!Number.isInteger(index) || index < 0 || index > MAX_OPTION_INDEX) {
      throw new GuionError("no-such-option", `there is no option ${index}: options are counted from 0`);
    }
    return decode<Shown>(this.#native.choose(index));
  }

  /** The story's variables (`values` and the `seen` scenes) as they now are. */
  get vars(): GuionVars {
    return decode<GuionVars>(this.#native.vars());
  }

  /**
   * Replaces the story's variables with `vars`, and returns them as they now are. The box on screen
   * does not change.
   *
   * @param vars - the new variables. Missing `values` or `seen` are read as empty.
   */
  setVars(vars: GuionVars): GuionVars {
    return decode<GuionVars>(this.#native.setVars(JSON.stringify(vars)));
  }
}
