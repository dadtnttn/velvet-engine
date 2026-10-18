// The error type of the guion API. Code that handles a failed call branches on `code`; the
// message is for logs and developers.

/**
 * Why a guion call failed:
 * - "bad-input": an argument (a cast, a vars object) is not valid, or has the wrong shape;
 * - "unknown-scene": no scene has that id (ids are `file.label`);
 * - "not-a-choice": `choose` was called when the box on screen is not a choice;
 * - "no-such-option": the choice has no option with that index;
 * - "unavailable": the option is closed (its condition does not hold);
 * - "internal": the module failed in a way that is not the caller's mistake (a bug).
 */
export type GuionErrorCode =
  | "bad-input"
  | "unknown-scene"
  | "not-a-choice"
  | "no-such-option"
  | "unavailable"
  | "internal";

/** An error from the guion API. */
export class GuionError extends Error {
  /** Why the call failed. */
  readonly code: GuionErrorCode;

  /**
   * @param code - the failure reason.
   * @param message - a sentence for logs and developers.
   * @param options - an optional `cause`.
   */
  constructor(code: GuionErrorCode, message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "GuionError";
    this.code = code;
  }
}
