// The constants of the format and the grammars of its ids, keys and art ids. The reader and the runtime
// share them, so every check of an id, a key or an art id is the same everywhere.

/** The format tag of every document. */
export const ACHIEVEMENTS_FORMAT = "velvet-achievements";

/** The version of the format that this package reads. */
export const ACHIEVEMENTS_VERSION = 1;

/** The key under which the progress is saved, unless the game names another. */
export const ACHIEVEMENTS_KEY = "achievements";

/** The prefix of the localStorage keys that hold the progress. The key follows it. */
export const ACHIEVEMENTS_STORAGE_PREFIX = "velvet-achievements:";

/** The group of an achievement that names none. */
export const DEFAULT_GROUP = "default";

/** How long a change waits before it is saved, in milliseconds, by default. */
export const DEFAULT_DEBOUNCE_MS = 250;

/** The longest i18n key, in UTF-16 code units. */
export const MAX_KEY_LENGTH = 200;

/** The longest art id, in characters. */
export const MAX_ART_ID_LENGTH = 128;

/** The largest number of points that an achievement may be worth. */
export const MAX_POINTS = 1000000;

/** The largest magnitude of an `order`, in either direction. */
export const MAX_ORDER = 1000000;

/** An id: a lowercase letter, then up to 63 lowercase letters, digits or underscores. */
const ID_PATTERN = /^[a-z][a-z0-9_]{0,63}$/;

/** An art id: lowercase segments of letters, digits, underscores and dashes, joined by dots. */
const ART_ID_PATTERN = /^[a-z0-9_-]+(\.[a-z0-9_-]+)*$/;

/** True for a string that is an id. */
export function isId(value: unknown): value is string {
  return typeof value === "string" && ID_PATTERN.test(value);
}

/** True for a string that is an art id. */
export function isArtId(value: unknown): value is string {
  return typeof value === "string" && value.length >= 1 && value.length <= MAX_ART_ID_LENGTH && ART_ID_PATTERN.test(value);
}

/** True for a string that can be an i18n key: 1 to 200 characters. */
export function isKeyText(value: unknown): value is string {
  return typeof value === "string" && value.length >= 1 && value.length <= MAX_KEY_LENGTH;
}
