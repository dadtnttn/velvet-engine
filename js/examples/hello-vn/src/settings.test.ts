// Tests of the settings: the defaults, the persistence through a backend, a damaged value that does not stop
// the game, and the steps that the settings menu cycles through.

import { describe, expect, it } from "vitest";
import { SETTINGS_DEFAULTS } from "./config";
import { createGameSettings, currentSettings, defaultSettings, nextStep } from "./settings";
import { memorySettings } from "./test-support";

describe("the settings", () => {
  it("starts with the defaults when nothing is saved", async () => {
    const backend = memorySettings();
    const store = createGameSettings(backend);
    await store.load();
    expect(currentSettings(store)).toEqual(defaultSettings());
    expect(defaultSettings().textSpeed).toBe(SETTINGS_DEFAULTS.textSpeed);
  });

  it("keeps what the player set, and reads it back in a new store", async () => {
    const backend = memorySettings();
    const first = createGameSettings(backend);
    await first.load();
    first.set("textSpeed", 80);
    first.set("autoDelay", 3);
    first.set("musicVolume", 0.25);
    first.set("sfxVolume", 0);
    first.set("language", "en");
    expect(await first.save()).toBe(true);

    const second = createGameSettings(backend);
    await second.load();
    expect(currentSettings(second)).toEqual({
      textSpeed: 80,
      autoDelay: 3,
      musicVolume: 0.25,
      sfxVolume: 0,
      language: "en",
    });
  });

  it("ignores a saved value of the wrong type, and keeps the others", async () => {
    const backend = memorySettings();
    await backend.set("hello-vn", {
      format: "velvet-settings",
      version: 1,
      values: { textSpeed: "fast", musicVolume: 0.8 },
    });
    const store = createGameSettings(backend);
    await store.load();
    const values = currentSettings(store);
    expect(values.textSpeed).toBe(SETTINGS_DEFAULTS.textSpeed);
    expect(values.musicVolume).toBe(0.8);
  });

  it("replaces a number out of range with its default, so the dialogue and the sound stay valid", async () => {
    const backend = memorySettings();
    await backend.set("hello-vn", {
      format: "velvet-settings",
      version: 1,
      values: { textSpeed: -5, autoDelay: Number.NaN, musicVolume: 3, sfxVolume: -1 },
    });
    const store = createGameSettings(backend);
    await store.load();
    const values = currentSettings(store);
    expect(values.textSpeed).toBe(SETTINGS_DEFAULTS.textSpeed);
    expect(values.autoDelay).toBe(SETTINGS_DEFAULTS.autoDelay);
    expect(values.musicVolume).toBe(SETTINGS_DEFAULTS.musicVolume);
    expect(values.sfxVolume).toBe(SETTINGS_DEFAULTS.sfxVolume);
  });
});

describe("the steps of the settings menu", () => {
  it("moves to the next step and wraps after the last one", () => {
    expect(nextStep([0, 20, 40], 0)).toBe(20);
    expect(nextStep([0, 20, 40], 40)).toBe(0);
  });

  it("moves a value that is not on the list to the first step above it", () => {
    expect(nextStep([0, 20, 40], 25)).toBe(40);
    expect(nextStep([0, 20, 40], 99)).toBe(0);
  });
});
