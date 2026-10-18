// Reading helpers for the parsers. Each reader checks one value and, when the value is wrong, records a
// problem with its path and returns undefined. A document is therefore reported in full: every problem
// is listed, not only the first one.

import { ArtPrepError, type ArtPrepErrorCode, type ArtPrepIssue } from "./error";
import { ARTPREP_FORMAT, ARTPREP_VERSION, type ArtPrepKind, type Phrase } from "./types";

/** The problems found in one document, in the order they were found. */
export type Issues = ArtPrepIssue[];

/** An art id, spelled as the art manifest spells it (docs/formats/ART_MANIFEST.md). */
export const ART_ID_PATTERN = /^[a-z0-9_-]+(\.[a-z0-9_-]+)*$/;

/** The longest art id, in characters. */
export const ART_ID_LIMIT = 128;

/** A variant name, spelled as the art manifest spells it. */
export const VARIANT_PATTERN = /^[a-z0-9_-]+$/;

/** The longest variant name, in characters. */
export const VARIANT_LIMIT = 32;

/** A profile id or a shot id: lowercase letters, digits, `_` and `-`, starting with a letter or a digit. */
export const SLUG_PATTERN = /^[a-z0-9][a-z0-9_-]{0,63}$/;

/** A character id, spelled as docs/formats/CHARACTER.md spells it. */
export const CHARACTER_PATTERN = /^[a-z][a-z0-9_]{0,63}$/;

/** The longest phrase, in characters. */
export const PHRASE_LIMIT = 200;

/** The keys that every document has. */
export const ENVELOPE_KEYS: readonly string[] = ["format", "version", "kind"];

/** Joins a path and a key: the path `shots[0]` and the key `art` make `shots[0].art`. */
export function joinPath(path: string, key: string): string {
  return path === "" ? key : `${path}.${key}`;
}

/** Records one problem. The path is left out when it is empty. */
export function addIssue(issues: Issues, code: ArtPrepErrorCode, message: string, path: string): void {
  issues.push(path === "" ? { code, message } : { code, message, path });
}

/** True for an object that is neither null nor an array. */
export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Builds the error for a document or a batch that has problems. The first problem gives the code. */
export function problemsError(issues: readonly ArtPrepIssue[], what: string): ArtPrepError {
  const first = issues[0];
  const count = issues.length === 1 ? "1 problem" : `${issues.length} problems`;
  const message = `${what} has ${count}; the first: ${first?.message ?? "unknown"}`;
  return new ArtPrepError(first?.code ?? "structure", message, issues);
}

/**
 * Parses the text of a document and checks its envelope: the format tag, the version and the kind.
 * A wrong envelope stops the reading at once, because nothing else of the document can be trusted.
 * @param text - the JSON text.
 * @param kind - the kind the call reads.
 */
export function readDocument(text: string, kind: ArtPrepKind): Record<string, unknown> {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (error) {
    throw new ArtPrepError("json", `the ${kind} document is not JSON`, [], { cause: error });
  }
  if (!isRecord(parsed)) throw new ArtPrepError("structure", `the ${kind} document must be a JSON object`);
  if (parsed.format !== ARTPREP_FORMAT) throw new ArtPrepError("format", `the format must be "${ARTPREP_FORMAT}"`);
  if (parsed.version !== ARTPREP_VERSION) throw new ArtPrepError("version", "the version must be 1");
  if (parsed.kind !== kind) {
    throw new ArtPrepError("kind", `the kind must be "${kind}", not "${String(parsed.kind)}"`);
  }
  return parsed;
}

/** Records a problem for each key that the format does not define. */
export function checkKeys(
  object: Record<string, unknown>,
  allowed: readonly string[],
  path: string,
  issues: Issues,
): void {
  for (const key of Object.keys(object)) {
    if (!allowed.includes(key)) {
      const at = joinPath(path, key);
      addIssue(issues, "unknown-field", `"${at}" is not a name of this format`, at);
    }
  }
}

/** Records a problem when a required value is missing. Returns true when it is missing. */
export function isMissing(value: unknown, path: string, issues: Issues): boolean {
  if (value !== undefined) return false;
  addIssue(issues, "structure", `${path} is required`, path);
  return true;
}

