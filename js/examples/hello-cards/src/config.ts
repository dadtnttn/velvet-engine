// Settings of hello-cards: the logical surface, the clock, the save slots, the data files and the
// starting table. Everything a designer might tune is here, so the game modules hold only logic.

/** The languages the game ships. `es` is the source language: its catalogs hold every id. */
export const LANGUAGES = ["es", "en"] as const;

/** One of the languages in {@link LANGUAGES}. */
export type Language = (typeof LANGUAGES)[number];

/** Each language's own name, for the pause menu. Names are not translated. */
export const LANGUAGE_NAMES: Readonly<Record<Language, string>> = { es: "Español", en: "English" };

/** The game's id. It must equal the `id` in velvet-game.json, because saves and logs use it. */
export const GAME_ID = "hello-cards";

/** The version of the save data. Bump it, and add a migration, when the shape changes. */
export const SAVE_VERSION = 1;

/** The logical surface, in pixels. Every position in the game is written in these units. */
export const SURFACE = { width: 1920, height: 1080 } as const;

/** The speed multipliers of the clock. The first one is the speed a new game starts at. */
export const SPEEDS: readonly number[] = [1, 2, 3];

/** The length of one in-game day, in game seconds. A short day keeps the demo lively. */
export const DAY_LENGTH = 120;

/** The save slot of the pause menu, and the slot that F5 and F9 use. Both are valid slot names. */
export const MANUAL_SLOT = "slot-1";
export const QUICK_SLOT = "quick";

/** The card catalog, in the velvet-stacks format (docs/formats/STACKS.md). */
export const CATALOG_URL = "data/catalog.json";

/** The catalog files of each language, in name order (docs/formats/LOCALE.md). */
export const LOCALE_URLS: Readonly<Record<Language, readonly string[]>> = {
  es: ["data/locales/es/00_ui.json", "data/locales/es/10_cards.json"],
  en: ["data/locales/en/00_ui.json", "data/locales/en/10_cards.json"],
};

/**
 * The intro conversation of each language. `name` is the file name that the guion library gives
 * the file (its id is the name without `.md`, slugged: `intro.en.md` is `intro_en`), `url` is where
 * the file is served, and `scene` is the scene that starts the conversation.
 */
export interface IntroFile {
  readonly name: string;
  readonly url: string;
  readonly scene: string;
}

/** The intro conversation of each language. */
export const INTRO_FILES: Readonly<Record<Language, IntroFile>> = {
  es: { name: "intro.md", url: "data/guion/intro.md", scene: "intro.inicio" },
  en: { name: "intro.en.md", url: "data/guion/intro.en.md", scene: "intro_en.start" },
};

/** One card that a new game starts with: its definition, and its position in board units. */
export interface StartCard {
  readonly def: string;
  readonly x: number;
  readonly y: number;
}

/** The table of a new game. */
export const START_CARDS: readonly StartCard[] = [
  { def: "villager", x: 0.4, y: 0.5 },
  { def: "villager", x: 0.4, y: 2.1 },
  { def: "forest", x: 2.6, y: 0.5 },
  { def: "quarry", x: 2.6, y: 2.1 },
  { def: "berry_bush", x: 2.6, y: 3.7 },
  { def: "wood", x: 6.0, y: 0.5 },
  { def: "wood", x: 6.0, y: 2.1 },
];

/** The materials that the top bar counts, in the order it shows them. Coins have their own readout. */
export const STOCK_DEFS: readonly string[] = ["wood", "plank", "stone", "berries"];

/** The recipes that only gather. Their results stay quiet, because they finish every few seconds. */
export const GATHER_RECIPES: readonly string[] = ["gather_wood", "gather_stone", "gather_berries"];

/** The toast queue: at most this many notices, each shown for `duration` ms and faded over `fadeMs`. */
export const TOAST = { maxVisible: 3, duration: 2600, fadeMs: 300 } as const;

/** How long a card takes to fade out when the table consumes it, in milliseconds. */
export const FADE_MS = 320;
