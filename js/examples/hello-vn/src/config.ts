// Settings of hello-vn: the languages, the surface, the story files, the data files, the sounds and
// the numbers a writer may tune. Everything here is data; the game modules hold the logic.

/** The languages the game ships. `es` is the source language: its catalogs hold every id. */
export const LANGUAGES = ["es", "en"] as const;

/** One of the languages in {@link LANGUAGES}. */
export type Language = (typeof LANGUAGES)[number];

/** Each language's own name, for the settings menu. Names are not translated. */
export const LANGUAGE_NAMES: Readonly<Record<Language, string>> = { es: "Español", en: "English" };

/** The game's id. It must equal the `id` in velvet-game.json, because saves use it. */
export const GAME_ID = "hello-vn";

/** The version of the save data. Bump it, and add a migration, when the shape changes. */
export const SAVE_VERSION = 1;

/** The logical surface, in pixels. Every position in the game is written in these units. */
export const SURFACE = { width: 1920, height: 1080 } as const;

/** The namespace and version of the game's settings. */
export const SETTINGS_NAMESPACE = "hello-vn";
export const SETTINGS_VERSION = 1;

/** The art manifest, and the sound manifest, both in the folders of public/. */
export const ART_MANIFEST_URL = "art/manifest.json";
export const AUDIO_MANIFEST_URL = "audio/manifest.json";

/** The first scene of a new game. Its label is a scene of the guion. */
export const START_SCENE = "puerto";

/** The scenes of the story, in the order a reader meets them. Each is a `##` heading in the guion. */
export const SCENES = ["puerto", "taberna", "faro"] as const;

/** One scene of the story, by its label. */
export type SceneLabel = (typeof SCENES)[number];

/** The guion of each language: the file name the library gives it, and where it is served. */
export interface StoryFile {
  readonly name: string;
  readonly url: string;
}

/** The story of each language. The two files have the same scenes and boxes, so a save plays in both. */
export const STORY_FILES: Readonly<Record<Language, StoryFile>> = {
  es: { name: "faro.md", url: "data/guion/faro.md" },
  en: { name: "faro.en.md", url: "data/guion/faro.en.md" },
};

/** The cast of the guiones: each speaker id with the names the files use for it. */
export const CAST = {
  characters: { tome: ["Tomé"], lupa: ["Lupa"] },
} as const;

/** The catalog files of each language, in name order (docs/formats/LOCALE.md). */
export const LOCALE_URLS: Readonly<Record<Language, readonly string[]>> = {
  es: ["data/locales/es/00_menus.json", "data/locales/es/10_story.json"],
  en: ["data/locales/en/00_menus.json", "data/locales/en/10_story.json"],
};

/** The sounds the game plays, by sound id (public/audio/manifest.json). */
export const SOUNDS = {
  blip: "ui.blip",
  click: "ui.click",
  pad: "music.pad",
} as const;

/** The art ids of the title and the places, and the prefix of a portrait's id. */
export const ART = {
  title: "bg.titulo",
  places: { puerto: "bg.puerto", taberna: "bg.taberna", faro: "bg.faro" },
  portraitPrefix: "portrait.",
} as const;

/** The story variable that says Lupa's sea glass is in the bag, and the variable that picks the ending. */
export const GLASS_VAR = "lente";
export const ENDING_VAR = "final";

/** The text speeds the settings menu cycles through, in letters a second (0 writes a line at once). */
export const TEXT_SPEEDS: readonly number[] = [0, 20, 40, 80];

/** The waits of automatic reading the settings menu cycles through, in seconds. */
export const AUTO_DELAYS: readonly number[] = [0.5, 1.5, 3];

/** The volume steps the settings menu cycles through, from 0 to 1. */
export const VOLUME_STEPS: readonly number[] = [0, 0.25, 0.5, 0.75, 1];

/** The defaults of the settings, before the player changes one. */
export const SETTINGS_DEFAULTS = {
  textSpeed: 40,
  autoDelay: 1.5,
  musicVolume: 0.5,
  sfxVolume: 0.75,
  language: null as Language | null,
} as const;

/** The notice that the game shows when a save or a load finishes, and how long it stays. */
export const NOTICE = { maxVisible: 3, durationMs: 2600, fadeMs: 300 } as const;
