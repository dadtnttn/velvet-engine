// Reads velvet-game.json the way the Velvet Desktop host does (every rule of crates/velvet-desktop/src/
// manifest.rs), and reads the optional `data` hints that velvet-js uses to find a game's files.

import { isLanguageTag } from "@velvet/i18n";
import { makeIssue, WHOLE_FILE, type Issue } from "../issues";
import {
  DEFAULT_ENTRY,
  DEFAULT_VERSION,
  DEFAULT_WINDOW,
  entryProblem,
  gameIdProblem,
  MANIFEST_FIELDS,
  MANIFEST_FILE,
  MAX_WINDOW_SIDE,
  titleProblem,
  versionProblem,
  WINDOW_FIELDS,
} from "./rules";

/**
 * The `data` hints of velvet-game.json. Every path is relative to the game folder, with forward slashes.
 * The hints are optional; an empty value means "look where the conventions say".
 *
 * - `roots`: the folders to search for data. Default: the whole game folder, except the folders that
 *   are never data (node_modules, dist, release, and hidden folders).
 * - `guion`: folders or files of guiones (Markdown dialogue). Default: any folder named `guion` or `guiones`.
 * - `ignore`: folders or files to leave out of the search.
 * - `source`: the source language of the locale catalogs. Default: `es` when the game has it, else
 *   the first language folder in name order.
 */
export interface DataHints {
  readonly roots: readonly string[];
  readonly guion: readonly string[];
  readonly ignore: readonly string[];
  readonly source: string | undefined;
}

/** The hints of a game that names none: every folder is searched, and the source is found by convention. */
export const NO_HINTS: DataHints = { roots: [], guion: [], ignore: [], source: undefined };

/** The fields of a manifest that velvet-js uses. Missing fields have their defaults. */
export interface ManifestInfo {
  /** The game id, as written. Empty when the manifest does not name one. */
  readonly id: string;
  /** The title, as written. Empty when the manifest does not name one. */
  readonly title: string;
  readonly version: string;
  readonly entry: string;
  readonly hints: DataHints;
}

/** The outcome of reading a manifest: what it says, and every problem found. */
export interface ManifestReading {
  /** The manifest's fields, or undefined when the text is not a JSON object. */
  readonly info: ManifestInfo | undefined;
  readonly issues: Issue[];
}

/** The hint names that velvet-js reads inside `data`. */
const HINT_FIELDS: readonly string[] = ["roots", "guion", "ignore", "source"];

/** True for a JSON object that is neither null nor an array. */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * True when a hint path is a plain relative path: forward slashes, no drive letter, no `..` segment.
 * @param path - the path as written in the hint.
 */
function isHintPath(path: string): boolean {
  if (path.trim() === "" || path.includes("\\") || path.startsWith("/") || path.includes(":")) return false;
  return path.split("/").every((segment) => segment !== "..");
}

