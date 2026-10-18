// Reads JSON fields with checks and paths. A read keeps going after a problem, so one run reports every
// problem in the document. After a problem each reader still returns a usable value (a default, or an
// empty list), and the caller throws the whole list when the read is done. The first issue of the list
// is the root cause, because the checks run from the top of the document down.

import type { TilesErrorCode, TilesIssue } from "./error";

/** The problems found so far in one read, in the order of the document. */
export class Problems {
  /** Every problem recorded, in order. */
  readonly issues: TilesIssue[] = [];

  /** Records a problem at `path`. An empty path means the whole document. */
  add(code: TilesErrorCode, message: string, path: string): void {
    this.issues.push(path === "" ? { code, message } : { code, message, path });
  }
}

/** True for a JSON object: not null, and not an array. */
export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** The path of a field of an object: `key` at the top, `path.key` below it. */
export function fieldPath(path: string, key: string): string {
  return path === "" ? key : `${path}.${key}`;
}

/** The path of an item of a list: `path[index]`. */
export function itemPath(path: string, index: number): string {
  return `${path}[${index}]`;
}

/** Limits and a default for an integer field. */
export interface IntOptions {
  /** The least allowed value. Default: the smallest safe integer. */
  readonly min?: number;
  /** The greatest allowed value. Default: the greatest safe integer. */
  readonly max?: number;
  /** The value of a field that is left out. Without one, a missing field is a problem. */
  readonly default?: number;
}

/** Limits and a default for a number field. */
export interface NumberOptions {
  /** The least allowed value. Default: no limit. */
  readonly min?: number;
  /** The greatest allowed value. Default: no limit. */
  readonly max?: number;
  /** The value of a field that is left out. Without one, a missing field is a problem. */
  readonly default?: number;
}

/**
 * The fields of one JSON object, read with checks. The class assigns its fields in the constructor, not through
 * parameter properties: the velvet-js bin runs the sources with Node's type stripping, which refuses those.
 */
export class Fields {
  /** The JSON object. */
  readonly source: Record<string, unknown>;
  /** The object's path in the document, "" for the top. */
  readonly path: string;
  /** Where the problems go. */
  private readonly problems: Problems;

  /**
   * @param problems - where the problems go.
   * @param source - the JSON object.
   * @param path - the object's path in the document, "" for the top.
   */
  constructor(problems: Problems, source: Record<string, unknown>, path: string) {
    this.problems = problems;
    this.source = source;
    this.path = path;
  }

  /** The path of one of this object's fields. */
  at(key: string): string {
    return fieldPath(this.path, key);
  }

  /** True when the object has the field, even with a null or an empty value. */
  has(key: string): boolean {
    return this.source[key] !== undefined;
  }

  /** Records a problem at one of this object's fields. */
  fail(code: TilesErrorCode, message: string, key: string): void {
    this.problems.add(code, message, this.at(key));
  }

  /** A string field. The default is used when the field is left out. */
  string(key: string, fallback?: string): string {
    const value = this.source[key];
    if (value === undefined) {
      if (fallback === undefined) this.fail("structure", `"${key}" is required`, key);
      return fallback ?? "";
    }
    if (typeof value !== "string") {
      this.fail("structure", `"${key}" must be a string`, key);
      return fallback ?? "";
    }
    return value;
  }

  /** A whole-number field, from `min` to `max`. */
  int(key: string, options: IntOptions = {}): number {
    const min = options.min ?? Number.MIN_SAFE_INTEGER;
    const max = options.max ?? Number.MAX_SAFE_INTEGER;
    const fallback = options.default ?? 0;
    const value = this.source[key];
    if (value === undefined) {
      if (options.default === undefined) this.fail("structure", `"${key}" is required`, key);
      return fallback;
    }
    if (typeof value !== "number") {
      this.fail("structure", `"${key}" must be a whole number`, key);
      return fallback;
    }
    if (!Number.isInteger(value)) {
      this.fail("value", `"${key}" must be a whole number, got ${value}`, key);
      return fallback;
    }
    if (value < min || value > max) {
      this.fail("value", `"${key}" must be from ${min} to ${max}, got ${value}`, key);
      return fallback;
    }
    return value;
  }

  /** A number field: finite, and from `min` to `max`. */
  number(key: string, options: NumberOptions = {}): number {
    const min = options.min ?? -Infinity;
    const max = options.max ?? Infinity;
    const fallback = options.default ?? 0;
    const value = this.source[key];
    if (value === undefined) {
      if (options.default === undefined) this.fail("structure", `"${key}" is required`, key);
      return fallback;
    }
    if (typeof value !== "number" || !Number.isFinite(value)) {
      this.fail("structure", `"${key}" must be a finite number`, key);
      return fallback;
    }
    if (value < min || value > max) {
      this.fail("value", `"${key}" must be from ${min} to ${max}, got ${value}`, key);
      return fallback;
    }
    return value;
  }

  /** A text field that must be one of `allowed`. The fallback is used when the field is left out. */
  choice<T extends string>(key: string, allowed: readonly T[], fallback: T): T {
    const value = this.source[key];
    if (value === undefined) return fallback;
    if (typeof value !== "string") {
      this.fail("structure", `"${key}" must be a string`, key);
      return fallback;
    }
    if (!(allowed as readonly string[]).includes(value)) {
      this.fail("value", `"${key}" must be one of ${allowed.join(", ")}, got "${value}"`, key);
      return fallback;
    }
    return value as T;
  }

  /** A true-or-false field. */
  bool(key: string, fallback?: boolean): boolean {
    const value = this.source[key];
    if (value === undefined) {
      if (fallback === undefined) this.fail("structure", `"${key}" is required`, key);
      return fallback ?? false;
    }
    if (typeof value !== "boolean") {
      this.fail("structure", `"${key}" must be true or false`, key);
      return fallback ?? false;
    }
    return value;
  }

  /** An object field, read with its own checks. Undefined when it is left out or is not an object. */
  object(key: string, required = false): Fields | undefined {
    const value = this.source[key];
    if (value === undefined) {
      if (required) this.fail("structure", `"${key}" is required`, key);
      return undefined;
    }
    if (!isRecord(value)) {
      this.fail("structure", `"${key}" must be an object`, key);
      return undefined;
    }
    return new Fields(this.problems, value, this.at(key));
  }

  /** A list field. An empty list is used when it is left out. Each item is raw JSON. */
  array(key: string, required = false): readonly unknown[] {
    const value = this.source[key];
    if (value === undefined) {
      if (required) this.fail("structure", `"${key}" is required`, key);
      return [];
    }
    if (!Array.isArray(value)) {
      this.fail("structure", `"${key}" must be an array`, key);
      return [];
    }
    return value;
  }

  /** The raw JSON value of a field. Undefined when it is left out. */
  raw(key: string): unknown {
    return this.source[key];
  }

  /** The problems of the read, for readers of nested values that record into the same list. */
  get recorder(): Problems {
    return this.problems;
  }
}

/** Reads a list of objects, one `read` call per item, and skips the items that are not objects. */
export function readItems<T>(
  problems: Problems,
  list: readonly unknown[],
  path: string,
  what: string,
  read: (fields: Fields, index: number) => T | undefined,
): T[] {
  const result: T[] = [];
  list.forEach((item, index) => {
    const itemAt = itemPath(path, index);
    if (!isRecord(item)) {
      problems.add("structure", `${what} must be an object`, itemAt);
      return;
    }
    const value = read(new Fields(problems, item, itemAt), index);
    if (value !== undefined) result.push(value);
  });
  return result;
}
