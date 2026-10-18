// The error type of the characters API. Code that handles a failed call branches on `code`; the
// message is for logs and developers.

/**
 * Why a characters call failed:
 * - "json": the text is not JSON;
 * - "unsupported-document": the format, version or kind is not one this reader reads;
 * - "invalid": a field is missing or has the wrong type or value, or a rule that has no code of its
 *   own broke (the message names the field);
 * - "bad-id": an id or an art id does not match its pattern, or a stat uses a reserved name;
 * - "duplicate-id": two entries of one list have the same id;
 * - "expression": a formula is not a valid expression (the message gives the position);
 * - "unknown-function": a formula calls a function that the language does not have;
 * - "arity": a function is called with the wrong number of arguments;
 * - "unknown-stat": an id, or a variable of a formula, names no stat;
 * - "unknown-need", "unknown-profession", "unknown-trait", "unknown-level": an id names no entry of
 *   the schema;
 * - "unknown-character": a roster has no character with that id;
 * - "derived-stat": a value was set on a derived stat (one with a formula);
 * - "out-of-range": a starting value of a character sheet is outside its allowed range;
 * - "cycle": derived stats depend on each other in a loop;
 * - "division-by-zero": a formula divides by zero (a literal zero is refused when the formula is read,
 *   a zero that comes from a value is refused when it is evaluated);
 * - "non-finite": a result of a formula is infinite or not a number;
 * - "argument": a method was called with a value that it does not accept.
 */
export type CharacterErrorCode =
  | "json"
  | "unsupported-document"
  | "invalid"
  | "bad-id"
  | "duplicate-id"
  | "expression"
  | "unknown-function"
  | "arity"
  | "unknown-stat"
  | "unknown-need"
  | "unknown-profession"
  | "unknown-trait"
  | "unknown-level"
  | "unknown-character"
  | "derived-stat"
  | "out-of-range"
  | "cycle"
  | "division-by-zero"
  | "non-finite"
  | "argument";

/** An error from the characters API. */
export class CharacterError extends Error {
  /** Why the call failed. */
  readonly code: CharacterErrorCode;

  /**
   * @param code - the failure reason.
   * @param message - a sentence for logs and developers.
   * @param options - an optional `cause`, such as the JSON parse error.
   */
  constructor(code: CharacterErrorCode, message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "CharacterError";
    this.code = code;
  }
}
