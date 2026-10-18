// The settings of hello-vn: the text speed, the wait of automatic reading, the volumes and the language.
// They are kept between runs by @velvet/core's settings store, under the game's own namespace.

import { createSettingsStore, type SettingsBackend, type SettingsStore } from "@velvet/core";
import { SETTINGS_DEFAULTS, SETTINGS_NAMESPACE, SETTINGS_VERSION, type Language } from "./config";

/** What the game keeps between runs. A saved value replaces a default only when it has the right type. */
export interface GameSettings {
  /** Letters a second; 0 writes each line at once. */
  readonly textSpeed: number;
  /** Seconds that a finished line waits before automatic reading goes on. */
  readonly autoDelay: number;
  /** The level of the music, from 0 to 1. */
  readonly musicVolume: number;
  /** The level of the sound effects, from 0 to 1. */
  readonly sfxVolume: number;
  /** The language of the texts, or null when none was chosen (the game then starts in Spanish). */
  readonly language: Language | null;
}

/** The store of the game's settings. Call `load` before reading them. */
export type GameSettingsStore = SettingsStore<GameSettings>;

/** The defaults as a plain settings object. */
export function defaultSettings(): GameSettings {
  return { ...SETTINGS_DEFAULTS, language: SETTINGS_DEFAULTS.language };
}

/**
 * Makes the store of the game's settings. Without a backend, the values go to the bridge's settings in Velvet
 * Desktop, to localStorage in a browser, and to memory otherwise.
 * @param backend - where the values are kept, for tests and custom hosts.
 */
export function createGameSettings(backend?: SettingsBackend): GameSettingsStore {
  const options = {
    namespace: SETTINGS_NAMESPACE,
    version: SETTINGS_VERSION,
    defaults: defaultSettings(),
  };
  return createSettingsStore<GameSettings>(backend === undefined ? options : { ...options, backend });
}

/**
 * Reads the settings of a store, and replaces a value that the store holds out of its range with the default.
 * A damaged number never reaches the screen or the sound.
 * @param store - the settings store, loaded or not.
 */
export function currentSettings(store: GameSettingsStore): GameSettings {
  const defaults = defaultSettings();
  const speed = store.get("textSpeed");
  const delay = store.get("autoDelay");
  const music = store.get("musicVolume");
  const sfx = store.get("sfxVolume");
  return {
    textSpeed: Number.isFinite(speed) && speed >= 0 ? speed : defaults.textSpeed,
    autoDelay: Number.isFinite(delay) && delay >= 0 ? delay : defaults.autoDelay,
    musicVolume: Number.isFinite(music) && music >= 0 && music <= 1 ? music : defaults.musicVolume,
    sfxVolume: Number.isFinite(sfx) && sfx >= 0 && sfx <= 1 ? sfx : defaults.sfxVolume,
    language: store.get("language"),
  };
}

/**
 * The next step of a list of steps, wrapping after the last one. A value that is not on the list (one from
 * another build) moves to the first step above it.
 * @param steps - the steps, in order.
 * @param current - the value now.
 */
export function nextStep(steps: readonly number[], current: number): number {
  const index = steps.indexOf(current);
  if (index >= 0) return steps[(index + 1) % steps.length] ?? current;
  return steps.find((step) => step > current) ?? steps[0] ?? current;
}
