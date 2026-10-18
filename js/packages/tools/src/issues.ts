// Problems found in a game: where each one is, which rule broke, and how serious it is.

/** How serious a problem is. An error makes the command fail; a warning is reported and does not. */
export type Severity = "error" | "warning";

/** One problem: its file, its place in the file, a machine code, and a sentence for people. */
export interface Issue {
  readonly severity: Severity;
  /** The file, relative to the game folder, with forward slashes. */
  readonly file: string;
  /** The place in the file: a JSON path such as `cards[2].stack.max`, or `(file)` for the whole file. */
  readonly path: string;
  /** A machine-readable code, such as `duplicate-card`, `missing-key` or `json`. */
  readonly code: string;
  /** A sentence in English for people. Its wording is not a contract. */
  readonly message: string;
}

/** The place of a problem that concerns the whole file. */
export const WHOLE_FILE = "(file)";

/**
 * Builds one issue.
 * @param severity - error or warning.
 * @param file - the file, relative to the game folder.
 * @param path - the place in the file, or WHOLE_FILE.
 * @param code - the machine-readable code.
 * @param message - the sentence for people.
 */
export function makeIssue(severity: Severity, file: string, path: string, code: string, message: string): Issue {
  return { severity, file, path, code, message };
}

/**
 * The line that `check` and `i18n` print for one issue: `file: path: severity: message`.
 * @param issue - the problem.
 */
export function formatIssue(issue: Issue): string {
  return `${issue.file}: ${issue.path}: ${issue.severity}: ${issue.message}`;
}

/**
 * How many errors and warnings a list of issues holds.
 * @param issues - the problems.
 */
export function countIssues(issues: readonly Issue[]): { readonly errors: number; readonly warnings: number } {
  let errors = 0;
  let warnings = 0;
  for (const issue of issues) {
    if (issue.severity === "error") errors += 1;
    else warnings += 1;
  }
  return { errors, warnings };
}
