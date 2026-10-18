// The error type of @velvet/assets. Code that handles a refused manifest or an unknown id branches
// on `code`; the message is for logs and developers. A refused manifest lists every problem it
// found in `issues`, so one run shows all of them.

/**
 * Why a call failed:
 * - "json": the manifest text is not JSON;
 * - "format": the format tag is not "velvet-art";
 * - "version": the version is not 1;
 * - "structure": a field is missing or has the wrong JSON type;
 * - "value": a value breaks a rule (the spelling of an id or a variant, a kind, a size, an anchor);
 * - "duplicate": two entries share an id, two variants share a name, or two files would share a name;
 * - "unknown-id": the id is not in the manifest;
 * - "unknown-variant": the entry does not declare that variant.
 */
export type ArtErrorCode =
  | "json"
  | "format"
  | "version"
  | "structure"
  | "value"
  | "duplicate"
  | "unknown-id"
  | "unknown-variant";

/** One problem found in a manifest. */
export interface ArtIssue {
  /** The rule that broke. */
  readonly code: ArtErrorCode;
  /** A sentence in English for logs and developers. Its wording is not part of the format. */
  readonly message: string;
  /** Where the problem is, such as `entries[2].size`. Absent when it concerns the whole document. */
  readonly path?: string;
}

/** An error thrown by this package. */
export class ArtError extends Error {
  /** Why the call failed. */
  readonly code: ArtErrorCode;
  /** Every problem found. Never empty: a single failure has one issue. */
  readonly issues: readonly ArtIssue[];

  /**
   * @param code - the failure reason.
   * @param message - a sentence for logs and developers.
   * @param issues - the problems found, when there are several; a single issue is made from the code and message.
   * @param options - an optional `cause`, such as the JSON parse error.
   */
  constructor(code: ArtErrorCode, message: string, issues: readonly ArtIssue[] = [], options?: ErrorOptions) {
    super(message, options);
    this.name = "ArtError";
    this.code = code;
    this.issues = issues.length > 0 ? issues : [{ code, message }];
  }
}
