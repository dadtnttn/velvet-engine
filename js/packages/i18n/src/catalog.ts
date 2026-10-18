// Reading one catalog file: its header, its meta block and its entries. A file with a file-level
// error contributes nothing. An entry with an error is dropped, and the other entries are kept.

import {
  LOCALE_FORMAT,
  LOCALE_VERSION,
  PLURAL_CATEGORIES,
  makeIssue,
  type Direction,
  type EntryValue,
  type LocaleIssue,
  type LocaleIssueCode,
  type LocaleIssueSeverity,
  type LocaleMeta,
  type PluralCategory,
  type PluralForms,
} from "./format";
import { isLanguageTag } from "./tags";
import { parseTemplate, type Template } from "./template";

/** One entry after reading: its value as the file writes it, and its templates by category. */
export interface ParsedEntry {
  /** The value as the file writes it. */
  readonly value: EntryValue;
  /** The template of each form. A plain string has only the `other` form. */
  readonly forms: ReadonlyMap<PluralCategory, Template>;
  /** The names of the holes of all the forms together. */
  readonly holes: ReadonlySet<string>;
}

/** The result of reading one catalog file. */
export interface ParsedCatalog {
  /** The file name, as it was given. */
  readonly name: string;
  /** False when a file-level error means the file contributes nothing. */
  readonly usable: boolean;
  /** The language the file names, when it names a valid tag. */
  readonly language: string | undefined;
  /** The file's meta block, with the fields that have a bad value left out. */
  readonly meta: LocaleMeta;
  /** The usable entries, by key. */
  readonly entries: ReadonlyMap<string, ParsedEntry>;
  /** Every problem found, errors and warnings, in the order they were found. */
  readonly issues: readonly LocaleIssue[];
}

/** The fields the format defines at the top level of a catalog file. */
const TOP_LEVEL_FIELDS = new Set(["format", "version", "language", "note", "meta", "entries"]);

/** The fields the format defines in a `meta` block. */
const META_FIELDS = new Set(["name", "script", "direction", "font", "shaping"]);

const SCRIPT = /^[A-Z][a-z]{3}$/u;
const LETTER = /\p{L}/u;

/** A compiled entry, or the reason it is dropped. */
type Compiled =
  | { readonly ok: true; readonly entry: ParsedEntry }
  | {
      readonly ok: false;
      readonly severity: LocaleIssueSeverity;
      readonly code: LocaleIssueCode;
      readonly message: string;
    };

/**
 * Reads the text of one catalog file. It never throws: every problem comes back as an issue.
 *
 * @param text - the file's JSON text.
 * @param name - the file name, for the issues. Defaults to `catalog.json`.
 * @returns the usable entries and every issue. A file-level error leaves `usable` false.
 */
export function parseCatalog(text: string, name = "catalog.json"): ParsedCatalog {
  const unusable = (code: LocaleIssueCode, message: string): ParsedCatalog => ({
    name,
    usable: false,
    language: undefined,
    meta: {},
    entries: new Map(),
    issues: [makeIssue("error", code, message, { file: name })],
  });

  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return unusable("json", `${name} is not valid JSON`);
  }
  if (!isRecord(parsed)) {
    return unusable("structure", `${name}: the top level must be a JSON object`);
  }
  if (parsed.format !== LOCALE_FORMAT) {
    return unusable("format", `${name}: format must be "${LOCALE_FORMAT}"`);
  }
  if (parsed.version !== LOCALE_VERSION) {
    return unusable("version", `${name}: version must be ${LOCALE_VERSION}`);
  }
  const language = parsed.language;
  if (typeof language !== "string" || !isLanguageTag(language)) {
    return unusable("language", `${name}: language must be a tag in canonical case, such as "pt-BR"`);
  }
  const entries = parsed.entries;
  if (!isRecord(entries)) {
    return unusable("structure", `${name}: entries must be a JSON object`);
  }

  const issues: LocaleIssue[] = [];
  for (const field of Object.keys(parsed)) {
    if (!TOP_LEVEL_FIELDS.has(field)) {
      issues.push(
        makeIssue("warning", "unknown-field", `${name}: "${field}" is not a field of the format; it is ignored`, { file: name }),
      );
    }
  }
  const meta = parsed.meta === undefined ? {} : readMeta(parsed.meta, name, issues);

  const compiled = new Map<string, ParsedEntry>();
  for (const [key, value] of Object.entries(entries)) {
    const result = compileEntry(key, value);
    if (result.ok) {
      compiled.set(key, result.entry);
    } else {
      issues.push(makeIssue(result.severity, result.code, result.message, { file: name, key }));
    }
  }
  return { name, usable: true, language, meta, entries: compiled, issues };
}

