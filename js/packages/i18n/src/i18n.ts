// I18n: the loaded languages, the fallback chain between them, and the language in use.
//
// For a requested language T, a lookup tries T, then its base, then the fallback language, then
// the source language. The first catalog that has the key answers. Failing all of them, the key
// itself is shown, and the key is reported as missing once per language.

import { LocaleError, makeIssue, type Direction, type LocaleArgs, type LocaleIssue } from "./format";
import { Locale, resolveKey, type CatalogFile } from "./locale";
import { readCatalogUrls, type CatalogFetch } from "./load";
import { baseLanguageTag, isLanguageTag, textDirection } from "./tags";

/** Options for an I18n instance. */
export interface I18nOptions {
  /** The source language: the language the game's keys are written in. Load it first. */
  readonly source: string;
  /** The fallback language, tried after the requested language and its base. Defaults to the source. */
  readonly fallback?: string;
  /** Called once per language and key, when a key has no text in any catalog of the chain. */
  readonly onMissing?: (key: string, language: string) => void;
  /**
   * Called with each issue found while a language loads, and with each hole that a lookup
   * could not fill (once per language, key and hole name).
   */
  readonly onIssue?: (issue: LocaleIssue) => void;
}

/** A listener for language changes. It receives the new language's tag. */
export type LanguageListener = (language: string) => void;

/** The outcome of a lookup: the text, and the language that answered (undefined: the key itself). */
export interface Lookup {
  readonly text: string;
  readonly language: string | undefined;
}

/**
 * The texts of a game in every loaded language, with the language in use.
 *
 * @example
 * const i18n = new I18n({ source: "es", fallback: "en" });
 * i18n.addLanguage("es", esFiles);
 * i18n.addLanguage("en", enFiles);
 * i18n.setLanguage("en");
 * i18n.t("menu.start"); // "Start"
 */
export class I18n {
  private readonly source: string;
  private readonly fallback: string;
  private readonly onMissing: ((key: string, language: string) => void) | undefined;
  private readonly onIssue: ((issue: LocaleIssue) => void) | undefined;
  private readonly locales = new Map<string, Locale>();
  private readonly listeners = new Set<LanguageListener>();
  private readonly reportedKeys = new Set<string>();
  private readonly reportedArguments = new Set<string>();
  private current: string;
  private steps: readonly string[];

  /**
   * @param options - the source and fallback languages, and the callbacks.
   * @throws LocaleError with code "language" when a tag is not in canonical case.
   */
  constructor(options: I18nOptions) {
    const fallback = options.fallback ?? options.source;
    if (!isLanguageTag(options.source)) {
      throw new LocaleError("language", `"${options.source}" is not a language tag in canonical case`);
    }
    if (!isLanguageTag(fallback)) {
      throw new LocaleError("language", `"${fallback}" is not a language tag in canonical case`);
    }
    this.source = options.source;
    this.fallback = fallback;
    this.onMissing = options.onMissing;
    this.onIssue = options.onIssue;
    this.current = options.source;
    this.steps = this.chainFor(options.source);
  }

  /** The language in use: the tag that was set, whether or not it has catalogs of its own. */
  get language(): string {
    return this.current;
  }

  /** The tags of the loaded languages, in the order they were added. */
  get languages(): string[] {
    return [...this.locales.keys()];
  }

  /**
   * The languages a lookup tries for the language in use, in order. Steps without catalogs are
   * skipped by lookups.
   */
  get chain(): readonly string[] {
    return this.steps;
  }

  /** The direction of the language in use: from its catalogs, or from its primary subtag. */
  get direction(): Direction {
    return this.locales.get(this.current)?.direction ?? textDirection(this.current);
  }

  /** The loaded locale of `language`, or undefined when that language is not loaded. */
  locale(language: string): Locale | undefined {
    return this.locales.get(language);
  }

  /**
   * Loads one language from catalog files. The source language must be loaded first. A file's
   * holes are checked against the source, and the entries that differ are ignored.
   *
   * @param language - the tag of the folder the files come from.
   * @param files - the catalog files.
   * @returns the loaded locale. Its issues are also passed to `onIssue`.
   * @throws LocaleError with code "duplicate-language", "source-missing" or "language".
   */
  addLanguage(language: string, files: readonly CatalogFile[]): Locale {
    if (this.locales.has(language)) {
      throw new LocaleError("duplicate-language", `the ${language} catalogs are already loaded`);
    }
    if (language !== this.source && !this.locales.has(this.source)) {
      throw new LocaleError("source-missing", `load the source language ${this.source} before ${language}`);
    }
    const reference = language === this.source ? undefined : this.locales.get(this.source);
    const locale = new Locale(language, files, { reference });
    this.locales.set(language, locale);
    for (const issue of locale.issues) {
      this.onIssue?.(issue);
    }
    return locale;
  }