/** Reads an object. */
export function readObject(value: unknown, path: string, issues: Issues): Record<string, unknown> | undefined {
  if (isRecord(value)) return value;
  if (value === undefined) {
    isMissing(value, path, issues);
    return undefined;
  }
  addIssue(issues, "structure", `${path} must be an object`, path);
  return undefined;
}

/** Reads an array of at most `max` items. Too many items are a value problem, and the items are still returned. */
export function readArray(value: unknown, path: string, issues: Issues, max: number): readonly unknown[] | undefined {
  if (value === undefined) {
    isMissing(value, path, issues);
    return undefined;
  }
  if (!Array.isArray(value)) {
    addIssue(issues, "structure", `${path} must be an array`, path);
    return undefined;
  }
  if (value.length > max) addIssue(issues, "value", `${path} has ${value.length} items; the most is ${max}`, path);
  return value;
}

/** Reads a line of text: a string, trimmed, of 1 to `max` characters, on one line. */
export function readText(value: unknown, path: string, issues: Issues, max: number): string | undefined {
  if (isMissing(value, path, issues)) return undefined;
  if (typeof value !== "string") {
    addIssue(issues, "structure", `${path} must be a string`, path);
    return undefined;
  }
  const text = value.trim();
  if (text.length === 0 || text.length > max || /[\r\n]/.test(text)) {
    addIssue(issues, "value", `${path} must be 1 to ${max} characters on one line`, path);
    return undefined;
  }
  return text;
}

/** Reads free text: a string of at most `max` characters, which may span lines and may be empty. */
export function readFreeText(value: unknown, path: string, issues: Issues, max: number): string | undefined {
  if (isMissing(value, path, issues)) return undefined;
  if (typeof value !== "string") {
    addIssue(issues, "structure", `${path} must be a string`, path);
    return undefined;
  }
  if (value.length > max) {
    addIssue(issues, "value", `${path} must be at most ${max} characters`, path);
    return undefined;
  }
  return value;
}

/** Reads a joiner: a string of up to 8 characters, on one line. It may be empty, and it is not trimmed. */
export function readJoiner(value: unknown, path: string, issues: Issues): string | undefined {
  if (isMissing(value, path, issues)) return undefined;
  if (typeof value !== "string") {
    addIssue(issues, "structure", `${path} must be a string`, path);
    return undefined;
  }
  if (value.length > 8 || /[\r\n]/.test(value)) {
    addIssue(issues, "value", `${path} must be at most 8 characters on one line`, path);
    return undefined;
  }
  return value;
}

/** Reads a whole number from `min` to `max`, both included. */
export function readInteger(value: unknown, path: string, issues: Issues, min: number, max: number): number | undefined {
  if (isMissing(value, path, issues)) return undefined;
  if (typeof value !== "number") {
    addIssue(issues, "structure", `${path} must be a number`, path);
    return undefined;
  }
  if (!Number.isInteger(value) || value < min || value > max) {
    addIssue(issues, "value", `${path} must be a whole number from ${min} to ${max}`, path);
    return undefined;
  }
  return value;
}

/** Reads a finite number from `min` to `max`, both included. */
export function readNumber(value: unknown, path: string, issues: Issues, min: number, max: number): number | undefined {
  if (isMissing(value, path, issues)) return undefined;
  if (typeof value !== "number") {
    addIssue(issues, "structure", `${path} must be a number`, path);
    return undefined;
  }
  if (!Number.isFinite(value) || value < min || value > max) {
    addIssue(issues, "value", `${path} must be a number from ${min} to ${max}`, path);
    return undefined;
  }
  return value;
}

/** Reads a true or false value. */
export function readBoolean(value: unknown, path: string, issues: Issues): boolean | undefined {
  if (isMissing(value, path, issues)) return undefined;
  if (typeof value !== "boolean") {
    addIssue(issues, "structure", `${path} must be true or false`, path);
    return undefined;
  }
  return value;
}

