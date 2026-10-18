// Settings of hello-2d: the languages, the surface, the files, the sounds and the numbers that a designer may tune.
// Everything here is data; the modules hold the logic.

/** The languages the game ships. `es` is the source language: its catalogs hold every text id. */
export const LANGUAGES = ["es", "en"] as const;

/** One of the languages in {@link LANGUAGES}. */
export type Language = (typeof LANGUAGES)[number];

/** The game's id. It names the logger, the achievements' saved progress and the settings. */
export const GAME_ID = "hello-2d";

/** The logical surface, in pixels. Every position in the menus is written in these units. */
export const SURFACE = { width: 1920, height: 1080 } as const;

/** The loop runs this many fixed steps a second. The world moves by one step at a time. */
export const STEPS_PER_SECOND = 60;

/** The catalog files of each language, in name order (docs/formats/LOCALE.md). */
export const LOCALE_URLS: Readonly<Record<Language, readonly string[]>> = {
  es: ["data/locales/es/00_ui.json", "data/locales/es/10_game.json"],
  en: ["data/locales/en/00_ui.json", "data/locales/en/10_game.json"],
};

/** The achievements of the game, in the file format of docs/formats/ACHIEVEMENTS.md. */
export const ACHIEVEMENTS_URL = "data/achievements.json";

/** The level: a Tiled map, and the folder that its external tileset is read from. */
export const LEVEL_URL = "maps/level1.tmj";
export const MAPS_FOLDER = "maps/";

/** The pictures and the sprite sheets. */
export const ART_URLS = {
  tiles: "art/tiles.png",
  hero: "art/hero.png",
  heroAtlas: "art/hero.json",
  coins: "art/coins.png",
  coinSheet: "art/coins.json",
} as const;

/** The name of the tileset in the map, which is the key of its picture when the level is drawn. */
export const TILESET_NAME = "tiles";

/** The tile ids of the tileset that the game draws by itself: the coin in the HUD and the flag at the goal. */
export const TILE_IDS = { coin: 5, flag: 6 } as const;

/** The layers of the map, by name. Collision and drawing read them by these names. */
export const LAYERS = {
  background: "background",
  ground: "ground",
  platforms: "platforms",
  hazards: "hazards",
  objects: "objects",
} as const;

/** The audio manifest, in the audio folder, and the sounds the game plays, by id. */
export const AUDIO_MANIFEST_URL = "audio/manifest.json";
export const SOUNDS = {
  jump: "sfx.jump",
  coin: "sfx.coin",
  hurt: "sfx.hurt",
  win: "sfx.win",
  click: "ui.click",
} as const;

/** The counters of the achievements, and the achievements that the game unlocks by itself. */
export const COUNTERS = { coins: "coins", runCoins: "run_coins" } as const;
export const ACHIEVEMENTS = {
  firstCoin: "first_coin",
  allCoins: "all_coins",
  cleanFinish: "clean_finish",
  speedrun: "speedrun",
} as const;

/** The most seconds a run may take and still earn the speedrun achievement (its text says 45). */
export const SPEEDRUN_SECONDS = 45;

/** The hero's box, in level pixels, and the picture that it is drawn with (a square of this many pixels). */
export const HERO = { width: 20, height: 28, frame: 32 } as const;

/**
 * The motion of the hero, in level pixels and seconds. The jump is about three tiles high (600 squared over
 * twice the gravity); letting go early cuts the rise to `cutSpeed`. The coyote time lets a jump start a little
 * after the hero walks off a ledge, and the jump buffer keeps a press that comes just before a landing.
 */
export const PHYSICS = {
  run: 210,
  groundAccel: 2400,
  airAccel: 1400,
  gravity: 1800,
  maxFall: 760,
  jumpSpeed: 600,
  cutSpeed: 220,
  coyote: 0.1,
  jumpBuffer: 0.12,
} as const;

/** How long the hero waits after a hit before it reappears at the last checkpoint. */
export const RESPAWN_SECONDS = 0.7;

/** The boxes of the things that the hero touches: a coin, a checkpoint pennant and the goal flag. */
export const COIN_BOX = { width: 16, height: 16 } as const;
export const CHECKPOINT_BOX = { width: 24, height: 56 } as const;
export const GOAL_BOX = { width: 32, height: 64 } as const;

/** The camera: the view zooms in so that tiles are big on the surface, and it follows the hero with a deadzone. */
export const CAMERA = {
  zoom: 2,
  lerp: 0.12,
  deadzone: { width: 160, height: 96 },
  shake: { intensity: 10, seconds: 0.3 },
} as const;

/** The colour of the sky behind the scenery, and the length of the red flash after a hit. */
export const SKY = "#8fd3ff";
export const HURT_FLASH_SECONDS = 0.45;

/** The settings: a namespace of their own, a version, and the defaults. The effects volume runs from 0 to 10. */
export const SETTINGS = { namespace: GAME_ID, version: 1 } as const;
export const SETTINGS_DEFAULTS = { sfxVolume: 8, language: "es" } as const;
export const VOLUME_STEPS = 10;

/** The notices: how many show at once, and how long each stays. */
export const NOTICE = { maxVisible: 3, durationMs: 2600, fadeMs: 300 } as const;
