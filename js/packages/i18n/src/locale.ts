// A locale: the catalog files of one language, read, checked and ready for lookups.
//
// A locale reads its files in name order, skips the tooling files whose names start with an
// underscore, and lets a later file override an earlier one. It knows nothing of other languages:
// the chain of fallbacks is I18n's job.

import {
  LocaleError,
  makeIssue,
  type Direction,
  type EntryValue,
  type LocaleArgs,
  type LocaleIssue,
  type LocaleMeta,
  type LocaleValue,
  type PluralCategory,
} from "./format";
import { parseCatalog, type ParsedEntry } from "./catalog";
import {
  argumentValue,
  describeHoles,
  parseTemplate,
  renderTemplate,
  sameHoles,
  type RenderedTemplate,
} from "./template";
import { isLanguageTag, textDirection } from "./tags";

/** One catalog file of a language: its name, such as `10_shop.json`, and its JSON text. */
export interface CatalogFile {
  readonly name: string;
  readonly text: string;
}

/** Options for building a locale. */
export interface LocaleOptions {
  /**
   * The locale of the source language. When it is given, the holes of each entry are checked
   * against the source text of the same key, or against the key itself when the source has no
   * entry for it (a text key). An entry whose holes differ is dropped with a `holes` error.
   */
  readonly reference?: Locale;
}

/** A lookup's outcome: the text, and the holes that had no argument (shown as `{name}`). */
export type Resolution = RenderedTemplate;

/**
 * The texts of one language, read from its catalog files.
 *
 * Games use `t` and `plural`. Tools use `has`, `keys`, `holesOf`, `issues` and `values`.
 */
export class Locale {
  /** The language tag of this locale, in canonical case. */
  readonly language: string;
  /** The merged `meta` block of the files. */
  readonly meta: LocaleMeta;
  /** The main direction of the text: from `meta`, or from the primary language subtag. */
  readonly direction: Direction;
  /** Every issue found while reading the files, errors and warnings. */
  readonly issues: readonly LocaleIssue[];
  /** The usable entries as the files write them, by key. */
  readonly values: ReadonlyMap<string, EntryValue>;

  private readonly compiled: ReadonlyMap<string, ParsedEntry>;
  private readonly rules: Intl.PluralRules;

  /**
   * @param language - the tag of the folder the files come from, in canonical case.
   * @param files - the catalog files. Their order does not matter: they are read in name order.
   * @param options - an optional reference locale for the holes check.
   * @throws LocaleError with code "language" when the tag is not in canonical case, and with code
   * "duplicate-file" when two files share a name.
   */
  constructor(language: string, files: readonly CatalogFile[], options: LocaleOptions = {}) {
    if (!isLanguageTag(language)) {
      throw new LocaleError("language", `"${language}" is not a language tag in canonical case`);
    }
    const selected = files.filter((file) => file.name.endsWith(".json") && !file.name.startsWith("_"));
    const names = new Set<string>();
    for (const file of selected) {
      if (names.has(file.name)) {
        throw new LocaleError("duplicate-file", `two catalog files are named "${file.name}" in the ${language} folder`);
      }
      names.add(file.name);
    }
    selected.sort((a, b) => compareCodePoints(a.name, b.name));

    const issues: LocaleIssue[] = [];
    const compiled = new Map<string, ParsedEntry>();
    const fileOf = new Map<string, string>();
    let meta: LocaleMeta = {};
    for (const file of selected) {
      const parsed = parseCatalog(file.text, file.name);
      for (const issue of parsed.issues) {
        issues.push({ ...issue, language });
      }
      if (!parsed.usable) {
        continue;
      }
      if (parsed.language !== language) {
        const named = parsed.language ?? "";
        issues.push(
          makeIssue("error", "language-mismatch", `${file.name} names "${named}", but its folder is "${language}"`, {
            language,
            file: file.name,
          }),
        );
        continue;
      }
      meta = mergeMeta(meta, parsed.meta);
      for (const [key, entry] of parsed.entries) {
        compiled.set(key, entry);
        fileOf.set(key, file.name);
      }
    }

    const reference = options.reference;
    if (reference !== undefined) {
      for (const [key, entry] of [...compiled]) {
        const wanted = reference.holesOf(key) ?? holesOfKey(key);
        if (wanted === undefined || sameHoles(wanted, entry.holes)) {
          continue;
        }
        compiled.delete(key);
        const uses = describeHoles(entry.holes);
        const sourceUses = describeHoles(wanted);
        issues.push(
          makeIssue("error", "holes", `${language} "${key}" uses ${uses} where its source uses ${sourceUses}; the entry is ignored`, {
            language,
            file: fileOf.get(key),
            key,
          }),
        );
      }
    }

    this.language = language;
    this.meta = meta;
    this.direction = meta.direction ?? textDirection(language);
    this.issues = issues;
    this.compiled = compiled;
    this.values = new Map([...compiled].map(([key, entry]): [string, EntryValue] => [key, entry.value]));
    this.rules = new Intl.PluralRules(language);
  }

