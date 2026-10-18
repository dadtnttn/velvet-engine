// Reads an art manifest: JSON text in, a checked ArtManifest out. Every rule of
// docs/formats/ART_MANIFEST.md is checked here, and all the problems of a document are reported
// together.

import { ArtError, type ArtErrorCode, type ArtIssue } from "./error";
import { isRecord } from "./guards";
import {
  ART_FORMAT,
  ART_KINDS,
  ART_MAX_SIDE,
  ART_VERSION,
  isArtKind,
  type ArtAnchor,
  type ArtEntry,
  type ArtKind,
  type ArtManifest,
  type ArtSize,
} from "./types";

/** An id: segments of lowercase letters, digits, `_` and `-`, joined by single dots. */
const ID_PATTERN = /^[a-z0-9_-]+(\.[a-z0-9_-]+)*$/;

/** A variant name: lowercase letters, digits, `_` and `-`. It has no dot, so file names stay readable. */
const VARIANT_PATTERN = /^[a-z0-9_-]+$/;

/** The longest id, in characters. */
const ID_LIMIT = 128;

/** The longest variant name, in characters. */
const VARIANT_LIMIT = 32;

/** Builds one issue, leaving out the path when there is none. */
function issue(code: ArtErrorCode, message: string, path?: string): ArtIssue {
  return path === undefined ? { code, message } : { code, message, path };
}

/** Builds the error for a document that fails before its entries are read. */
function singleError(code: ArtErrorCode, message: string, path?: string): ArtError {
  return new ArtError(code, message, [issue(code, message, path)]);
}

/** Builds the error for a manifest with one or more problems in its entries. */
function manifestError(issues: readonly ArtIssue[]): ArtError {
  const first = issues[0];
  const count = issues.length === 1 ? "1 problem" : `${issues.length} problems`;
  const message = `art manifest has ${count}; the first: ${first?.message ?? "unknown"}`;
  return new ArtError(first?.code ?? "structure", message, issues);
}

/** True when `value` is a whole number from 1 to ART_MAX_SIDE. */
function isSide(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 1 && value <= ART_MAX_SIDE;
}

/** True when `value` is a number from 0 to 1, both included. */
function isFraction(value: unknown): value is number {
  return typeof value === "number" && value >= 0 && value <= 1;
}

/** Reads an id. Returns undefined, with an issue, when the id is not valid. */
function readId(value: unknown, path: string, issues: ArtIssue[]): string | undefined {
  if (typeof value !== "string") {
    issues.push(issue("structure", `${path} must be a string`, path));
    return undefined;
  }
  if (value.length > ID_LIMIT || !ID_PATTERN.test(value)) {
    issues.push(
      issue("value", `${path} "${value}" is not a valid art id: lowercase segments joined by dots, at most ${ID_LIMIT} characters`, path),
    );
    return undefined;
  }
  return value;
}

/** Reads a kind. Returns undefined, with an issue, when the kind is not in ART_KINDS. */
function readKind(value: unknown, path: string, issues: ArtIssue[]): ArtKind | undefined {
  if (typeof value !== "string") {
    issues.push(issue("structure", `${path} must be a string`, path));
    return undefined;
  }
  if (!isArtKind(value)) {
    issues.push(issue("value", `${path} "${value}" is not a kind; use one of ${ART_KINDS.join(", ")}`, path));
    return undefined;
  }
  return value;
}

/** Reads a size, a pair of whole numbers. Returns undefined, with an issue, when it is not valid. */
function readSize(value: unknown, path: string, issues: ArtIssue[]): ArtSize | undefined {
  if (!Array.isArray(value)) {
    issues.push(issue("structure", `${path} must be an array of two numbers`, path));
    return undefined;
  }
  const items: readonly unknown[] = value;
  const [width, height] = items;
  if (items.length !== 2 || !isSide(width) || !isSide(height)) {
    issues.push(issue("value", `${path} must be two whole numbers from 1 to ${ART_MAX_SIDE}`, path));
    return undefined;
  }
  return [width, height];
}

/** Reads an anchor. A missing anchor is the top-left corner. */
function readAnchor(value: unknown, path: string, issues: ArtIssue[]): ArtAnchor | undefined {
  if (value === undefined) return [0, 0];
  if (!Array.isArray(value)) {
    issues.push(issue("structure", `${path} must be an array of two numbers`, path));
    return undefined;
  }
  const items: readonly unknown[] = value;
  const [x, y] = items;
  if (items.length !== 2 || !isFraction(x) || !isFraction(y)) {
    issues.push(issue("value", `${path} must be two numbers from 0 to 1`, path));
    return undefined;
  }
  return [x, y];
}

