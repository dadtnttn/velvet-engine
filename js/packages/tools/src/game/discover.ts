// Finds a game's data files: it reads velvet-game.json, walks the game folder, and sorts each file by
// its format. The rules are written out in packages/tools/README.md, "Where the data is".

import type { Dirent } from "node:fs";
import { readdir, readFile, stat } from "node:fs/promises";
import { join, resolve } from "node:path";
import { makeIssue, WHOLE_FILE, type Issue } from "../issues";
import { NO_HINTS, readManifest, type DataHints, type ManifestInfo } from "./manifest";
import { MANIFEST_FILE } from "./rules";

/** The kinds of data document that velvet-js reads. */
export type DocumentKind =
  | "stacks-catalog"
  | "stacks-case"
  | "character-schema"
  | "character-sheet"
  | "art-manifest"
  | "artprep-profiles"
  | "artprep-style"
  | "artprep-identity"
  | "artprep-shots"
  | "artprep-results"
  | "guion"
  | "locale";

/** One data document of a game: its file, its text, and for JSON documents the parsed value. */
export interface GameDocument {
  readonly kind: DocumentKind;
  /** The file, relative to the game folder, with forward slashes. */
  readonly file: string;
  /** The folder of the file, relative to the game folder, with forward slashes ("." at the top). */
  readonly folder: string;
  /** The file's name, such as `10_cards.json`. */
  readonly name: string;
  /** The text, without a byte order mark. */
  readonly text: string;
  /** The parsed JSON of a JSON document. Undefined for a guion, which is Markdown. */
  readonly json: unknown;
}

/** A file that the search visited. `i18n` scans the code files of this list. */
export interface WalkedFile {
  /** The file, relative to the game folder, with forward slashes. */
  readonly file: string;
  /** The file's absolute path on disk. */
  readonly abs: string;
  /** The file's name. */
  readonly name: string;
}

/** Everything that discovery found in a game folder. */
export interface GameDiscovery {
  /** The game folder, as given. */
  readonly folder: string;
  /** The manifest's fields, or undefined when velvet-game.json is missing or not an object. */
  readonly info: ManifestInfo | undefined;
  /** Problems with the manifest, the hints, the JSON files and the data folders. */
  readonly issues: Issue[];
  /** The data documents, sorted by file. */
  readonly documents: readonly GameDocument[];
  /** Every file the search visited, sorted by file. */
  readonly files: readonly WalkedFile[];
}

/** Folders that are never game data, wherever they are. */
const SKIPPED_FOLDERS: ReadonlySet<string> = new Set(["node_modules", "dist", "release", "coverage"]);

/** Folder names that mark data: a broken JSON file in one of them is reported, not skipped. */
const DATA_FOLDERS: ReadonlySet<string> = new Set([
  "data",
  "locales",
  "art",
  "characters",
  "artprep",
  "stacks",
  "cases",
  "guion",
  "guiones",
]);

/** Folder names whose Markdown files are guiones. */
const GUION_FOLDERS: ReadonlySet<string> = new Set(["guion", "guiones"]);

/**
 * Maps a JSON document's envelope to its kind.
 * @param format - the `format` field.
 * @param kind - the `kind` field, when the format has one.
 * @returns the document kind, "skip" for a save or a job that no check reads, or undefined when the
 *   format or kind is not one velvet-js knows.
 */
function documentKind(format: string, kind: unknown): DocumentKind | "skip" | undefined {
  switch (format) {
    case "velvet-stacks":
      if (kind === "catalog") return "stacks-catalog";
      if (kind === "case") return "stacks-case";
      return kind === "board" ? "skip" : undefined;
    case "velvet-character":
      if (kind === "schema") return "character-schema";
      if (kind === "character") return "character-sheet";
      return kind === "state" ? "skip" : undefined;
    case "velvet-art":
      return "art-manifest";
    case "velvet-artprep":
      if (kind === "profiles") return "artprep-profiles";
      if (kind === "style") return "artprep-style";
      if (kind === "identity") return "artprep-identity";
      if (kind === "shots") return "artprep-shots";
      if (kind === "results") return "artprep-results";
      return kind === "job" ? "skip" : undefined;
    case "velvet-locale":
      return "locale";
    default:
      return undefined;
  }
}

/**
 * True when a relative path is inside one of the hinted folders, or is one of the hinted files.
 * @param file - the file's relative path.
 * @param hinted - the hinted paths, relative to the game folder.
 */
function isUnder(file: string, hinted: readonly string[]): boolean {
  return hinted.some((path) => path === "." || file === path || file.startsWith(`${path}/`));
}

/**
 * Lists the files under the given start folders, leaving out the skipped folders and the ignored paths.
 * @param folder - the game folder.
 * @param starts - the start folders, relative to the game folder.
 * @param ignore - the paths to leave out.
 * @param issues - where a folder that cannot be read is reported.
 */
