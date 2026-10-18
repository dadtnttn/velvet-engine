// Stable text and hashes for game state. A state snapshot is written with sorted keys and rounded
// numbers, so two runs that reach the same state write the same text, whatever order the game built
// its objects in. `hashState` gives a short fingerprint of that text, which replay checkpoints store.

import { TestingError } from "./errors";

/** Options for {@link stableStringify}. */
export interface StableJsonOptions {
  /** Digits kept after the decimal point of every number, from 0 to 15. Default 6. */
  readonly decimals?: number;
  /** Spaces per indent level. Default 0, which writes everything on one line. */
  readonly indent?: number;
}

/** Digits kept after the decimal point when the options name none. */
const DEFAULT_DECIMALS = 6;

/** The starting values of the two FNV-1a passes that make up a hash, and their shared prime. */
const FNV_OFFSET_A = 0x811c9dc5;
const FNV_OFFSET_B = 0x9e3779b9;
const FNV_PRIME = 0x01000193;

/**
 * Writes a value as JSON text with sorted object keys and rounded numbers.
 *
 * The rules: keys are sorted by their text; `undefined`, functions and symbols are left out of objects
 * and written as null in arrays; NaN and the infinities are written as the strings "NaN", "Infinity"
 * and "-Infinity"; -0 is written as 0; a bigint is written as its digits followed by "n"; a Map is
 * written as an object, a Set as a sorted array, and a value with a `toJSON` method as what that
 * method returns. A value that refers to itself is refused.
 *
 * @throws TestingError with code "bad-argument" for a bad `decimals` or `indent`, or for a cycle.
 */
export function stableStringify(value: unknown, options: StableJsonOptions = {}): string {
  const decimals = options.decimals ?? DEFAULT_DECIMALS;
  const indent = options.indent ?? 0;
  if (!Number.isInteger(decimals) || decimals < 0 || decimals > 15) {
    throw new TestingError("bad-argument", `decimals must be a whole number from 0 to 15, got ${decimals}`);
  }
  if (!Number.isInteger(indent) || indent < 0) {
    throw new TestingError("bad-argument", `indent must be a whole number from 0 up, got ${indent}`);
  }
  return new StableWriter(decimals, indent).write(value, "$", 0, []) ?? "null";
}

/**
 * Hashes a text to 16 hex digits. It runs two 32-bit FNV-1a passes with different starting values.
 * The result is stable across runs and spreads well enough to tell states apart. It is not a
 * cryptographic hash, and it is not meant to resist anyone who wants a collision.
 */
export function hashText(text: string): string {
  return fnvPass(text, FNV_OFFSET_A) + fnvPass(text, FNV_OFFSET_B);
}

/**
 * Hashes a game state: the stable text of `value`, then {@link hashText}. Equal states give equal
 * hashes, whatever order their objects were built in.
 */
export function hashState(value: unknown, options: StableJsonOptions = {}): string {
  return hashText(stableStringify(value, options));
}

/** One 32-bit FNV-1a pass over the UTF-16 code units of a text, as eight hex digits. */
function fnvPass(text: string, offset: number): string {
  let hash = offset >>> 0;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= text.charCodeAt(index);
    hash = Math.imul(hash, FNV_PRIME) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

/**
 * Writes one value. The indent and the cycle check live here, so that the object, array, Set and Map
 * rules share them.
 */
class StableWriter {
  constructor(
    private readonly decimals: number,
    private readonly indent: number,
  ) {}

  /** The JSON text of `value`, or undefined when the value is left out. */
  write(value: unknown, path: string, depth: number, ancestors: readonly object[]): string | undefined {
    if (value === undefined || typeof value === "function" || typeof value === "symbol") return undefined;
    if (value === null) return "null";
    switch (typeof value) {
      case "string":
        return JSON.stringify(value);
      case "boolean":
        return value ? "true" : "false";
      case "bigint":
        return JSON.stringify(`${value}n`);
      case "number":
        return this.number(value);
      default:
        return this.object(value as object, path, depth, ancestors);
    }
  }

  /** A number, rounded to the configured digits. Non-finite numbers become strings. */
  private number(value: number): string {
    if (Number.isNaN(value)) return '"NaN"';
    if (value === Infinity) return '"Infinity"';
    if (value === -Infinity) return '"-Infinity"';
    const factor = 10 ** this.decimals;
    const scaled = value * factor;
    const rounded = Number.isFinite(scaled) ? Math.round(scaled) / factor : value;
    return JSON.stringify(rounded === 0 ? 0 : rounded);
  }

  /** An object, an array, a typed array, a Set, a Map, or a value that has its own toJSON. */
  private object(value: object, path: string, depth: number, ancestors: readonly object[]): string | undefined {
    if (ancestors.includes(value)) {
      throw new TestingError("bad-argument", `cannot snapshot a value that refers to itself, at ${path}`);
    }
    const toJSON = (value as { toJSON?: unknown }).toJSON;
    if (typeof toJSON === "function") {
      const replaced: unknown = (toJSON as () => unknown).call(value);
      if (replaced !== value) return this.write(replaced, path, depth, ancestors);
    }
    const chain = [...ancestors, value];
    if (Array.isArray(value)) {
      const items = Array.from(value, (item, index) => this.write(item, `${path}[${index}]`, depth + 1, chain) ?? "null");
      return this.list(items, depth);
    }
    if (ArrayBuffer.isView(value)) {
      const items = Array.from(value as unknown as ArrayLike<unknown>, (item) => this.write(item, path, depth + 1, chain) ?? "null");
      return this.list(items, depth);
    }
    if (value instanceof Set) {
      const items = [...value].map((item, index) => this.write(item, `${path}[${index}]`, depth + 1, chain) ?? "null");
      return this.list(items.sort(), depth);
    }
    if (value instanceof Map) {
      const entries: Array<[string, unknown]> = [];
      for (const [key, item] of value) entries.push([String(key), item]);
      return this.members(entries, path, depth, chain);
    }
    const entries: Array<[string, unknown]> = Object.keys(value).map((key) => [key, Reflect.get(value, key)]);
    return this.members(entries, path, depth, chain);
  }

  /** The members of an object, with sorted keys. A member that is left out writes nothing. */
  private members(entries: Array<[string, unknown]>, path: string, depth: number, chain: readonly object[]): string {
    const sorted = [...entries].sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0));
    const parts: string[] = [];
    for (const [key, item] of sorted) {
      const text = this.write(item, `${path}.${key}`, depth + 1, chain);
      if (text !== undefined) parts.push(`${this.newline(depth + 1)}${JSON.stringify(key)}:${this.space()}${text}`);
    }
    return this.wrap("{", "}", parts, depth);
  }

  /** The items of an array or a Set, already written. */
  private list(items: readonly string[], depth: number): string {
    return this.wrap("[", "]", items.map((item) => `${this.newline(depth + 1)}${item}`), depth);
  }

  /** Joins the parts that sit between an opening and a closing mark, with the indent. */
  private wrap(open: string, close: string, parts: readonly string[], depth: number): string {
    if (parts.length === 0) return open + close;
    return `${open}${parts.join(",")}${this.newline(depth)}${close}`;
  }

  /** A line break and the indent of a depth, or nothing when the output is on one line. */
  private newline(depth: number): string {
    return this.indent === 0 ? "" : `\n${" ".repeat(this.indent * depth)}`;
  }

  /** The space after a colon, which only the indented output has. */
  private space(): string {
    return this.indent === 0 ? "" : " ";
  }
}
