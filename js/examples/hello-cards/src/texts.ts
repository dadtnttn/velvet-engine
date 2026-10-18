// The game's texts: every visible string of the interface comes from @velvet/i18n, read from the
// velvet-locale catalogs in public/data/locales. Spanish is the source language, English the fallback.

import { I18n } from "@velvet/i18n";
import { LANGUAGES, LOCALE_URLS, type Language } from "./config";
import { logLine } from "./platform";

/**
 * Loads both languages. The source language must load first, which `LANGUAGES` guarantees. A
 * missing text is logged once per key and language, and then shows its id.
 */
export async function createTexts(): Promise<I18n> {
  const texts = new I18n({
    source: "es",
    fallback: "en",
    onMissing: (key, language) => logLine("warn", `no text for ${key} in ${language}`),
    onIssue: (issue) => logLine("warn", `catalog: ${issue.message}`),
  });
  for (const language of LANGUAGES) {
    await texts.addLanguageFromUrls(language, LOCALE_URLS[language]);
  }
  return texts;
}

/** The language that a stored setting names, or null when it names none the game ships. */
export function asLanguage(value: unknown): Language | null {
  return LANGUAGES.find((language) => language === value) ?? null;
}