/** Reads one word of a fixed list of choices. */
export function readChoice<T extends string>(
  value: unknown,
  path: string,
  issues: Issues,
  choices: readonly T[],
): T | undefined {
  if (isMissing(value, path, issues)) return undefined;
  if (typeof value !== "string") {
    addIssue(issues, "structure", `${path} must be a string`, path);
    return undefined;
  }
  const choice = choices.find((option) => option === value);
  if (choice === undefined) {
    addIssue(issues, "value", `${path} must be one of ${choices.map((option) => `"${option}"`).join(", ")}`, path);
  }
  return choice;
}

/** Reads an id that matches `pattern` and is at most `max` characters long. `what` names it in the message. */
export function readIdentifier(
  value: unknown,
  path: string,
  issues: Issues,
  pattern: RegExp,
  max: number,
  what: string,
): string | undefined {
  if (isMissing(value, path, issues)) return undefined;
  if (typeof value !== "string") {
    addIssue(issues, "structure", `${path} must be a string`, path);
    return undefined;
  }
  if (value.length > max || !pattern.test(value)) {
    addIssue(issues, "value", `${path} "${value}" is not a valid ${what}`, path);
    return undefined;
  }
  return value;
}

/** Reads an optional variant name. Absent and null both mean no variant, and that is null. */
export function readOptionalVariant(value: unknown, path: string, issues: Issues): string | null {
  if (value === undefined || value === null) return null;
  return readIdentifier(value, path, issues, VARIANT_PATTERN, VARIANT_LIMIT, "variant name") ?? null;
}

/** Reads a list of ids. A repeated id is a duplicate. */
export function readIdentifierList(
  value: unknown,
  path: string,
  issues: Issues,
  max: number,
  pattern: RegExp,
  limit: number,
  what: string,
): string[] {
  const list = readArray(value, path, issues, max);
  const ids: string[] = [];
  list?.forEach((item, index) => {
    const at = `${path}[${index}]`;
    const id = readIdentifier(item, at, issues, pattern, limit, what);
    if (id === undefined) return;
    if (ids.includes(id)) addIssue(issues, "duplicate", `${what} "${id}" is listed twice`, at);
    ids.push(id);
  });
  return ids;
}

/** Reads a list of lines of text. Items that are not valid are left out, and their problems are recorded. */
export function readTextList(value: unknown, path: string, issues: Issues, max: number, itemMax: number): string[] {
  const list = readArray(value, path, issues, max);
  const texts: string[] = [];
  list?.forEach((item, index) => {
    const text = readText(item, `${path}[${index}]`, issues, itemMax);
    if (text !== undefined) texts.push(text);
  });
  return texts;
}

/**
 * Reads a phrase: either a line of text, or an object with `text` and an optional `weight`. A weight is
 * above 0 and at most 2. A plain string has the weight 1.
 */
export function readPhrase(value: unknown, path: string, issues: Issues): Phrase | undefined {
  if (typeof value === "string") {
    const text = readText(value, path, issues, PHRASE_LIMIT);
    return text === undefined ? undefined : { text, weight: 1 };
  }
  const object = readObject(value, path, issues);
  if (object === undefined) return undefined;
  checkKeys(object, ["text", "weight"], path, issues);
  const text = readText(object.text, joinPath(path, "text"), issues, PHRASE_LIMIT);
  const raw = object.weight;
  let weight = 1;
  if (raw !== undefined) {
    if (typeof raw !== "number" || !Number.isFinite(raw) || raw <= 0 || raw > 2) {
      const at = joinPath(path, "weight");
      addIssue(issues, "value", `${at} must be a number above 0 and at most 2`, at);
      return undefined;
    }
    weight = raw;
  }
  return text === undefined ? undefined : { text, weight };
}

/** Reads a list of phrases. Items that are not valid are left out, and their problems are recorded. */
export function readPhraseList(value: unknown, path: string, issues: Issues, max: number): Phrase[] {
  const list = readArray(value, path, issues, max);
  const phrases: Phrase[] = [];
  list?.forEach((item, index) => {
    const phrase = readPhrase(item, `${path}[${index}]`, issues);
    if (phrase !== undefined) phrases.push(phrase);
  });
  return phrases;
}
