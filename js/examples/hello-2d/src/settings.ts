// The settings of hello-2d: the volume of the sound effects and the language. They are kept between runs by the
// core's settings store, under the game's own namespace, and the settings panel of @velvet/ui edits them.

import { createSettingsStore, type SettingsBackend, type SettingsStore } from "@velvet/core";
import { LANGUAGES, SETTINGS, SETTINGS_DEFAULTS, VOLUME_STEPS, type Language } from "./config";
import { asLanguage } from "./texts";

/** What the game keeps between runs. */
export interface GameSettings {
  /** The level of the sound effects, from 0 (silent) to 10 (full). The settings panel shows it as a percentage. */
  readonly sfxVolume: number;
  /** The language of the texts. */
  readonly language: Language;
}

/** The store of the settings. Call `load` before reading them. */
export type GameSettingsStore = SettingsStore<GameSettings>;

/**
 * Makes the store of the settings. Without a backend the values go to the bridge of Velvet Desktop, to localStorage in a
 * browser, and to memory otherwise.
 * @param backend - where the values are kept, for tests and custom hosts.
 */
export function createGameSettings(backend?: SettingsBackend): GameSettingsStore {
  const options = {
    namespace: SETTINGS.namespace,
    version: SETTINGS.version,
    defaults: { sfxVolume: SETTINGS_DEFAULTS.sfxVolume, language: SETTINGS_DEFAULTS.language },
    choices: { language: LANGUAGES },
  };
  return createSettingsStore<GameSettings>(backend === undefined ? options : { ...options, backend });
}

/**
 * Reads the settings of a store. A value that is out of its range is replaced by the default, so a damaged setting
 * never reaches the sound or the screen.
 * @param store - the settings store, loaded or not.
 */
export function currentSettings(store: GameSettingsStore): GameSettings {
  const volume = store.get("sfxVolume");
  const sfxVolume = Number.isInteger(volume) && volume >= 0 && volume <= VOLUME_STEPS ? volume : SETTINGS_DEFAULTS.sfxVolume;
  return { sfxVolume, language: asLanguage(store.get("language")) ?? "es" };
}

/**
 * The gain of the sound effects for a setting: 0 to 1, with the curve of the audio engine applied by the engine itself.
 * @param sfxVolume - the setting, from 0 to 10.
 */
export function effectsLevel(sfxVolume: number): number {
  return sfxVolume / VOLUME_STEPS;
}