  /** True when this language has an entry for `key`. It does not look at other languages. */
  has(key: string): boolean {
    return this.compiled.has(key);
  }

  /** Every key this language has an entry for, in the order the files gave them. */
  keys(): string[] {
    return [...this.compiled.keys()];
  }

  /** The hole names of an entry (all its forms together), or undefined when there is no entry. */
  holesOf(key: string): ReadonlySet<string> | undefined {
    return this.compiled.get(key)?.holes;
  }

  /**
   * The text of `key` with its holes filled, from this language only. A key with no entry is shown
   * as itself, read as a template. A plural entry uses its `other` form.
   * @param key - an id, or a text key.
   * @param args - the arguments: a list for `{0}`, `{1}`, or an object for `{gold}`.
   */
  t(key: string, args?: LocaleArgs): string {
    return (this.resolve(key, undefined, args) ?? resolveKey(key, undefined, args)).text;
  }

  /**
   * The text of a plural entry for `count`, with the hole `{count}` filled from `count`. The
   * category comes from `Intl.PluralRules` for this language, and a missing category falls back to
   * `other`. A key with no entry is shown as itself.
   * @param key - an id, or a text key.
   * @param count - the number the text is about.
   * @param args - the other arguments.
   */
  plural(key: string, count: number, args?: LocaleArgs): string {
    return (this.resolve(key, count, args) ?? resolveKey(key, count, args)).text;
  }

  /**
   * Looks a key up in this language only. Games call `t` and `plural`; I18n uses this to walk
   * the fallback chain.
   * @returns the text and the holes without an argument, or undefined when there is no entry here.
   */
  resolve(key: string, count: number | undefined, args: LocaleArgs | undefined): Resolution | undefined {
    const entry = this.compiled.get(key);
    if (entry === undefined) {
      return undefined;
    }
    const category: PluralCategory = count === undefined ? "other" : this.rules.select(count);
    const template = entry.forms.get(category) ?? entry.forms.get("other");
    if (template === undefined) {
      return undefined;
    }
    return renderTemplate(template, lookupFor(args, count));
  }
}

/**
 * Merges the meta block of one file into the block of the files read before it. A field set later wins, except
 * `sameAsSource`, whose lists are joined, so that each file may name its own keys.
 * @param before - the block so far.
 * @param after - the block of the file read next.
 */
function mergeMeta(before: LocaleMeta, after: LocaleMeta): LocaleMeta {
  const merged: LocaleMeta = { ...before, ...after };
  if (before.sameAsSource === undefined || after.sameAsSource === undefined) return merged;
  return { ...merged, sameAsSource: [...new Set([...before.sameAsSource, ...after.sameAsSource])] };
}

/**
 * The text of a key that has no entry: the key itself, read as a template, so that a text key
 * shows its text with the holes filled. A key that is not a valid template is shown as written.
 */
export function resolveKey(key: string, count: number | undefined, args: LocaleArgs | undefined): Resolution {
  const parsed = parseTemplate(key);
  if (!parsed.ok) {
    return { text: key, missing: [] };
  }
  return renderTemplate(parsed.template, lookupFor(args, count));
}

/** The hole names of a key read as a template, or undefined when the key is not a valid template. */
function holesOfKey(key: string): ReadonlySet<string> | undefined {
  const parsed = parseTemplate(key);
  return parsed.ok ? parsed.template.holes : undefined;
}

/** The value of each hole: `count` for the plural count, the arguments for the rest. */
function lookupFor(args: LocaleArgs | undefined, count: number | undefined): (name: string) => LocaleValue | undefined {
  return (name) => (count !== undefined && name === "count" ? count : argumentValue(args, name));
}

/** Compares two names by their Unicode code points, the order the format sorts files in. */
function compareCodePoints(a: string, b: string): number {
  const x = Array.from(a);
  const y = Array.from(b);
  const shared = Math.min(x.length, y.length);
  for (let index = 0; index < shared; index += 1) {
    const left = x[index]?.codePointAt(0) ?? 0;
    const right = y[index]?.codePointAt(0) ?? 0;
    if (left !== right) {
      return left - right;
    }
  }
  return x.length - y.length;
}