/** Reads the variant names of an entry. A missing list is empty. A name listed twice is refused. */
function readVariants(value: unknown, path: string, issues: ArtIssue[]): readonly string[] | undefined {
  if (value === undefined) return [];
  if (!Array.isArray(value)) {
    issues.push(issue("structure", `${path} must be an array of names`, path));
    return undefined;
  }
  const items: readonly unknown[] = value;
  const names: string[] = [];
  let valid = true;
  for (const [index, item] of items.entries()) {
    const itemPath = `${path}[${index}]`;
    if (typeof item !== "string") {
      issues.push(issue("structure", `${itemPath} must be a string`, itemPath));
      valid = false;
    } else if (item.length > VARIANT_LIMIT || !VARIANT_PATTERN.test(item)) {
      issues.push(issue("value", `${itemPath} "${item}" is not a valid variant name: lowercase letters, digits, "_" or "-"`, itemPath));
      valid = false;
    } else if (names.includes(item)) {
      issues.push(issue("duplicate", `${itemPath} repeats the variant "${item}"`, itemPath));
      valid = false;
    } else {
      names.push(item);
    }
  }
  return valid ? names : undefined;
}

/** Reads the notes of an entry. A missing block is an empty object. */
function readMeta(value: unknown, path: string, issues: ArtIssue[]): Readonly<Record<string, unknown>> | undefined {
  if (value === undefined) return {};
  if (!isRecord(value)) {
    issues.push(issue("structure", `${path} must be an object`, path));
    return undefined;
  }
  return value;
}

/** Reads one entry. Returns undefined when any of its fields is wrong; the issues say which. */
function parseEntry(raw: unknown, path: string, issues: ArtIssue[]): ArtEntry | undefined {
  if (!isRecord(raw)) {
    issues.push(issue("structure", `${path} must be an object`, path));
    return undefined;
  }
  const before = issues.length;
  const id = readId(raw.id, `${path}.id`, issues);
  const kind = readKind(raw.kind, `${path}.kind`, issues);
  const size = readSize(raw.size, `${path}.size`, issues);
  const anchor = readAnchor(raw.anchor, `${path}.anchor`, issues);
  const variants = readVariants(raw.variants, `${path}.variants`, issues);
  const meta = readMeta(raw.meta, `${path}.meta`, issues);
  if (
    issues.length > before ||
    id === undefined ||
    kind === undefined ||
    size === undefined ||
    anchor === undefined ||
    variants === undefined ||
    meta === undefined
  ) {
    return undefined;
  }
  return { id, kind, size, anchor, variants, meta };
}

/**
 * Finds the ids and file names that are used twice. Two entries share a file name when they have the
 * same kind and one's id or variant file name equals the other's (for example `a` with variant `b`,
 * and an entry `a.b`). Entries that are undefined, because they were refused, are skipped.
 * @param entries - the entries in file order. An undefined entry keeps its place, so the paths in the
 *   issues match the file.
 * @returns one "duplicate" issue for each id or file name that is already used.
 */
export function entryProblems(entries: readonly (ArtEntry | undefined)[]): ArtIssue[] {
  const problems: ArtIssue[] = [];
  const ids = new Set<string>();
  const owners = new Map<string, string>();
  entries.forEach((entry, index) => {
    if (entry === undefined) return;
    const path = `entries[${index}]`;
    if (ids.has(entry.id)) {
      problems.push(issue("duplicate", `id "${entry.id}" is already used by an earlier entry`, `${path}.id`));
      return;
    }
    ids.add(entry.id);
    const names = [entry.id, ...entry.variants.map((variant) => `${entry.id}.${variant}`)];
    for (const name of names) {
      const key = `${entry.kind}/${name}`;
      const owner = owners.get(key);
      if (owner === undefined) {
        owners.set(key, path);
      } else {
        problems.push(issue("duplicate", `file name "${key}" would be used twice, by ${owner} and ${path}`, path));
      }
    }
  });
  return problems;
}

/**
 * Reads an art manifest from JSON text.
 * @param json - the text of `art/manifest.json`.
 * @returns the manifest in its normal form: defaults filled in, entries in the order of the file.
 * @throws ArtError. Its `code` is the code of the first problem: "json", "format", "version",
 *   "structure", "value" or "duplicate". Its `issues` list every problem that was found.
 */
export function parseArtManifest(json: string): ArtManifest {
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch (cause) {
    throw new ArtError("json", "art manifest is not valid JSON", [issue("json", "the text is not valid JSON")], { cause });
  }
  if (!isRecord(parsed)) throw singleError("structure", "the manifest is not a JSON object");
  if (parsed.format !== ART_FORMAT) {
    throw singleError("format", `the format tag is not "${ART_FORMAT}"`, "format");
  }
  if (parsed.version !== ART_VERSION) {
    throw singleError("version", `the version is not ${ART_VERSION}`, "version");
  }
  const rawEntries: unknown = parsed.entries;
  if (!Array.isArray(rawEntries)) throw singleError("structure", "entries must be an array", "entries");

  const items: readonly unknown[] = rawEntries;
  const issues: ArtIssue[] = [];
  const parsedEntries = items.map((raw, index) => parseEntry(raw, `entries[${index}]`, issues));
  issues.push(...entryProblems(parsedEntries));
  if (issues.length > 0) throw manifestError(issues);

  const entries = parsedEntries.filter((entry): entry is ArtEntry => entry !== undefined);
  return { format: ART_FORMAT, version: ART_VERSION, entries };
}
