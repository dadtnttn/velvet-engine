// The rules that a game's id and manifest must follow. They mirror the Velvet Desktop host
// (crates/velvet-desktop/src/manifest.rs) and the packaging step (crates/velvet-cli/src/js_build_cmd.rs),
// so that a game which passes these checks builds and starts.

/** The manifest file of a game folder. */
export const MANIFEST_FILE = "velvet-game.json";

/** The page that opens first when the manifest names none. */
export const DEFAULT_ENTRY = "index.html";

/** The version that a manifest gets when it names none. */
export const DEFAULT_VERSION = "0.1.0";

/** The window that a manifest gets when it names none. */
export const DEFAULT_WINDOW = {
  width: 1280,
  height: 720,
  min_width: 960,
  min_height: 540,
  resizable: true,
  fullscreen: false,
} as const;

/**
 * The fields that the host accepts at the top of a manifest. Any other field makes the host refuse it.
 * The host reads `icon`; it accepts `release` and does not read it (the release step does, and so does
 * the `data` hint, which velvet-js reads).
 */
export const MANIFEST_FIELDS: readonly string[] = ["id", "title", "version", "entry", "window", "icon", "release"];

/** The file, in the top folder of a release, that records how it was built. */
export const BUILD_INFO_FILE = "build-info.json";

/** The file, in the top folder of a release, that lists a checksum of every file. */
export const CHECKSUM_FILE = "SHA256SUMS";

/** The folder of a release that holds the game's web files. */
export const RELEASE_GAME_FOLDER = "game";

/** The fields of the `release` object. */
export const RELEASE_FIELDS: readonly string[] = ["exclude", "include", "archive"];

/** An icon whose side is shorter than this (in pixels) is reported as a warning: it will look soft. */
export const ICON_WARN_PIXELS = 256;

/** The fields that the host reads in the `window` object. */
export const WINDOW_FIELDS: readonly string[] = [
  "width",
  "height",
  "min_width",
  "min_height",
  "resizable",
  "fullscreen",
];

const ID_PATTERN = /^[a-z0-9][a-z0-9_-]{0,63}$/u;
const RESERVED_NAME = /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])$/u;
const CONTROL_CHARACTER = /\p{Cc}/u;
const VERSION_CHARACTERS = /^[A-Za-z0-9._+-]+$/u;
const ENTRY_SEGMENT = /^[A-Za-z0-9._-]+$/u;
const PATTERN_FORBIDDEN = /[\\:\p{Cc}]/u;

/** The longest title, in characters. */
export const MAX_TITLE_CHARACTERS = 120;

/** The longest version, in characters. */
export const MAX_VERSION_CHARACTERS = 64;

/** The longest path in the manifest (the entry page, the icon, the included files, the patterns), in bytes. */
export const MAX_PATH_BYTES = 256;

/** The largest window side, in logical pixels. */
export const MAX_WINDOW_SIDE = 16384;

/**
 * True when `id` is a valid game id: 1 to 64 lowercase letters, digits, `-` or `_`, starting with a
 * letter or a digit, and not a Windows device name (the id names a folder of the user's data).
 * @param id - the candidate id.
 */
export function isValidGameId(id: string): boolean {
  return gameIdProblem(id) === undefined;
}

/**
 * Why a game id is not valid, or undefined when it is.
 * @param id - the candidate id.
 */
export function gameIdProblem(id: string): string | undefined {
  if (!ID_PATTERN.test(id)) {
    return `"${id}" is not a valid id: use 1 to 64 lowercase letters, digits, "-" or "_", starting with a letter or a digit`;
  }
  if (RESERVED_NAME.test(id)) {
    return `"${id}" is a Windows device name and cannot name a game folder`;
  }
  return undefined;
}

/**
 * Why a title is not valid, or undefined when it is: not blank, at most 120 characters, no control characters.
 * @param title - the candidate title.
 */
export function titleProblem(title: string): string | undefined {
  if (title.trim() === "") return "the title must not be empty";
  if ([...title].length > MAX_TITLE_CHARACTERS) {
    return `the title must be at most ${MAX_TITLE_CHARACTERS} characters`;
  }
  if (CONTROL_CHARACTER.test(title)) return "the title must not contain control characters";
  return undefined;
}

/**
 * Why a version is not valid, or undefined when it is. A version names the release folder, so it may
 * hold only ASCII letters, digits and `.`, `-`, `_`, `+`.
 * @param version - the candidate version.
 */
export function versionProblem(version: string): string | undefined {
  if (version.trim() === "") return "the version must not be empty";
  if ([...version].length > MAX_VERSION_CHARACTERS) {
    return `the version must be at most ${MAX_VERSION_CHARACTERS} characters`;
  }
  if (!VERSION_CHARACTERS.test(version)) {
    return "the version may hold only letters, digits and \".\", \"-\", \"_\" or \"+\"";
  }
  return undefined;
}

/**
 * Why a path inside the game folder is not valid, or undefined when it is: a relative path with forward
 * slashes, made of plain ASCII segments (no empty segment, no `.` or `..`, no backslash, no drive letter).
 * The entry page, the icon and the included files all follow this rule.
 * @param path - the candidate path.
 * @param example - a valid path, named in the message.
 */
export function relativePathProblem(path: string, example: string): string | undefined {
  const plain =
    path !== "" &&
    Buffer.byteLength(path, "utf8") <= MAX_PATH_BYTES &&
    path.split("/").every((segment) => segment !== "." && segment !== ".." && ENTRY_SEGMENT.test(segment));
  return plain
    ? undefined
    : `"${path}" must be a relative path inside the game folder with forward slashes, such as ${example}`;
}

/**
 * Why an entry page is not valid, or undefined when it is. See `relativePathProblem`.
 * @param entry - the candidate entry path.
 */
export function entryProblem(entry: string): string | undefined {
  return relativePathProblem(entry, DEFAULT_ENTRY);
}

/**
 * Why a release pattern is not valid, or undefined when it is: not blank, at most 256 bytes, forward slashes
 * only, no empty, `.` or `..` folder name, no colon and no control character. A pattern may hold `*`, `?`
 * and `**` (see the release step's `glob_matches`).
 * @param pattern - the candidate pattern.
 */
export function patternProblem(pattern: string): string | undefined {
  const plain =
    pattern.trim() !== "" &&
    Buffer.byteLength(pattern, "utf8") <= MAX_PATH_BYTES &&
    !PATTERN_FORBIDDEN.test(pattern) &&
    pattern.split("/").every((segment) => segment !== "" && segment !== "." && segment !== "..");
  return plain
    ? undefined
    : `"${pattern}" must be a relative pattern with forward slashes, such as **/*.psd or dev/**`;
}
