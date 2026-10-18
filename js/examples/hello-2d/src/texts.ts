// The game's texts: every visible word comes from @velvet/i18n, read from the velvet-locale catalogs in
// public/data/locales. Spanish is the source language, English the fallback. The level has no spoken lines, so
// the catalogs hold only the menus, the score line, the notices, the achievements and the error notice.

import { I18n } from "@velvet/i18n";
import type { Logger } from "@velvet/core";
import { LANGUAGES, LOCALE_URLS, type Language } from "./config";

/**
 * Every text id that the game uses, including the ids that the ui package looks up by itself (the error overlay).
 * The content test checks that both languages have each of them, and that the catalogs have no others.
 */
export const TEXT_KEYS: readonly string[] = [
  "app.title",
  "title.sub",
  "title.play",
  "title.settings",
  "title.hint",
  "pause.title",
  "pause.continue",
  "pause.restart",
  "pause.settings",
  "pause.toTitle",
  "settings.title",
  "settings.sound",
  "settings.sfx",
  "settings.language",
  "settings.languageLabel",
  "settings.reset",
  "settings.back",
  "language.name.es",
  "language.name.en",
  "notice.history.title",
  "notice.history.close",
  "notice.history.empty",
  "complete.heading",
  "complete.time",
  "complete.coins",
  "complete.hits",
  "complete.again",
  "complete.toTitle",
  "error.title",
  "error.body",
  "error.reportText",
  "error.reportId",
  "error.copy",
  "error.copied",
  "error.copyFailed",
  "error.continue",
  "error.reload",
  "hud.coins",
  "hud.time",
  "hud.level",
  "hud.hint",
  "notice.checkpoint",
  "notice.achievement",
  "ach.first_coin.name",
  "ach.first_coin.desc",
  "ach.all_coins.name",
  "ach.all_coins.desc",
  "ach.clean_finish.name",
  "ach.clean_finish.desc",
  "ach.speedrun.name",
  "ach.speedrun.desc",
  "ach.completionist.name",
  "ach.completionist.desc",
];

/**
 * Loads both languages. The source language loads first, which `LANGUAGES` guarantees. A text that a language
 * lacks is logged once, and the lookup then falls back to English, and at last to the id.
 * @param logger - the game's logger, for the problems of the catalogs.
 * @throws when a catalog file cannot be fetched or is not valid.
 */
export async function createTexts(logger: Logger): Promise<I18n> {
  const texts = new I18n({
    source: "es",
    fallback: "en",
    onMissing: (key, language) => logger.warn(`no text for ${key} in ${language}`),
    onIssue: (issue) => logger.warn(`catalog: ${issue.message}`),
  });
  for (const language of LANGUAGES) {
    await texts.addLanguageFromUrls(language, LOCALE_URLS[language]);
  }
  return texts;
}

/**
 * The language that a stored setting names.
 * @param value - the value read from the settings.
 * @returns the language, or null when the value names none the game ships.
 */
export function asLanguage(value: unknown): Language | null {
  return LANGUAGES.find((language) => language === value) ?? null;
}
