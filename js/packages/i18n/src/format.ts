// The shared vocabulary of velvet-locale: the format tag and version, the types of a catalog's
// values, the issues that a reader reports, and the error that a load, a lookup or a fetch can
// throw. The rules themselves are in docs/formats/LOCALE.md.

/** The format tag that every catalog file carries. */
export const LOCALE_FORMAT = "velvet-locale";

/** The format version this package reads. Any other version is refused. */
export const LOCALE_VERSION = 1;

/** The CLDR plural categories, in the order the format lists them. */
export const PLURAL_CATEGORIES = ["zero", "one", "two", "few", "many", "other"] as const;

/** One CLDR plural category. */
export type PluralCategory = (typeof PLURAL_CATEGORIES)[number];

/** The main direction of a language's text. */
export type Direction = "ltr" | "rtl";

/** Facts about a language, from the `meta` blocks of its catalog files. Absent fields are undefined. */
export interface LocaleMeta {
  /** The language's name in the language itself, for a language picker. */
  readonly name?: string;
  /** The writing system as an ISO 15924 code, such as `Latn` or `Hebr`. */
  readonly script?: string;
  /** The main direction of the text. */
  readonly direction?: Direction;
  /** A hint for the platform's font choice. The engine decides what it means. */
  readonly font?: string;
  /** True when the text needs complex shaping: joined or reordered letters, combining marks. */
  readonly shaping?: boolean;
  /**
   * The keys that this language keeps as the source text on purpose, such as `Esc` or a brand name. The tooling does
   * not report them as untranslated. The lists of all the files of the language are joined.
   */
  readonly sameAsSource?: readonly string[];
}

/** The forms of a plural entry, keyed by CLDR category. `other` is always present. */
export interface PluralForms {
  readonly zero?: string;
  readonly one?: string;
  readonly two?: string;
  readonly few?: string;
  readonly many?: string;
  readonly other: string;
}

/** The value of one entry as its catalog file writes it: a template, or plural forms. */
export type EntryValue = string | PluralForms;

/** A value filled into a hole: text or a number. */
export type LocaleValue = string | number;

/**
 * The arguments of a lookup. A list fills the positional holes (`{0}` is the first item). An
 * object fills the named holes (`{gold}`), and a positional hole whose name is one of its keys.
 */
export type LocaleArgs = readonly LocaleValue[] | Readonly<Record<string, LocaleValue>>;

/** How serious an issue is. An error means the data is not used; a warning means it is used or harmless. */
export type LocaleIssueSeverity = "error" | "warning";

/**
 * Why a reader reported an issue. The codes are the same in every reader; the rules behind them
 * are in docs/formats/LOCALE.md. `duplicate-key` is reported only by a reader that can see
 * duplicate keys, which JSON.parse cannot.
 */
export type LocaleIssueCode =
  | "json"
  | "structure"
  | "format"
  | "version"
  | "language"
  | "language-mismatch"
  | "meta"
  | "unknown-field"
  | "value"
  | "plural"
  | "template"
  | "empty"
  | "holes"
  | "duplicate-key"
  | "missing-key"
  | "unknown-key"
  | "missing-argument";

/** A problem found while reading, loading or checking catalogs. */
export interface LocaleIssue {
  /** Whether the data was not used (error) or is used but should be fixed (warning). */
  readonly severity: LocaleIssueSeverity;
  /** The machine-readable reason. */
  readonly code: LocaleIssueCode;
  /** A sentence in English for logs and developers. Its wording is not part of the format. */
  readonly message: string;
  /** The language the issue belongs to, when it is known. */
  readonly language?: string;
  /** The catalog file the issue belongs to, when it belongs to one file. */
  readonly file?: string;
  /** The key the issue belongs to, when it belongs to one entry. */
  readonly key?: string;
}

/** Why a load, a lookup or a fetch failed. */
export type LocaleErrorCode =
  | "language"
  | "unknown-language"
  | "duplicate-language"
  | "source-missing"
  | "duplicate-file"
  | "fetch";

/** An error thrown by this package. Code that handles it branches on `code`. */
export class LocaleError extends Error {
  /** Why the operation failed. */
  readonly code: LocaleErrorCode;

  /**
   * @param code - the failure reason.
   * @param message - a sentence for logs and developers.
   * @param options - an optional `cause`.
   */
  constructor(code: LocaleErrorCode, message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "LocaleError";
    this.code = code;
  }
}

/** Where an issue belongs. Each part is optional. */
export interface IssuePlace {
  readonly language?: string;
  readonly file?: string;
  readonly key?: string;
}

/**
 * Builds an issue.
 * @param severity - error or warning.
 * @param code - the reason.
 * @param message - a sentence for logs.
 * @param place - the language, file and key the issue belongs to, when known.
 */
export function makeIssue(
  severity: LocaleIssueSeverity,
  code: LocaleIssueCode,
  message: string,
  place: IssuePlace = {},
): LocaleIssue {
  return { severity, code, message, ...place };
}
