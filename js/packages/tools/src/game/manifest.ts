// Reads velvet-game.json the way the Velvet Desktop host does (every rule of crates/velvet-desktop/src/
// manifest.rs), reads the `release` object the way the release step does (crates/velvet-cli/src/
// js_build_cmd.rs), and reads the optional `data` hints that velvet-js uses to find a game's files. The
// files that the manifest names (the icon and the included files) are checked against the game folder.

import { readFile, stat } from "node:fs/promises";
import { join } from "node:path";
import { isLanguageTag } from "@velvet/i18n";
import { makeIssue, WHOLE_FILE, type Issue } from "../issues";
import {
  BUILD_INFO_FILE,
  CHECKSUM_FILE,
  DEFAULT_ENTRY,
  DEFAULT_VERSION,
  DEFAULT_WINDOW,
  entryProblem,
  gameIdProblem,
  ICON_WARN_PIXELS,
  MANIFEST_FIELDS,
  MANIFEST_FILE,
  MAX_WINDOW_SIDE,
  patternProblem,
  RELEASE_FIELDS,
  RELEASE_GAME_FOLDER,
  relativePathProblem,
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

/**
 * The `release` object of velvet-game.json: what a release folder leaves out of the web files, what it adds
 * from the game folder, and the archive it is also written as.
 */
export interface ReleaseHints {
  /** Patterns of files of the built web folder that the release leaves out. */
  readonly exclude: readonly string[];
  /** Files of the game folder that the release copies to its root. */
  readonly include: readonly string[];
  /** `none` (the folder only), or `zip` (also a zip archive beside the folder). */
  readonly archive: "none" | "zip";
}

/** The release hints of a manifest that names none. */
export const NO_RELEASE: ReleaseHints = { exclude: [], include: [], archive: "none" };

/** The fields of a manifest that velvet-js uses. Missing fields have their defaults. */
export interface ManifestInfo {
  /** The game id, as written. Empty when the manifest does not name one. */
  readonly id: string;
  /** The title, as written. Empty when the manifest does not name one. */
  readonly title: string;
  readonly version: string;
  readonly entry: string;
  /** The icon's path, relative to the game folder, or undefined when the manifest names none. */
  readonly icon: string | undefined;
  readonly release: ReleaseHints;
  readonly hints: DataHints;
}

/** The outcome of reading a manifest: what it says, and every problem found. */
export interface ManifestReading {
  /** The manifest's fields, or undefined when the text is not a JSON object. */
  readonly info: ManifestInfo | undefined;
  readonly issues: Issue[];
}

/** Reports a problem against a path inside the manifest. */
type ProblemReporter = (path: string, code: string, message: string) => void;

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
    raw = JSON.parse(text.startsWith("﻿") ? text.slice(1) : text);
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
  const icon = readPath(raw, "icon", "icon.png", problem);
  const release = readRelease(raw["release"], problem);
  for (const path of release.include) {
    const reason = reservedNameProblem(path, id ?? "");
    if (reason !== undefined) problem("release.include", "value", reason);
  }

  return {
    info: {
      id: id ?? "",
      title: title ?? "",
      version,
      entry,
      icon,
      release,
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
  problem: ProblemReporter,
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
 * Reads an optional path field, such as `icon`. A missing or null field is undefined. A path that is not
 * valid is reported and left out.
 * @param raw - the manifest object.
 * @param key - the field name.
 * @param example - a valid path, named in the message.
 * @param problem - reports a problem against the field.
 */
function readPath(raw: Record<string, unknown>, key: string, example: string, problem: ProblemReporter): string | undefined {
  const value = raw[key];
  if (value === undefined || value === null) return undefined;
  if (typeof value !== "string") {
    problem(key, "structure", `${key} must be a string`);
    return undefined;
  }
  const reason = relativePathProblem(value, example);
  if (reason !== undefined) {
    problem(key, key, reason);
    return undefined;
  }
  return value;
}

/**
 * Reads the `release` object: only the known fields, and patterns and paths that are valid. The archive is
 * `none` (the default) or `zip`. A missing or null object means the defaults.
 * @param value - the `release` field as written.
 * @param problem - reports a problem against a path inside the manifest.
 */
function readRelease(value: unknown, problem: ProblemReporter): ReleaseHints {
  if (value === undefined || value === null) return NO_RELEASE;
  if (!isRecord(value)) {
    problem("release", "structure", "release must be an object");
    return NO_RELEASE;
  }
  for (const key of Object.keys(value)) {
    if (!RELEASE_FIELDS.includes(key)) {
      problem(`release.${key}`, "unknown-field", `"${key}" is not a release field: the release refuses unknown fields`);
    }
  }
  const exclude = readList(value, "exclude", patternProblem, problem);
  const include = readList(value, "include", (path) => relativePathProblem(path, "LICENSE"), problem);
  let archive: "none" | "zip" = "none";
  const kind = value["archive"];
  if (kind !== undefined) {
    if (kind === "none" || kind === "zip") archive = kind;
    else problem("release.archive", "value", 'release.archive must be "none" or "zip"');
  }
  return { exclude, include, archive };
}

/**
 * Reads a list of strings in the `release` object. An item that the check refuses is reported and left out.
 * @param value - the `release` object.
 * @param key - the list's name.
 * @param check - returns why an item is not valid, or undefined.
 * @param problem - reports a problem against a path inside the manifest.
 */
function readList(
  value: Record<string, unknown>,
  key: "exclude" | "include",
  check: (item: string) => string | undefined,
  problem: ProblemReporter,
): string[] {
  const list = value[key];
  if (list === undefined) return [];
  if (!Array.isArray(list) || !list.every((item): item is string => typeof item === "string")) {
    problem(`release.${key}`, "structure", `release.${key} must be a list of strings`);
    return [];
  }
  const kept: string[] = [];
  for (const item of list) {
    const reason = check(item);
    if (reason === undefined) kept.push(item);
    else problem(`release.${key}`, "value", reason);
  }
  return kept;
}

/**
 * Why a file in the top folder of a release cannot be an included file: the game folder, the two build
 * files and the host program (named after the id, with or without `.exe`) are written by the release itself.
 * @param path - the included path.
 * @param id - the game id, or an empty text when the manifest names none.
 */
function reservedNameProblem(path: string, id: string): string | undefined {
  const top = path.split("/")[0] ?? path;
  const reserved = top === RELEASE_GAME_FOLDER || [BUILD_INFO_FILE, CHECKSUM_FILE, id, `${id}.exe`].includes(path);
  return reserved ? `"${path}" is a name that the release writes itself` : undefined;
}

/**
 * Checks the `window` object: only the known fields, whole sizes from 1 to 16384, minimum sizes that
 * are not larger than the sizes, and true or false for the two flags.
 * @param value - the `window` field as written, or undefined.
 * @param problem - reports a problem against a path inside the manifest.
 */
function readWindow(value: unknown, problem: ProblemReporter): void {
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
function readHints(value: unknown, problem: ProblemReporter): DataHints {
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

/**
 * The width and height of a PNG, read from its header. Undefined when the bytes do not start with the PNG
 * signature and an IHDR chunk, or when a side is zero. The picture itself is not decoded.
 * @param bytes - the file's bytes (at least the first 24).
 */
export function pngSize(bytes: Uint8Array): { readonly width: number; readonly height: number } | undefined {
  const signature = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
  if (bytes.length < 24 || !signature.every((byte, index) => bytes[index] === byte)) return undefined;
  if (String.fromCharCode(...bytes.subarray(12, 16)) !== "IHDR") return undefined;
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const width = view.getUint32(16);
  const height = view.getUint32(20);
  return width === 0 || height === 0 ? undefined : { width, height };
}

/**
 * Checks the files that the manifest names, against the game folder: the icon must be a file, a PNG, square,
 * and at least `ICON_WARN_PIXELS` on a side (a smaller one is a warning); every included file must be a file of
 * the game folder.
 * @param folder - the game folder.
 * @param info - the manifest's fields, from `readManifest`.
 * @returns the problems found, each reported against velvet-game.json.
 */
export async function manifestFileIssues(folder: string, info: ManifestInfo): Promise<Issue[]> {
  const issues: Issue[] = [];
  if (info.icon !== undefined) issues.push(...(await iconIssues(folder, info.icon)));
  for (const path of info.release.include) {
    // A name that the release writes itself is reported by `readManifest`, once.
    if (reservedNameProblem(path, info.id) !== undefined) continue;
    if (!(await isFile(join(folder, path)))) {
      issues.push(
        makeIssue("error", MANIFEST_FILE, "release.include", "missing-file", `"${path}" is not a file in the game folder`),
      );
    }
  }
  return issues;
}

/**
 * The problems of the icon: it must be a file of the game folder, a PNG, square, and at least
 * `ICON_WARN_PIXELS` on a side.
 * @param folder - the game folder.
 * @param icon - the icon's path, relative to the game folder.
 */
async function iconIssues(folder: string, icon: string): Promise<Issue[]> {
  const issue = (severity: "error" | "warning", code: string, message: string): Issue =>
    makeIssue(severity, MANIFEST_FILE, "icon", code, message);
  let bytes: Uint8Array;
  try {
    bytes = await readFile(join(folder, icon));
  } catch {
    return [issue("error", "missing-file", `the icon "${icon}" is not a file in the game folder`)];
  }
  const size = pngSize(bytes);
  if (size === undefined) return [issue("error", "not-png", `the icon "${icon}" is not a PNG image`)];
  if (size.width !== size.height) {
    return [issue("error", "not-square", `the icon "${icon}" must be square, but it is ${size.width} by ${size.height} pixels`)];
  }
  if (size.width < ICON_WARN_PIXELS) {
    return [
      issue(
        "warning",
        "small-icon",
        `the icon "${icon}" is ${size.width} pixels: use at least ${ICON_WARN_PIXELS}, so the program's icon stays sharp`,
      ),
    ];
  }
  return [];
}

/**
 * True when a path is an existing file.
 * @param path - the absolute path to test.
 */
async function isFile(path: string): Promise<boolean> {
  try {
    return (await stat(path)).isFile();
  } catch {
    return false;
  }
}
