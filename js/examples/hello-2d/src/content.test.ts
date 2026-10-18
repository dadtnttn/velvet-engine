// Tests of the game's content: every text exists in both languages and nothing else is in the catalogs; the
// achievements file is valid and matches the level (its coin count, and the speedrun time in its texts); the sounds
// and the pictures the game names exist, and the pictures are the sizes that their sheets and tileset describe.

import { stat } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { parseAchievements } from "@velvet/achievements";
import { parseAudioManifest } from "@velvet/audio";
import { I18n, validateCatalog, type Locale } from "@velvet/i18n";
import { parseAseprite, parseTexturePacker } from "@velvet/scene2d";
import { ACHIEVEMENTS, ACHIEVEMENTS_URL, COUNTERS, LANGUAGES, LOCALE_URLS, SOUNDS, SPEEDRUN_SECONDS, type Language } from "./config";
import { TEXT_KEYS } from "./texts";
import { PUBLIC_DIR, loadShippedLevel, readPublic, readPublicBytes } from "./test-support";

/** Reads the catalog files of a language, in the form that `addLanguage` takes them. */
async function localeFiles(language: Language): Promise<Array<{ name: string; text: string }>> {
  const files: Array<{ name: string; text: string }> = [];
  for (const url of LOCALE_URLS[language]) {
    files.push({ name: url.slice(url.lastIndexOf("/") + 1), text: await readPublic(url) });
  }
  return files;
}

/** Both languages, loaded the way the game loads them. */
async function loadTexts(): Promise<I18n> {
  const texts = new I18n({ source: "es", fallback: "en" });
  for (const language of LANGUAGES) texts.addLanguage(language, await localeFiles(language));
  return texts;
}

/** The locale of a language that the test has loaded. */
function localeOf(texts: I18n, language: Language): Locale {
  const locale = texts.locale(language);
  if (locale === undefined) throw new Error(`no catalogs for ${language}`);
  return locale;
}

/** The width and height of a PNG file, read from its header. */
async function pngSize(path: string): Promise<{ width: number; height: number }> {
  const bytes = await readPublicBytes(path);
  expect(bytes.subarray(1, 4).toString("ascii"), `${path} is a PNG`).toBe("PNG");
  return { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
}

/** Whether a file of the public folder exists. */
function exists(path: string): Promise<boolean> {
  return stat(resolve(PUBLIC_DIR, path)).then(
    (info) => info.isFile(),
    () => false,
  );
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

  it("has every text that the game looks up, in both languages, and no other text", async () => {
    const texts = await loadTexts();
    for (const language of LANGUAGES) {
      const locale = localeOf(texts, language);
      for (const key of TEXT_KEYS) expect(locale.has(key), `${language}: ${key}`).toBe(true);
      expect([...locale.keys()].sort(), `${language} keys`).toEqual([...TEXT_KEYS].sort());
    }
  });
});

describe("achievements", () => {
  it("has a definitions file that the runtime accepts", async () => {
    const catalog = parseAchievements(await readPublic(ACHIEVEMENTS_URL));
    expect(catalog.achievements.map((entry) => entry.id)).toEqual([
      ACHIEVEMENTS.firstCoin,
      ACHIEVEMENTS.allCoins,
      ACHIEVEMENTS.cleanFinish,
      ACHIEVEMENTS.speedrun,
      "completionist",
    ]);
  });

  it("asks for as many run coins as the level has, so that the collector is possible", async () => {
    const catalog = parseAchievements(await readPublic(ACHIEVEMENTS_URL));
    const level = await loadShippedLevel();
    const collector = catalog.achievements.find((entry) => entry.id === ACHIEVEMENTS.allCoins);
    expect(collector?.goal).toEqual({ kind: "counter", counter: COUNTERS.runCoins, at_least: level.coins.length });
  });

  it("names the speedrun time in both languages, the same as the game checks", async () => {
    for (const language of LANGUAGES) {
      const texts = await loadTexts();
      const text = localeOf(texts, language).t("ach.speedrun.desc");
      expect(text, language).toContain(String(SPEEDRUN_SECONDS));
    }
  });

  it("uses only texts that exist for its names and descriptions", async () => {
    const catalog = parseAchievements(await readPublic(ACHIEVEMENTS_URL));
    for (const entry of catalog.achievements) {
      expect(TEXT_KEYS.includes(entry.name), entry.name).toBe(true);
      expect(TEXT_KEYS.includes(entry.description), entry.description).toBe(true);
    }
  });
});

describe("the sound", () => {
  it("has a manifest with the sounds the game plays, and each file on disk", async () => {
    const manifest = parseAudioManifest(await readPublic("audio/manifest.json"));
    const ids = new Set(manifest.sounds.map((sound) => sound.id));
    for (const id of Object.values(SOUNDS)) expect(ids.has(id), id).toBe(true);
    for (const sound of manifest.sounds) {
      for (const variant of sound.variants) {
        for (const file of variant.files) expect(await exists(`audio/${file}`), `audio/${file}`).toBe(true);
      }
    }
  });
});

describe("the art", () => {
  it("has pictures of the sizes that their sheets and tileset describe", async () => {
    const tiles = await pngSize("art/tiles.png");
    const hero = await pngSize("art/hero.png");
    const coins = await pngSize("art/coins.png");
    expect(tiles).toEqual({ width: 160, height: 64 });
    expect(hero).toEqual({ width: 256, height: 32 });
    expect(coins).toEqual({ width: 96, height: 16 });

    const tileset = JSON.parse(await readPublic("maps/tiles.tsj")) as Record<string, unknown>;
    expect(tileset["imagewidth"]).toBe(tiles.width);
    expect(tileset["imageheight"]).toBe(tiles.height);
    expect(tileset["tilecount"]).toBe(9);
    expect(tileset["columns"]).toBe(5);
    expect(tileset["image"]).toBe("../art/tiles.png");
  });

  it("has a hero atlas with the eight frames of the poses, inside the picture", async () => {
    const atlas = parseTexturePacker(await readPublic("art/hero.json"));
    expect(atlas.names).toEqual(["idle_0", "idle_1", "run_0", "run_1", "run_2", "run_3", "jump_0", "fall_0"]);
    const hero = await pngSize("art/hero.png");
    expect(atlas.size).toEqual({ width: hero.width, height: hero.height });
    for (const name of atlas.names) {
      const frame = atlas.frame(name);
      expect(frame.x + frame.width).toBeLessThanOrEqual(hero.width);
      expect(frame.y + frame.height).toBeLessThanOrEqual(hero.height);
    }
  });

  it("has a coin spin: a tag of six frames with their times, inside the picture", async () => {
    const sheet = parseAseprite(await readPublic("art/coins.json"));
    const spin = sheet.tag("spin");
    expect(spin.frames).toHaveLength(6);
    expect(spin.durations.every((seconds) => seconds > 0)).toBe(true);
    const coins = await pngSize("art/coins.png");
    expect(sheet.size).toEqual({ width: coins.width, height: coins.height });
  });
});