async function walk(
  folder: string,
  starts: readonly string[],
  ignore: readonly string[],
  issues: Issue[],
): Promise<WalkedFile[]> {
  const found: WalkedFile[] = [];
  const pending: string[] = [...starts];
  for (let index = 0; index < pending.length; index += 1) {
    const rel = pending[index] ?? ".";
    const abs = resolve(folder, rel);
    let entries: Dirent[];
    try {
      entries = await readdir(abs, { withFileTypes: true });
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error);
      issues.push(makeIssue("error", rel, WHOLE_FILE, "folder", `cannot read this folder: ${reason}`));
      continue;
    }
    entries.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
    for (const entry of entries) {
      const child = rel === "." ? entry.name : `${rel}/${entry.name}`;
      if (isUnder(child, ignore)) continue;
      if (entry.isDirectory()) {
        if (entry.name.startsWith(".") || SKIPPED_FOLDERS.has(entry.name)) continue;
        pending.push(child);
      } else if (entry.isFile()) {
        found.push({ file: child, abs: join(abs, entry.name), name: entry.name });
      }
    }
  }
  return found;
}

/**
 * True when a path is an existing folder.
 * @param path - the absolute path to test.
 */
async function isDirectory(path: string): Promise<boolean> {
  try {
    return (await stat(path)).isDirectory();
  } catch {
    return false;
  }
}

/**
 * Reads a file's text. A leading byte order mark is reported and removed, so that the readers see
 * plain UTF-8 JSON.
 * @param file - the file, relative to the game folder.
 * @param abs - the file's absolute path.
 * @param issues - where the byte order mark is reported.
 */
async function readText(file: string, abs: string, issues: Issue[]): Promise<string> {
  const text = await readFile(abs, "utf8");
  if (!text.startsWith("\uFEFF")) return text;
  issues.push(makeIssue("error", file, WHOLE_FILE, "bom", "the file starts with a byte order mark: save it as UTF-8 without one"));
  return text.slice(1);
}

/**
 * Finds the data of a game folder.
 *
 * It reads velvet-game.json and its `data` hints, walks the folder (or the hinted roots), and sorts
 * each file by content: a JSON file by its `format` and `kind`, a Markdown file inside a `guion`
 * folder as a guion. Files whose names start with `_` are tooling files and are skipped.
 *
 * @param folder - the game folder, as given on the command line.
 * @returns what was found, with every problem met on the way.
 */
export async function discoverGame(folder: string): Promise<GameDiscovery> {
  const manifestAbs = join(folder, MANIFEST_FILE);
  let manifestText: string;
  try {
    manifestText = await readFile(manifestAbs, "utf8");
  } catch {
    const issue = makeIssue(
      "error",
      MANIFEST_FILE,
      WHOLE_FILE,
      "manifest-missing",
      `${MANIFEST_FILE} is missing: a game folder needs one`,
    );
    return { folder, info: undefined, issues: [issue], documents: [], files: [] };
  }

  const manifest = readManifest(manifestText);
  const issues: Issue[] = [...manifest.issues];
  const hints: DataHints = manifest.info?.hints ?? NO_HINTS;

  const starts: string[] = [];
  if (hints.roots.length === 0) {
    starts.push(".");
  } else {
    for (const root of hints.roots) {
      if (await isDirectory(resolve(folder, root))) {
        starts.push(root);
      } else {
        issues.push(makeIssue("error", MANIFEST_FILE, "data.roots", "data-hint", `the folder "${root}" does not exist`));
      }
    }
  }
  const walked = (await walk(folder, starts, hints.ignore, issues)).sort((a, b) => compareText(a.file, b.file));

  const documents: GameDocument[] = [];
  for (const file of walked) {
    if (file.name.startsWith("_")) continue;
    const segments = file.file.split("/");
    const folderPath = segments.length > 1 ? segments.slice(0, -1).join("/") : ".";
    const folders = segments.slice(0, -1).map((segment) => segment.toLowerCase());
    const inDataFolder = folders.some((segment) => DATA_FOLDERS.has(segment));

    if (file.name.endsWith(".md")) {
      const isGuion = folders.some((segment) => GUION_FOLDERS.has(segment)) || isUnder(file.file, hints.guion);
      if (!isGuion) continue;
      const text = await readText(file.file, file.abs, issues);
      documents.push({ kind: "guion", file: file.file, folder: folderPath, name: file.name, text, json: undefined });
      continue;
    }
    if (!file.name.endsWith(".json")) continue;

    const text = await readText(file.file, file.abs, issues);
    let json: unknown;
    try {
      json = JSON.parse(text);
    } catch (error) {
      if (inDataFolder) {
        const reason = error instanceof Error ? error.message : String(error);
        issues.push(makeIssue("error", file.file, WHOLE_FILE, "json", `not valid JSON: ${reason}`));
      }
      continue;
    }
    if (typeof json !== "object" || json === null || Array.isArray(json)) continue;
    const format = Reflect.get(json, "format");
    if (typeof format !== "string" || !format.startsWith("velvet-")) continue;

    const kind = documentKind(format, Reflect.get(json, "kind"));
    if (kind === "skip") continue;
    if (kind === undefined) {
      issues.push(
        makeIssue("warning", file.file, WHOLE_FILE, "unknown-document", `${format} documents of this kind are not checked`),
      );
      continue;
    }
    documents.push({ kind, file: file.file, folder: folderPath, name: file.name, text, json });
  }

  return {
    folder,
    info: manifest.info,
    issues,
    documents,
    files: walked,
  };
}

/**
 * Compares two texts by their code points, the order the tools print and sort in.
 * @param a - the first text.
 * @param b - the second text.
 */
export function compareText(a: string, b: string): number {
  if (a === b) return 0;
  return a < b ? -1 : 1;
}