  /**
   * Fetches a language's catalog files and loads them. See `addLanguage`.
   * @param language - the tag of the folder the files come from.
   * @param urls - the catalog file URLs.
   * @param fetchFn - the fetch to use. Defaults to the global `fetch`.
   * @throws LocaleError with code "fetch" when a file cannot be fetched.
   */
  async addLanguageFromUrls(language: string, urls: readonly string[], fetchFn?: CatalogFetch): Promise<Locale> {
    return this.addLanguage(language, await readCatalogUrls(urls, fetchFn));
  }

  /**
   * Makes `language` the language in use. Its catalogs need not be loaded when its base is.
   * @throws LocaleError with code "language" for a malformed tag, and "unknown-language" when
   * neither the tag nor its base is loaded.
   */
  setLanguage(language: string): void {
    if (!isLanguageTag(language)) {
      throw new LocaleError("language", `"${language}" is not a language tag in canonical case`);
    }
    if (language === this.current) {
      return;
    }
    const base = baseLanguageTag(language);
    if (!this.locales.has(language) && (base === undefined || !this.locales.has(base))) {
      throw new LocaleError("unknown-language", `no catalogs are loaded for ${language}`);
    }
    this.current = language;
    this.steps = this.chainFor(language);
    for (const listener of [...this.listeners]) {
      listener(language);
    }
  }

  /**
   * Calls `listener` each time the language in use changes.
   * @returns a function that removes the listener.
   */
  onChange(listener: LanguageListener): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  /**
   * The text of `key` with its holes filled, in the language in use.
   * @param key - an id, or a text key.
   * @param args - the arguments: a list for `{0}`, `{1}`, or an object for `{gold}`.
   */
  t(key: string, args?: LocaleArgs): string {
    return this.lookup(key, args).text;
  }

  /**
   * The text of a plural entry for `count`, in the language in use. See `Locale.plural`.
   * @param key - an id, or a text key.
   * @param count - the number the text is about; it fills `{count}`.
   * @param args - the other arguments.
   */
  plural(key: string, count: number, args?: LocaleArgs): string {
    return this.lookup(key, args, count).text;
  }

  /** True when some catalog of the chain has an entry for `key`. */
  has(key: string): boolean {
    return this.steps.some((step) => this.locales.get(step)?.has(key) === true);
  }

  /**
   * A lookup that also says which language answered, for tools and tests.
   * @param key - an id, or a text key.
   * @param args - the arguments.
   * @param count - the plural count, when the key is a plural entry.
   */
  lookup(key: string, args?: LocaleArgs, count?: number): Lookup {
    for (const step of this.steps) {
      const locale = this.locales.get(step);
      if (locale === undefined) {
        continue;
      }
      const found = locale.resolve(key, count, args);
      if (found !== undefined) {
        this.reportArguments(found.missing, key);
        return { text: found.text, language: step };
      }
    }
    if (this.current !== this.source) {
      this.reportKey(key);
    }
    const shown = resolveKey(key, count, args);
    this.reportArguments(shown.missing, key);
    return { text: shown.text, language: undefined };
  }

  /** The chain for a language: the language, its base, the fallback and the source. */
  private chainFor(language: string): readonly string[] {
    const steps = [language];
    const base = baseLanguageTag(language);
    if (base !== undefined) {
      steps.push(base);
    }
    if (!steps.includes(this.source)) {
      steps.push(this.fallback);
    }
    steps.push(this.source);
    return [...new Set(steps)];
  }

  private reportKey(key: string): void {
    const memo = `${this.current}\n${key}`;
    if (this.reportedKeys.has(memo)) {
      return;
    }
    this.reportedKeys.add(memo);
    this.onMissing?.(key, this.current);
  }

  private reportArguments(names: readonly string[], key: string): void {
    for (const name of names) {
      const memo = `${this.current}\n${key}\n${name}`;
      if (this.reportedArguments.has(memo)) {
        continue;
      }
      this.reportedArguments.add(memo);
      this.onIssue?.(
        makeIssue("warning", "missing-argument", `"${key}" has no argument for {${name}}`, {
          language: this.current,
          key,
        }),
      );
    }
  }
}
