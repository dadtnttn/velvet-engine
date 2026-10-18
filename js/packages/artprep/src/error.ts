// The error type of @velvet/artprep. Code that handles a refused document or a refused batch branches
// on `code`; the message is for logs and developers. A refusal lists every problem it found in
// `issues`, so one run shows all of them.

/**
 * Why a call failed:
 * - "json": the text is not JSON;
 * - "format": the format tag is not "velvet-artprep";
 * - "version": the version is not 1;
 * - "kind": the document is not the kind that the call reads;
 * - "structure": a field is missing or has the wrong JSON type;
 * - "value": a value breaks a rule (a spelling, a range, or a combination of fields);
 * - "duplicate": two entries share an id, two shots make the same picture, or a value is listed twice;
 * - "unknown-field": a name that the format does not define;
 * - "unknown-id", "unknown-variant": an art id or a variant is not in the art manifest;
 * - "unknown-character": a shot names a character that has no identity layer;
 * - "identities": a shot shows more characters than its profile allows;
 * - "references": a shot uses more input images than its profile allows;
 * - "edit": an edit does not fit its profile: a missing edit, an edit on a profile that makes new
 *   pictures, or an edit that changes more than one thing.
 */
export type ArtPrepErrorCode =
  | "json"
  | "format"
  | "version"
  | "kind"
  | "structure"
  | "value"
  | "duplicate"
  | "unknown-field"
  | "unknown-id"
  | "unknown-variant"
  | "unknown-character"
  | "identities"
  | "references"
  | "edit";

/** One problem found in a document or a batch. */
export interface ArtPrepIssue {
  /** The rule that broke. */
  readonly code: ArtPrepErrorCode;
  /** A sentence in English for logs and developers. Its wording is not part of the format. */
  readonly message: string;
  /** Where the problem is, such as `profiles[1].prompt.order`. Absent when it concerns the whole input. */
  readonly path?: string;
}

/** An error thrown by this package. */
export class ArtPrepError extends Error {
  /** Why the call failed: the code of the first problem. */
  readonly code: ArtPrepErrorCode;
  /** Every problem found. Never empty: a single failure has one issue. */
  readonly issues: readonly ArtPrepIssue[];

  /**
   * @param code - the failure reason.
   * @param message - a sentence for logs and developers.
   * @param issues - the problems found, when there are several; a single issue is made from the code and message.
   * @param options - an optional `cause`, such as the JSON parse error.
   */
  constructor(code: ArtPrepErrorCode, message: string, issues: readonly ArtPrepIssue[] = [], options?: ErrorOptions) {
    super(message, options);
    this.name = "ArtPrepError";
    this.code = code;
    this.issues = issues.length > 0 ? issues : [{ code, message }];
  }
}
