// Coverage of a game's texts: which keys each language lacks, which keys nothing uses, and which texts
// were left as they are in the source. The source language holds every id; a text key (the source text
// itself) needs an entry in each other language, so each text that the code shows is listed as well.

import type { EntryValue } from "@velvet/i18n";
import { makeIssue, WHOLE_FILE, type Issue } from "../issues";
import { isIdKey } from "../game/keys";
import type { LoadedLocale, LoadedLocales } from "../game/locales";
import type { CodeScan } from "./scan";

/** The coverage of one language. */
export interface LanguageCoverage {
  readonly tag: string;
  /** The folder of the language, relative to the game folder. */
  readonly folder: string;
  /** True for the source language, which is the reference for the others. */
  readonly isSource: boolean;
  /** Keys that the source (or the code) has and this language has no text for. */
  readonly missing: readonly string[];
  /** Keys in this language that no code, string or data uses. */
  readonly unused: readonly string[];
  /** Keys whose text is the same as the source text, in a language that is not the source. */
  readonly untranslated: readonly string[];
}

/** The whole coverage report of a game. */
export interface CoverageReport {
  readonly languages: readonly LanguageCoverage[];
  /** Every problem: the load problems of the catalogs, the coverage gaps, and the ids the source lacks. */
  readonly issues: Issue[];
}

/** The text of an entry, as the source language's reader would give it: the `other` form for a plural. */
function textOf(value: EntryValue): string {
  return typeof value === "string" ? value : value.other;
}

/**
 * The keys that the code shows as text: literal keys that are not ids, which the source need not hold.
 * @param scan - the code scan.
 * @param source - the source language, when there is one.
 */
export function textKeysOf(scan: CodeScan, source: LoadedLocale | undefined): string[] {
  const keys = new Set<string>();
  for (const call of scan.calls) {
    if (isIdKey(call.key)) continue;
    if (source !== undefined && source.locale.has(call.key)) continue;
    keys.add(call.key);
  }
  return [...keys].sort();
}

/**
 * Compares the languages with each other and with the code.
 * @param locales - the loaded languages, with their own load problems.
 * @param scan - what the code uses.
 * @param dataKeys - the keys that the game's data names in its key fields.
 */
export function coverageOf(locales: LoadedLocales, scan: CodeScan, dataKeys: ReadonlySet<string>): CoverageReport {
  const issues: Issue[] = [...locales.layout.issues];
  const source = locales.source;
  const callKeys = new Set(scan.calls.map((call) => call.key));
  const used = (key: string): boolean =>
    scan.strings.has(key) ||
    dataKeys.has(key) ||
    callKeys.has(key) ||
    scan.prefixes.some((prefix) => key.startsWith(prefix));

  for (const language of locales.languages) issues.push(...language.issues);

  // A key that the code asks for as an id must be in the source, or the game shows the id itself.
  const reported = new Set<string>();
  for (const call of scan.calls) {
    if (!isIdKey(call.key) || source === undefined || source.locale.has(call.key)) continue;
    const marker = `${call.file}\u0000${call.key}`;
    if (reported.has(marker)) continue;
    reported.add(marker);
    issues.push(
      makeIssue(
        "error",
        call.file,
        WHOLE_FILE,
        "missing-source-key",
        `the code asks for "${call.key}", which the source language (${source.tag}) does not have`,
      ),
    );
  }

  const textKeys = textKeysOf(scan, source);
  const expected = source === undefined ? [] : [...source.locale.keys(), ...textKeys];
  const languages: LanguageCoverage[] = [];
  for (const language of locales.languages) {
    const isSource = source !== undefined && language.tag === source.tag;
    const { locale } = language;
    const missing = isSource ? [] : [...new Set(expected)].filter((key) => !locale.has(key));
    const unused = locale.keys().filter((key) => !used(key));
    const untranslated: string[] = [];
    if (!isSource && source !== undefined) {
      for (const key of locale.keys()) {
        const sourceValue = source.locale.values.get(key);
        const sourceText = sourceValue === undefined ? key : textOf(sourceValue);
        const translated = locale.values.get(key);
        if (translated === undefined) continue;
        if (textOf(translated) === sourceText && /\p{L}/u.test(sourceText)) untranslated.push(key);
      }
    }
    languages.push({
      tag: language.tag,
      folder: language.folder,
      isSource,
      missing: missing.sort(),
      unused: unused.sort(),
      untranslated: untranslated.sort(),
    });
  }

  for (const coverage of languages) {
    for (const key of coverage.missing) {
      issues.push(makeIssue("warning", coverage.folder, `entries["${key}"]`, "missing-key", `${coverage.tag} has no text for "${key}", so the game shows the key or the source text`));
    }
    for (const key of coverage.unused) {
      issues.push(makeIssue("warning", coverage.folder, `entries["${key}"]`, "unused-key", `no code or data uses "${key}"`));
    }
    for (const key of coverage.untranslated) {
      issues.push(makeIssue("warning", coverage.folder, `entries["${key}"]`, "untranslated", `${coverage.tag} keeps the source text of "${key}"`));
    }
  }
  return { languages, issues };
}