/** Removes a leading `./` and a trailing `/` from a hint path, so that paths compare as text. */
function normalizeHintPath(path: string): string {
  const trimmed = path.replace(/^\.\//u, "").replace(/\/+$/u, "");
  return trimmed === "" ? "." : trimmed;
}

/**
 * Reads the text of velvet-game.json.
 * @param text - the file's text. A leading byte order mark is ignored, as the host ignores it.
 * @returns the manifest's fields and every problem found, each reported against velvet-game.json.
 */
export function readManifest(text: string): ManifestReading {
  const issues: Issue[] = [];
  const problem = (path: string, code: string, message: string): void => {
    issues.push(makeIssue("error", MANIFEST_FILE, path, code, message));
  };

  let raw: unknown;
  try {
    raw = JSON.parse(text.startsWith("\uFEFF") ? text.slice(1) : text);
  } catch (error) {
    const reason = error instanceof Error ? error.message : String(error);
    problem(WHOLE_FILE, "json", `the manifest is not valid JSON: ${reason}`);
    return { info: undefined, issues };
  }
  if (!isRecord(raw)) {
    problem(WHOLE_FILE, "structure", "the manifest must be a JSON object");
    return { info: undefined, issues };
  }

  for (const key of Object.keys(raw)) {
    if (MANIFEST_FIELDS.includes(key) || key === "data") continue;
    problem(key, "unknown-field", `"${key}" is not a manifest field: the Velvet Desktop host refuses unknown fields`);
  }

  const id = typeof raw["id"] === "string" ? raw["id"] : undefined;
  if (id === undefined) problem("id", "structure", "id is required: a string");
  else {
    const reason = gameIdProblem(id);
    if (reason !== undefined) problem("id", "id", reason);
  }

  const title = typeof raw["title"] === "string" ? raw["title"] : undefined;
  if (title === undefined) problem("title", "structure", "title is required: a string");
  else {
    const reason = titleProblem(title);
    if (reason !== undefined) problem("title", "title", reason);
  }

  const version = readString(raw, "version", DEFAULT_VERSION, problem, versionProblem);
  const entry = readString(raw, "entry", DEFAULT_ENTRY, problem, entryProblem);
  readWindow(raw["window"], problem);
  const hints = readHints(raw["data"], problem);

  return {
    info: {
      id: id ?? "",
      title: title ?? "",
      version,
      entry,
      hints,
    },
    issues,
  };
}

/**
 * Reads a string field with a default, and checks it.
 * @param raw - the manifest object.
 * @param key - the field name.
 * @param fallback - the value when the field is missing.
 * @param problem - reports a problem against the field.
 * @param check - returns why a value is not valid, or undefined.
 */
function readString(
  raw: Record<string, unknown>,
  key: string,
  fallback: string,
  problem: (path: string, code: string, message: string) => void,
  check: (value: string) => string | undefined,
): string {
  const value = raw[key];
  if (value === undefined) return fallback;
  if (typeof value !== "string") {
    problem(key, "structure", `${key} must be a string`);
    return fallback;
  }
  const reason = check(value);
  if (reason !== undefined) problem(key, key, reason);
  return value;
}

/**
 * Checks the `window` object: only the known fields, whole sizes from 1 to 16384, minimum sizes that
 * are not larger than the sizes, and true or false for the two flags.
 * @param value - the `window` field as written, or undefined.
 * @param problem - reports a problem against a path inside the manifest.
 */
function readWindow(value: unknown, problem: (path: string, code: string, message: string) => void): void {
  if (value === undefined) return;
  if (!isRecord(value)) {
    problem("window", "structure", "window must be an object");
    return;
  }
  for (const key of Object.keys(value)) {
    if (!WINDOW_FIELDS.includes(key)) {
      problem(`window.${key}`, "unknown-field", `"${key}" is not a window field: the host refuses unknown fields`);
    }
  }
  const sides: Record<string, number> = {};
  for (const key of ["width", "height", "min_width", "min_height"] as const) {
    const field = value[key] ?? DEFAULT_WINDOW[key];
    if (typeof field !== "number" || !Number.isInteger(field) || field < 1 || field > MAX_WINDOW_SIDE) {
      problem(`window.${key}`, "value", `window.${key} must be a whole number from 1 to ${MAX_WINDOW_SIDE}`);
    } else {
      sides[key] = field;
    }
  }
  for (const key of ["resizable", "fullscreen"] as const) {
    const field = value[key];
    if (field !== undefined && typeof field !== "boolean") {
      problem(`window.${key}`, "structure", `window.${key} must be true or false`);
    }
  }
  const width = sides["width"];
  const minWidth = sides["min_width"];
  if (width !== undefined && minWidth !== undefined && minWidth > width) {
    problem("window.min_width", "value", "window.min_width must not be larger than window.width");
  }
  const height = sides["height"];
  const minHeight = sides["min_height"];
  if (height !== undefined && minHeight !== undefined && minHeight > height) {
    problem("window.min_height", "value", "window.min_height must not be larger than window.height");
  }
}

/**
 * Reads the `data` hints. A hint that is not valid is reported and left out.
 * @param value - the `data` field as written, or undefined.
 * @param problem - reports a problem against a path inside the manifest.
 */
function readHints(value: unknown, problem: (path: string, code: string, message: string) => void): DataHints {
  const hints: { roots: string[]; guion: string[]; ignore: string[]; source: string | undefined } = {
    roots: [],
    guion: [],
    ignore: [],
    source: undefined,
  };
  if (value === undefined) return hints;
  if (!isRecord(value)) {
    problem("data", "structure", "data must be an object of hints");
    return hints;
  }
  for (const key of Object.keys(value)) {
    if (!HINT_FIELDS.includes(key)) problem(`data.${key}`, "unknown-field", `"${key}" is not a data hint`);
  }
  for (const key of ["roots", "guion", "ignore"] as const) {
    const list = value[key];
    if (list === undefined) continue;
    if (!Array.isArray(list) || !list.every((item): item is string => typeof item === "string")) {
      problem(`data.${key}`, "structure", `data.${key} must be a list of paths`);
      continue;
    }
    for (const item of list) {
      if (!isHintPath(item)) {
        problem(`data.${key}`, "value", `"${item}" must be a relative path with forward slashes, inside the game folder`);
        continue;
      }
      hints[key].push(normalizeHintPath(item));
    }
  }
  const source = value["source"];
  if (source !== undefined) {
    if (typeof source === "string" && isLanguageTag(source)) hints.source = source;
    else problem("data.source", "value", "data.source must be a language tag in canonical case, such as es or pt-BR");
  }
  return hints;
}
