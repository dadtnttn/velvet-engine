// The issues of a failure. A reader that refuses a document throws its own error, and most of them list every
// problem they found: each one becomes an issue with its path. Any other failure becomes one issue, with its code
// and its message. The readers' error classes are all known here, so each check reports them the same way.

import { ArtError } from "@velvet/assets";
import { ArtPrepError } from "@velvet/artprep";
import { AchievementError } from "@velvet/achievements";
import { AudioError } from "@velvet/audio";
import { CardsError } from "@velvet/cards";
import { CharacterError } from "@velvet/characters";
import { TilesError } from "@velvet/tiles";
import { makeIssue, WHOLE_FILE, type Issue } from "../issues";

/** A problem that a reader lists: its code, its sentence, and its path when it has one. */
interface ListedProblem {
  readonly code: string;
  readonly message: string;
  readonly path?: string;
}

/**
 * The code of a failure: the reader's own code when it has one, otherwise "internal" (a bug, or a module that could
 * not load).
 * @param error - what was thrown.
 */
export function codeOf(error: unknown): string {
  const code = typeof error === "object" && error !== null ? Reflect.get(error, "code") : undefined;
  return typeof code === "string" ? code : "internal";
}

/**
 * The message of a failure.
 * @param error - what was thrown.
 */
export function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * The errors of a file, one issue for each problem that the reader listed.
 * @param file - the file, relative to the game folder.
 * @param problems - the problems the reader listed.
 */
function listedIssues(file: string, problems: readonly ListedProblem[]): Issue[] {
  return problems.map((item) => makeIssue("error", file, item.path ?? WHOLE_FILE, item.code, item.message));
}

/**
 * The issues of a failure from a reader. A reader that lists its problems gives one issue for each; any other failure
 * gives one issue with the code and the message.
 * @param file - the document's file.
 * @param error - what the reader threw.
 */
export function failureIssues(file: string, error: unknown): Issue[] {
  if (error instanceof ArtError || error instanceof ArtPrepError) return listedIssues(file, error.issues);
  if (error instanceof AudioError || error instanceof AchievementError || error instanceof TilesError) {
    return listedIssues(file, error.issues);
  }
  if (error instanceof CardsError || error instanceof CharacterError) {
    return [makeIssue("error", file, WHOLE_FILE, error.code, error.message)];
  }
  return [makeIssue("error", file, WHOLE_FILE, codeOf(error), messageOf(error))];
}