/** Reads a `meta` block, leaving out the fields that have a bad value. */
function readMeta(raw: unknown, file: string, issues: LocaleIssue[]): LocaleMeta {
  if (!isRecord(raw)) {
    issues.push(makeIssue("error", "meta", `${file}: meta must be a JSON object`, { file }));
    return {};
  }
  const meta: { name?: string; font?: string; script?: string; direction?: Direction; shaping?: boolean } = {};
  const bad = (field: string, why: string): void => {
    issues.push(makeIssue("error", "meta", `${file}: meta.${field} ${why}`, { file }));
  };
  for (const [field, value] of Object.entries(raw)) {
    if (!META_FIELDS.has(field)) {
      issues.push(
        makeIssue("warning", "unknown-field", `${file}: meta.${field} is not a field of the format; it is ignored`, { file }),
      );
      continue;
    }
    switch (field) {
      case "name":
      case "font":
        if (typeof value === "string") meta[field] = value;
        else bad(field, "must be a string");
        break;
      case "script":
        if (typeof value === "string" && SCRIPT.test(value)) meta.script = value;
        else bad(field, 'must be an ISO 15924 code with a capital first, such as "Latn"');
        break;
      case "direction":
        if (value === "ltr" || value === "rtl") meta.direction = value;
        else bad(field, 'must be "ltr" or "rtl"');
        break;
      case "shaping":
        if (typeof value === "boolean") meta.shaping = value;
        else bad(field, "must be true or false");
        break;
    }
  }
  return meta;
}

/** Compiles one entry: a plain string, or a plural object. */
function compileEntry(key: string, value: unknown): Compiled {
  if (typeof value === "string") {
    return compileForms(key, new Map<PluralCategory, string>([["other", value]]), value);
  }
  if (!isRecord(value)) {
    return rejected("error", "value", `"${key}": the value must be a string or a plural object`);
  }
  const forms = new Map<PluralCategory, string>();
  for (const [category, form] of Object.entries(value)) {
    if (!isPluralCategory(category)) {
      return rejected("error", "plural", `"${key}": "${category}" is not a plural category`);
    }
    if (typeof form !== "string") {
      return rejected("error", "plural", `"${key}": the ${category} form must be a string`);
    }
    forms.set(category, form);
  }
  if (!forms.has("other")) {
    return rejected("error", "plural", `"${key}": a plural object needs an "other" form`);
  }
  return compileForms(key, forms, toPluralForms(forms));
}

/** Parses the template of each form, and checks that no form is an empty text for a key with letters. */
function compileForms(key: string, texts: ReadonlyMap<PluralCategory, string>, value: EntryValue): Compiled {
  const forms = new Map<PluralCategory, Template>();
  const holes = new Set<string>();
  for (const [category, text] of texts) {
    if (text === "" && LETTER.test(key)) {
      return rejected("warning", "empty", `"${key}": the ${category} text is empty, so the key counts as untranslated`);
    }
    const parsed = parseTemplate(text);
    if (!parsed.ok) {
      return rejected("error", "template", `"${key}": ${parsed.message}`);
    }
    forms.set(category, parsed.template);
    for (const hole of parsed.template.holes) {
      holes.add(hole);
    }
  }
  return { ok: true, entry: { value, forms, holes } };
}

/** The plural forms as a plain object, with `other` always present. */
function toPluralForms(forms: ReadonlyMap<PluralCategory, string>): PluralForms {
  const result: { -readonly [Category in keyof PluralForms]: PluralForms[Category] } = {
    other: forms.get("other") ?? "",
  };
  for (const category of PLURAL_CATEGORIES) {
    if (category !== "other") {
      const form = forms.get(category);
      if (form !== undefined) {
        result[category] = form;
      }
    }
  }
  return result;
}

function rejected(severity: LocaleIssueSeverity, code: LocaleIssueCode, message: string): Compiled {
  return { ok: false, severity, code, message };
}

function isPluralCategory(category: string): category is PluralCategory {
  return (PLURAL_CATEGORIES as readonly string[]).includes(category);
}

/** True for an object that is neither null nor an array. */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
