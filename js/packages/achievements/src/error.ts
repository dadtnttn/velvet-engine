// The error type of @velvet/achievements. Code that handles a refused document or a refused call
// branches on `code`; the message is for logs and developers. A refused document lists every problem
// it found in `issues`, so one run shows all of them.

/**
 * Why a call failed:
 * - "json": the definitions text is not JSON;
 * - "format": the format tag or the kind is not one this reader reads;
 * - "version": the version is not 1;
 * - "structure": a field is missing or has the wrong JSON type;
 * - "bad-id": an id, a group or an art id does not match its pattern;
 * - "value": a value breaks a range or a rule (a key's length, a number, a goal, progress steps);
 * - "duplicate": two counters, two achievements or two references of one list share an id;
 * - "unknown-counter": a counter goal, or a call, names no counter of the definitions;
 * - "unknown-achievement": an `all` or `any` list, or a call, names no achievement of the definitions;
 * - "cycle": the `all` and `any` goals form a loop;
 * - "not-manual": `unlock` names an achievement whose goal is not manual, and no force was given;
 * - "dev-only": `force` or `resetAll` was used outside a dev build;
 * - "argument": a call gives a value that it does not accept (a count, a delta or a clock time).
 */
export type AchievementErrorCode =
  | "json"
  | "format"
  | "version"
  | "structure"
  | "bad-id"
  | "value"
  | "duplicate"
  | "unknown-counter"
  | "unknown-achievement"
  | "cycle"
  | "not-manual"
  | "dev-only"
  | "argument";

/** One problem found in a document, or in a call. */
export interface AchievementIssue {
  /** The rule that broke. */
  readonly code: AchievementErrorCode;
  /** A sentence in English for logs and developers. Its wording is not part of the format. */
  readonly message: string;
  /** Where the problem is, such as `achievements[2].goal.any[1]`. Absent when it concerns the whole document. */
  readonly path?: string;
}

/** An error thrown by this package. */
export class AchievementError extends Error {
  /** Why the call failed. */
  readonly code: AchievementErrorCode;
  /** Every problem found. Never empty: a single failure has one issue. */
  readonly issues: readonly AchievementIssue[];

  /**
   * @param code - the failure reason.
   * @param message - a sentence for logs and developers.
   * @param issues - the problems found, when there are several. A single issue is made from the code and message.
   * @param options - an optional `cause`, such as the JSON parse error.
   */
  constructor(code: AchievementErrorCode, message: string, issues: readonly AchievementIssue[] = [], options?: ErrorOptions) {
    super(message, options);
    this.name = "AchievementError";
    this.code = code;
    this.issues = issues.length > 0 ? issues : [{ code, message }];
  }
}
