// The error type of the testing tools. Code that handles a failure branches on `code`; the message
// is for the developer who reads the test output.

/**
 * Why a testing operation failed:
 * - "bad-argument": an option or an argument has a value that the tools cannot use;
 * - "not-running": no frame is scheduled, because the game's loop has stopped or never started;
 * - "timeout": a condition did not come true within its frame budget;
 * - "clock-stalled": game time did not move as far as asked, because the clock is paused or the
 *   frame budget ran out;
 * - "bad-replay": a replay document does not match the format (see docs/formats/REPLAY.md);
 * - "disposed": the driver was disposed, so it cannot run any more.
 */
export type TestingErrorCode =
  | "bad-argument"
  | "not-running"
  | "timeout"
  | "clock-stalled"
  | "bad-replay"
  | "disposed";

/** An error raised by the testing tools. */
export class TestingError extends Error {
  /** Why the operation failed. */
  readonly code: TestingErrorCode;

  /**
   * @param code - the failure reason.
   * @param message - a sentence for the developer.
   * @param options - an optional `cause`.
   */
  constructor(code: TestingErrorCode, message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "TestingError";
    this.code = code;
  }
}
