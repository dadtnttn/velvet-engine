// Tests of the game's content: every text exists in both languages and nothing else is in the catalogs, the two
// guiones parse without problems and have the same scenes and endings, and every art id, sound id and sound file
// that the story or the game names exists.

import { stat } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { parseArtManifest } from "@velvet/assets";
import { parseAudioManifest } from "@velvet/audio";
import { parseGuion } from "@velvet/guion";
import { I18n, validateCatalog, type Locale } from "@velvet/i18n";
import { ART, LANGUAGES, LOCALE_URLS, SCENES, SOUNDS, STORY_FILES, type Language } from "./config";
import { TEXT_KEYS } from "./texts";
import { PUBLIC_DIR, readPublic } from "./test-support";

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

/** The text of a guion file, read from the public folder. */
function guionText(language: Language): Promise<string> {
  return readPublic(STORY_FILES[language].url);
}

/** The speakers and moods of a guion, read from its dialogue lines (`**Name (mood):** text`). */
function speakersOf(text: string): Array<{ name: string; mood: string | null }> {
  const found: Array<{ name: string; mood: string | null }> = [];
  for (const match of text.matchAll(/^\*\*([^*(]+?)\s*(?:\(([^)]+)\))?:\*\*/gm)) {
    found.push({ name: (match[1] ?? "").trim(), mood: match[2]?.trim() ?? null });
  }
  return found;
}

/** The slug of a name, for the speaker ids: ASCII letters and digits, with the accents folded. */
function speakerId(name: string): string {
  return name
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
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

describe("the guiones", () => {
  for (const language of LANGUAGES) {
    it(`parses the ${language} guion with no problems, and has every scene of the story`, async () => {
      const file = STORY_FILES[language];
      const parsed = await parseGuion(await guionText(language), { file: file.name });
      expect(parsed.problems).toEqual([]);
      const labels = parsed.scenes.map((scene) => scene.id.slice(scene.id.indexOf(".") + 1));
      expect(labels).toEqual([...SCENES]);
    });
  }

  it("ends in two endings, with the same numbers in both languages", async () => {
    for (const language of LANGUAGES) {
      const text = await guionText(language);
      expect(text.match(/-> fin \{final = 1\}/g)?.length ?? 0, `${language} ending 1`).toBe(1);
      expect(text.match(/-> fin \{final = 2\}/g)?.length ?? 0, `${language} ending 2`).toBe(1);
    }
  });

  it("has the same speakers, moods and stage directives in both languages, line for line", async () => {
    const spanish = await guionText("es");
    const english = await guionText("en");
    expect(speakersOf(english)).toEqual(speakersOf(spanish));
    const directives = (text: string): string[] => text.match(/^@\S+.*$/gm) ?? [];
    expect(directives(english)).toEqual(directives(spanish));
    const choices = (text: string): number => text.match(/^- /gm)?.length ?? 0;
    expect(choices(english)).toBe(choices(spanish));
  });
});

describe("the art", () => {
  it("has a manifest that declares every background and portrait the game names", async () => {
    const manifest = parseArtManifest(await readPublic("art/manifest.json"));
    const ids = new Set(manifest.entries.map((entry) => entry.id));
    expect(ids.has(ART.title)).toBe(true);
    for (const id of Object.values(ART.places)) expect(ids.has(id), id).toBe(true);
    for (const language of LANGUAGES) {
      const text = await guionText(language);
      for (const match of text.matchAll(/^@fondo\s+(\S+)/gm)) {
        expect(ids.has(match[1] ?? ""), `@fondo ${match[1]}`).toBe(true);
      }
      for (const speaker of speakersOf(text)) {
        const id = `${ART.portraitPrefix}${speakerId(speaker.name)}`;
        expect(ids.has(id), `${language}: ${id}`).toBe(true);
        const entry = manifest.entries.find((candidate) => candidate.id === id);
        if (speaker.mood !== null) {
          expect(entry?.variants.includes(speaker.mood), `${language}: ${id} (${speaker.mood})`).toBe(true);
        }
      }
    }
  });

  it("has portraits as big as their slot in the dialogue box, and a size for every picture", async () => {
    const manifest = parseArtManifest(await readPublic("art/manifest.json"));
    for (const entry of manifest.entries) {
      if (entry.kind === "portrait") expect(entry.size, entry.id).toEqual([360, 520]);
      if (entry.kind === "background") expect(entry.size, entry.id).toEqual([1920, 1080]);
    }
  });
});

describe("the sound", () => {
  it("has a manifest with the sounds the game plays, on their buses, and each file on disk", async () => {
    const manifest = parseAudioManifest(await readPublic("audio/manifest.json"));
    const byId = new Map(manifest.sounds.map((sound) => [sound.id, sound]));
    expect(byId.get(SOUNDS.blip)?.bus).toBe("ui");
    expect(byId.get(SOUNDS.click)?.bus).toBe("ui");
    expect(byId.get(SOUNDS.pad)?.bus).toBe("music");
    expect(byId.get(SOUNDS.pad)?.loop).toBe(true);
    for (const sound of manifest.sounds) {
      for (const variant of sound.variants) {
        for (const file of variant.files) {
          const found = await stat(resolve(PUBLIC_DIR, "audio", file)).then(
            (info) => info.isFile(),
            () => false,
          );
          expect(found, `audio/${file}`).toBe(true);
        }
      }
    }
  });

  it("names only music that is on the music bus", async () => {
    const manifest = parseAudioManifest(await readPublic("audio/manifest.json"));
    const bus = new Map(manifest.sounds.map((sound) => [sound.id, sound.bus]));
    for (const language of LANGUAGES) {
      for (const match of (await guionText(language)).matchAll(/^@musica\s+(\S+)/gm)) {
        expect(bus.get(match[1] ?? ""), `@musica ${match[1]}`).toBe("music");
      }
    }
  });
});
