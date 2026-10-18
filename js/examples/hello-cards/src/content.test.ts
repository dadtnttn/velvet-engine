// Tests of the game's content: every text exists in both languages, the catalog's names are there,
// and the intro guiones parse without problems and start at the scene the game names.

import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import { loadCatalog } from "@velvet/cards";
import { parseGuion } from "@velvet/guion";
import { I18n, validateCatalog, type Locale } from "@velvet/i18n";
import { GATHER_RECIPES, INTRO_FILES, LANGUAGES, LOCALE_URLS, type Language } from "./config";

/** A file of the public folder, as text. */
function readPublic(path: string): Promise<string> {
  return readFile(new URL(`../public/${path}`, import.meta.url), "utf8");
}

/** The catalog files of a language, in the form that `addLanguage` takes them. */
async function localeFiles(language: Language): Promise<Array<{ name: string; text: string }>> {
  const files: Array<{ name: string; text: string }> = [];
  for (const url of LOCALE_URLS[language]) {
    files.push({ name: url.slice(url.lastIndexOf("/") + 1), text: await readPublic(url) });
  }
  return files;
}

/** Every text id that the game looks up by name in its code. Keep this list in step with the code. */
const CODE_KEYS: readonly string[] = [
  "app.title",
  "table.label",
  "job.progress",
  "hud.day",
  "hud.dayBar",
  "hud.pause",
  "hud.resume",
  "hud.menu",
  "hud.coins",
  "hint.pause",
  "hint.speed",
  "hint.quickSave",
  "hint.quickLoad",
  "hint.menu",
  "hint.drag",
  "dialogue.next",
  "pause.title",
  "pause.continue",
  "pause.save",
  "pause.load",
  "pause.langOn",
  "pause.fullscreen.on",
  "pause.fullscreen.off",
  "pause.quit",
  "toast.day",
  "toast.saved",
  "toast.saveFailed",
  "toast.loaded",
  "toast.loadFailed",
  "toast.noSave",
  "toast.refused",
  "toast.full",
  "toast.back",
  "toast.cancelled",
  "toast.closeTab",
  "error.boot",
];

/** Both languages of the game, loaded the way the game loads them. */
async function loadTexts(): Promise<I18n> {
  const texts = new I18n({ source: "es", fallback: "en" });
  for (const language of LANGUAGES) {
    texts.addLanguage(language, await localeFiles(language));
  }
  return texts;
}

/** The locale of a language that the test has loaded. */
function localeOf(texts: I18n, language: Language): Locale {
  const locale = texts.locale(language);
  if (locale === undefined) throw new Error(`no catalogs for ${language}`);
  return locale;
}

describe("texts", () => {
  it("loads both languages with no errors, and every Spanish text has an English one", async () => {
    const texts = await loadTexts();
    const spanish = localeOf(texts, "es");
    const english = localeOf(texts, "en");
    expect(spanish.issues.filter((issue) => issue.severity === "error")).toEqual([]);
    expect(english.issues.filter((issue) => issue.severity === "error")).toEqual([]);
    expect(validateCatalog(spanish, english)).toEqual([]);
  });

  it("has every text that the code looks up, in both languages", async () => {
    const texts = await loadTexts();
    for (const language of LANGUAGES) {
      const locale = localeOf(texts, language);
      for (const key of CODE_KEYS) expect(locale.has(key), `${language}: ${key}`).toBe(true);
    }
  });

  it("has the names of every card and recipe of the catalog", async () => {
    const texts = await loadTexts();
    const catalog = await loadCatalog(await readPublic("data/catalog.json"));
    for (const language of LANGUAGES) {
      const locale = localeOf(texts, language);
      for (const card of catalog.cards) {
        expect(locale.has(card.name), `${language}: ${card.name}`).toBe(true);
      }
      for (const recipe of catalog.recipes) {
        expect(locale.has(`recipe.${recipe.id}`), `${language}: recipe.${recipe.id}`).toBe(true);
        if (!GATHER_RECIPES.includes(recipe.id)) {
          expect(locale.has(`done.${recipe.id}`), `${language}: done.${recipe.id}`).toBe(true);
        }
      }
    }
  });
});

describe("the intro guiones", () => {
  for (const language of LANGUAGES) {
    it(`parses the ${language} guion with no problems, and has its starting scene`, async () => {
      const file = INTRO_FILES[language];
      const parsed = await parseGuion(await readPublic(file.url), { file: file.name });
      expect(parsed.problems).toEqual([]);
      expect(parsed.scenes.map((scene) => scene.id)).toContain(file.scene);
    });
  }
});
