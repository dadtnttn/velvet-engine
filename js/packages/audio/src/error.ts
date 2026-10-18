// The error type of @velvet/audio. Code that handles a refused manifest or a call with an unknown
// name branches on `code`; the message is for logs and developers. A refused manifest lists every
// problem it found in `issues`, so one run shows all of them.

/**
 * Why a call failed:
 * - "json": the manifest text is not JSON;
 * - "format": the format tag is not "velvet-audio";
 * - "version": the version is not 1;
 * - "structure": a field is missing or has the wrong JSON type;
 * - "value": a value breaks a rule (an id, a bus name, a file path, a volume, loop points, a limit);
 * - "duplicate": two sounds share an id, two candidates of one sound are the same file, or a bus name repeats;
 * - "unknown-id": no sound of the manifest has the id;
 * - "unknown-bus": the bus is neither built in nor declared by the manifest;
 * - "wrong-bus": a music call names a sound that is not on the music bus.
 */
export type AudioErrorCode =
  | "json"
  | "format"
  | "version"
  | "structure"
  | "value"
  | "duplicate"
  | "unknown-id"
  | "unknown-bus"
  | "wrong-bus";

/** One problem found in a manifest, or in a call. */
export interface AudioIssue {
  /** The rule that broke. */
  readonly code: AudioErrorCode;
  /** A sentence in English for logs and developers. Its wording is not part of the format. */
  readonly message: string;
  /** Where the problem is, such as `sounds[2].files[1]`. Absent when it concerns the whole document. */
  readonly path?: string;
}

/** An error thrown by this package. */
export class AudioError extends Error {
  /** Why the call failed. */
  readonly code: AudioErrorCode;
  /** Every problem found. Never empty: a single failure has one issue. */
  readonly issues: readonly AudioIssue[];

  /**
   * @param code - the failure reason.
   * @param message - a sentence for logs and developers.
   * @param issues - the problems found, when there are several. A single issue is made from the code and message.
   * @param options - an optional `cause`, such as the JSON parse error.
   */
  constructor(code: AudioErrorCode, message: string, issues: readonly AudioIssue[] = [], options?: ErrorOptions) {
    super(message, options);
    this.name = "AudioError";
    this.code = code;
    this.issues = issues.length > 0 ? issues : [{ code, message }];
  }
}

/** A short text for any thrown value, for messages. */
export function describeError(error: unknown): string {
  if (error instanceof Error) return error.message;
  return typeof error === "string" ? error : String(error);
}
