// The game's texts: every visible word of the interface comes from @velvet/i18n, read from the
// velvet-locale catalogs in public/data/locales. Spanish is the source language, English the fallback.
// The story's own lines are not here: they are the guiones (public/data/guion), one file per language.

import { I18n } from "@velvet/i18n";
import { LANGUAGES, LOCALE_URLS, type Language } from "./config";
import { logLine } from "./platform";

/**
 * Every text id that the game uses, including the ids that the ui package looks up by itself (the save
 * list, the dialogue box and the notices). The content test checks that both languages have each of them,
 * and that the catalogs have no others.
 */
export const TEXT_KEYS: readonly string[] = [
  "app.title",
  "title.sub",
  "title.new",
  "title.load",
  "title.settings",
  "pause.title",
  "pause.continue",
  "pause.save",
  "pause.load",
  "pause.settings",
  "pause.toTitle",
  "settings.title",
  "settings.speed",
  "settings.instant",
  "settings.autoDelay",
  "settings.music",
  "settings.sfx",
  "settings.language",
  "settings.back",
  "game.menu",
  "game.hint.next",
  "game.key.fast",
  "game.key.menu",
  "game.key.next",
  "game.hint.menu",
  "game.hint.fast",
  "game.item.lente",
  "dialogue.next",
  "dialogue.auto",
  "dialogue.skip",
  "dialogue.log",
  "dialogue.log.title",
  "dialogue.log.close",
  "dialogue.choices",
  "save.autosave",
  "save.slot",
  "save.empty",
  "save.damaged",
  "save.untitled",
  "save.delete",
  "save.confirm.overwrite.title",
  "save.confirm.overwrite.body",
  "save.confirm.delete.title",
  "save.confirm.delete.body",
  "save.confirm.yes",
  "save.confirm.no",
  "save.status.saved",
  "save.status.loaded",
  "save.status.deleted",
  "save.status.empty",
  "save.status.failed",
  "notice.history.title",
  "notice.history.close",
  "notice.history.empty",
  "notice.saved",
  "notice.loaded",
  "notice.saveFailed",
  "notice.loadFailed",
  "scene.puerto",
  "scene.taberna",
  "scene.faro",
  "ending.heading",
  "ending.back",
  "ending.boxes",
  "ending.luz.title",
  "ending.luz.body",
  "ending.campana.title",
  "ending.campana.body",
];

/** The text key of each place, by scene label: the place's name in the top bar and in the save list. */
export const SCENE_KEYS: Readonly<Record<string, string>> = {
  puerto: "scene.puerto",
  taberna: "scene.taberna",
  faro: "scene.faro",
};

/**
 * Loads both languages. The source language must load first, which `LANGUAGES` guarantees. A missing text
 * is logged once per key and language, and then shows its id.
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

/**
 * The language that a stored setting names.
 * @param value - the value read from the settings.
 * @returns the language, or null when the value names none the game ships.
 */
export function asLanguage(value: unknown): Language | null {
  return LANGUAGES.find((language) => language === value) ?? null;
}
