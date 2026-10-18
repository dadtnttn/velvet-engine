// Reading JSON text and the fields of a document. Each check names the field that broke, so that a
// message can point at it. Internal to the package: the public readers are in schema.ts and
// definition.ts.

import { CharacterError } from "./error";
import { CHARACTER_FORMAT, CHARACTER_VERSION } from "./types";
import { isRecord } from "./util";

/** An id: a lowercase letter, then up to 63 lowercase letters, digits or underscores. */
const ID_PATTERN = /^[a-z][a-z0-9_]{0,63}$/;

/**
 * An art id, as the art manifest (docs/formats/ART_MANIFEST.md) writes it: segments of lowercase letters,
 * digits, `_` and `-`, joined by single dots, and at most 128 characters in all.
 */
const ART_PATTERN = /^[a-z0-9_-]+(\.[a-z0-9_-]+)*$/;

/** The longest art id that the art manifest accepts. */
const MAX_ART_ID_LENGTH = 128;

/** The longest i18n key, or reason, that the format accepts. */
export const MAX_KEY_LENGTH = 200;

/** True when `value` is an id. */
export function isId(value: unknown): value is string {
  return typeof value === "string" && ID_PATTERN.test(value);
}

/**
 * The value that a JSON object holds under `key`, or undefined when the object has no such key of its
 * own. A plain lookup is not enough: `constructor` is a valid id, and objects inherit a `constructor`.
 */
export function ownValue(record: Record<string, unknown>, key: string): unknown {
  return Object.hasOwn(record, key) ? record[key] : undefined;
}

/**
 * Parses JSON text.
 * @throws {CharacterError} with code "json" when the text is not JSON.
 */
export function parseJson(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch (cause) {
    throw new CharacterError("json", "the text is not JSON", { cause });
  }
}

/** A plain "invalid" error with a message for developers. */
export function invalid(message: string): CharacterError {
  return new CharacterError("invalid", message);
}

/** Reads a JSON object, or throws naming `path`. */
export function readRecord(value: unknown, path: string): Record<string, unknown> {
  if (!isRecord(value)) throw invalid(`${path} must be an object`);
  return value;
}

/** Reads a JSON array, or throws naming `path`. */
export function readList(value: unknown, path: string): readonly unknown[] {
  if (!Array.isArray(value)) throw invalid(`${path} must be a list`);
  return value;
}

/**
 * Reads the list under `key` of `record`, or an empty list when the key is absent.
 * @param path - the path of `record`, for the message.
 */
export function optionalList(record: Record<string, unknown>, path: string, key: string): readonly unknown[] {
  const value = record[key];
  return value === undefined ? [] : readList(value, `${path}.${key}`);
}

/** Reads an id. @throws {CharacterError} with code "bad-id" when the value is not one. */
export function readId(value: unknown, path: string): string {
  if (!isId(value)) {
    throw new CharacterError(
      "bad-id",
      `${path} must be an id: a lowercase letter, then lowercase letters, digits or "_", up to 64 characters`,
    );
  }
  return value;
}

/** Reads an art id. @throws {CharacterError} with code "bad-id" when the value is not one. */
export function readArtId(value: unknown, path: string): string {
  if (typeof value !== "string" || value.length > MAX_ART_ID_LENGTH || !ART_PATTERN.test(value)) {
    throw new CharacterError(
      "bad-id",
      `${path} must be an art id: lowercase letters, digits, "_" and "-", joined by dots, ` +
        `up to ${MAX_ART_ID_LENGTH} characters`,
    );
  }
  return value;
}

/** Reads an i18n key, or a reason: text of 1 to 200 characters. */
export function readKey(value: unknown, path: string): string {
  if (typeof value !== "string" || value.length === 0 || value.length > MAX_KEY_LENGTH) {
    throw invalid(`${path} must be an i18n key: text of 1 to ${MAX_KEY_LENGTH} characters`);
  }
  return value;
}

/** Reads a finite number. */
export function readNumber(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) throw invalid(`${path} must be a finite number`);
  return value;
}

/** Reads a whole number from `min` to `max`. */
export function readInteger(value: unknown, path: string, min: number, max: number): number {
  if (typeof value !== "number" || !Number.isInteger(value) || value < min || value > max) {
    throw invalid(`${path} must be a whole number from ${min} to ${max}`);
  }
  return value;
}

/** Reads one of the allowed strings. */
export function readOneOf<T extends string>(value: unknown, path: string, allowed: readonly T[]): T {
  const found = allowed.find((item) => item === value);
  if (found === undefined) {
    throw invalid(`${path} must be one of ${allowed.map((item) => `"${item}"`).join(", ")}`);
  }
  return found;
}

/**
 * Reads the envelope of a schema or character document: the format, the version and the kind must be
 * the ones this package reads.
 * @throws {CharacterError} with code "unsupported-document" when they are not.
 */
export function readEnvelope(value: unknown, kind: string): Record<string, unknown> {
  const doc = readRecord(value, "the document");
  if (doc["format"] !== CHARACTER_FORMAT || doc["version"] !== CHARACTER_VERSION || doc["kind"] !== kind) {
    throw new CharacterError(
      "unsupported-document",
      `expected a ${CHARACTER_FORMAT} document of kind "${kind}" and version ${CHARACTER_VERSION}`,
    );
  }
  return doc;
}

/**
 * Records an id in a list's set, and throws when it was there already.
 * @throws {CharacterError} with code "duplicate-id".
 */
export function checkUnique(seen: Set<string>, id: string, path: string): void {
  if (seen.has(id)) throw new CharacterError("duplicate-id", `${path}: "${id}" is listed twice`);
  seen.add(id);
}
