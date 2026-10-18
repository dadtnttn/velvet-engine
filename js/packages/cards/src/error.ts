// The error type of the cards API. Code that handles a failed call branches on `code`; the message
// is for logs and developers.

/**
 * Why a cards call failed:
 * - "bad-input": an argument is not valid: a catalog or a save that is not JSON of the expected
 *   shape, a seed, or a position that is not a finite number;
 * - "bad-id": a card or stack id is not a whole number from 1 up to 2^53 - 1;
 * - "unsupported-document": a document's format, kind or version is not one this engine reads;
 * - "empty-card-id", "duplicate-card", "zero-max", "unknown-card", "duplicate-recipe",
 *   "unknown-recipe", "no-ingredients", "zero-count", "bad-seconds" and "bad-chance": the catalog or
 *   a save names something that is wrong, and the code names the rule that broke;
 * - "no-such-card": no card on the table has that id;
 * - "same-stack": a stack was dropped onto itself;
 * - "drop-refused": the card on top of the target does not accept the dragged card;
 * - "stack-full": the drop would make the stack taller than the target's rule allows;
 * - "invalid-save": a save is internally inconsistent, or the board cannot be written exactly;
 * - "disposed": the board was freed with `free()` and can no longer be used;
 * - "internal": the module failed in a way that is not the caller's mistake (a bug).
 */
export type CardsErrorCode =
  | "bad-input"
  | "bad-id"
  | "unsupported-document"
  | "empty-card-id"
  | "duplicate-card"
  | "zero-max"
  | "unknown-card"
  | "duplicate-recipe"
  | "unknown-recipe"
  | "no-ingredients"
  | "zero-count"
  | "bad-seconds"
  | "bad-chance"
  | "no-such-card"
  | "same-stack"
  | "drop-refused"
  | "stack-full"
  | "invalid-save"
  | "disposed"
  | "internal";

/** An error from the cards API. */
export class CardsError extends Error {
  /** Why the call failed. */
  readonly code: CardsErrorCode;

  /**
   * @param code - the failure reason.
   * @param message - a sentence for logs and developers.
   * @param options - an optional `cause`.
   */
  constructor(code: CardsErrorCode, message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "CardsError";
    this.code = code;
  }
}
