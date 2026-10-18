// Small helpers shared by the modules of this package. Not part of the public API.

import { CharacterError } from "./error";

/** True for an object that is neither null nor an array. */
export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Keeps `value` between `min` and `max`. When `min` is above `max`, the result is `max`. */
export function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

/** Compares two strings by their UTF-16 code units. Every sort in this package uses this order. */
export function compareText(a: string, b: string): number {
  if (a === b) return 0;
  return a < b ? -1 : 1;
}

/** Throws an argument error unless `value` is a finite number. */
export function assertFinite(value: number, name: string): void {
  if (!Number.isFinite(value)) throw new CharacterError("argument", `${name} must be a finite number`);
}

/** The entry under `key`. Every map that is read here holds all its keys, so a missing one is a bug in this package. */
export function requireEntry<T>(map: ReadonlyMap<string, T>, key: string): T {
  const value = map.get(key);
  if (value === undefined) throw new Error(`internal: no entry for "${key}"`);
  return value;
}

/** Builds a map from a list, keyed by each item's id, in the list's order. */
export function indexById<T extends { readonly id: string }>(items: readonly T[]): Map<string, T> {
  return new Map(items.map((item): [string, T] => [item.id, item]));
}
