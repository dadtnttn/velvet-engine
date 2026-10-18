// The error type of the save store. Code that handles a failed save branches on `code`;
// the message is for logs and developers.

/**
 * Why a save operation failed:
 * - "bad-slot": the slot name does not match SLOT_PATTERN;
 * - "bad-data": the data is undefined, a function or a symbol, or cannot be written as JSON;
 * - "corrupt": the stored text is not a save envelope;
 * - "wrong-game": the save belongs to another game;
 * - "newer-version": the save was written by a newer build than this one;
 * - "missing-migration": no migration exists to bring the save up to date.
 */
export type SaveErrorCode =
  | "bad-slot"
  | "bad-data"
  | "corrupt"
  | "wrong-game"
  | "newer-version"
  | "missing-migration";

/** An error raised by the save store. */
export class SaveError extends Error {
  /** Why the operation failed. */
  readonly code: SaveErrorCode;

  /**
   * @param code - the failure reason.
   * @param message - a sentence for logs and developers.
   * @param options - an optional `cause`, such as the JSON parse error.
   */
  constructor(code: SaveErrorCode, message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "SaveError";
    this.code = code;
  }
}
