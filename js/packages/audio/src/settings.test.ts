// Tests for the saved mix: the volumes and mutes are read at start, written after changes, and a
// value that is not what this build expects is ignored. Also the stores that save them: the bridge in
// Velvet Desktop, localStorage in a browser, and nothing at all.

import { afterEach, describe, expect, it, vi } from "vitest";
import { AudioError } from "./error";
import type { AudioSettingsStore } from "./api";
import { AUDIO_SETTINGS_KEY, defaultSettingsStore, localStorageStore, readStoredMix } from "./settings";
import { createMemorySettings, createRig, settle, thrownBy } from "./test-support";

const door = { id: "sfx.door", files: ["sfx/door.opus"] };

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("reading the saved mix", () => {
  it("applies the saved volumes and mutes once the store has answered", async () => {
    const memory = createMemorySettings({
      [AUDIO_SETTINGS_KEY]: { volumes: { master: 0.5, music: 0.4 }, muted: ["sfx"] },
    });
    const rig = createRig({ sounds: [door], settings: memory.store });
    await rig.audio.ready;
    expect(rig.audio.getVolume("master")).toBe(0.5);
    expect(rig.audio.getVolume("music")).toBe(0.4);
    expect(rig.audio.isMuted("sfx")).toBe(true);
    expect(rig.audio.getVolume("voice")).toBe(1);
  });

  it("keeps a value that the game set before the saved mix was read", async () => {
    const memory = createMemorySettings({ [AUDIO_SETTINGS_KEY]: { volumes: { music: 0.4 }, muted: ["voice"] } });
    const rig = createRig({ sounds: [door], settings: memory.store });
    rig.audio.setVolume("music", 0.9);
    rig.audio.setMuted("voice", false);
    await rig.audio.ready;
    expect(rig.audio.getVolume("music")).toBe(0.9);
    expect(rig.audio.isMuted("voice")).toBe(false);
  });

  it("ignores buses the manifest does not have, bad numbers and bad names", async () => {
    const memory = createMemorySettings({
      [AUDIO_SETTINGS_KEY]: { volumes: { battle: 0.2, sfx: "loud", ui: 3 }, muted: [7, "sfx"] },
    });
    const rig = createRig({ sounds: [door], settings: memory.store });
    await rig.audio.ready;
    expect(rig.audio.getVolume("sfx")).toBe(1);
    expect(rig.audio.getVolume("ui")).toBe(1);
    expect(rig.audio.isMuted("sfx")).toBe(true);
  });

  it("starts with the defaults when the store cannot be read", async () => {
    const store: AudioSettingsStore = {
      get: () => Promise.reject(new Error("the bridge is closed")),
      set: async () => undefined,
    };
    const rig = createRig({ sounds: [door], settings: store });
    await expect(rig.audio.ready).resolves.toBeUndefined();
    expect(rig.audio.getVolume("master")).toBe(1);
  });

  it("keeps the mix for the session when there is no store", () => {
    const rig = createRig({ sounds: [door], settings: null });
    rig.audio.setVolume("sfx", 0.2);
    expect(rig.audio.getVolume("sfx")).toBe(0.2);
  });
});

describe("writing the saved mix", () => {
  it("writes the whole mix once, after the saved mix has been read", async () => {
    const memory = createMemorySettings();
    const rig = createRig({ sounds: [door], settings: memory.store });
    rig.audio.setVolume("sfx", 0.2);
    rig.audio.setVolume("sfx", 0.3);
    rig.audio.setMuted("ui", true);
    await rig.audio.ready;
    await settle();
    expect(memory.writes).toBe(1);
    const saved = memory.values.get(AUDIO_SETTINGS_KEY);
    expect(saved).toEqual({
      volumes: { master: 1, music: 1, sfx: 0.3, voice: 1, ui: 1 },
      muted: ["ui"],
    });
  });

  it("writes the mute of the master", async () => {
    const memory = createMemorySettings();
    const rig = createRig({ sounds: [door], settings: memory.store });
    rig.audio.setMuted("master", true);
    await settle();
    expect(memory.values.get(AUDIO_SETTINGS_KEY)).toMatchObject({ muted: ["master"] });
  });

  it("refuses a bus that does not exist, and a volume out of range", () => {
    const rig = createRig({ sounds: [door], settings: null });
    const error = thrownBy(() => rig.audio.setVolume("battle", 0.5));
    expect(error instanceof AudioError ? error.code : null).toBe("unknown-bus");
    expect(() => rig.audio.setVolume("sfx", 1.5)).toThrow(RangeError);
    expect(() => rig.audio.isMuted("battle")).toThrow();
  });
});

describe("readStoredMix", () => {
  it("clamps numbers to 0 to 1 and drops values of the wrong type", () => {
    expect(readStoredMix({ volumes: { a: 2, b: -1, c: "x", d: 0.25 }, muted: ["a", 1, "a"] })).toEqual({
      volumes: { a: 1, b: 0, d: 0.25 },
      muted: ["a"],
    });
  });

  it("gives empty values for anything that is not a saved mix", () => {
    expect(readStoredMix(null)).toEqual({ volumes: {}, muted: [] });
    expect(readStoredMix([1, 2])).toEqual({ volumes: {}, muted: [] });
  });
});

describe("localStorageStore", () => {
  /** A minimal storage in memory, with the methods of Web Storage. */
  function memoryStorage(): Storage {
    const items = new Map<string, string>();
    return {
      get length() {
        return items.size;
      },
      key: (index: number) => [...items.keys()][index] ?? null,
      getItem: (key: string) => items.get(key) ?? null,
      setItem: (key: string, value: string) => {
        items.set(key, value);
      },
      removeItem: (key: string) => {
        items.delete(key);
      },
      clear: () => items.clear(),
    };
  }

  it("keeps each value as JSON under a prefixed key, and reads it back", async () => {
    const storage = memoryStorage();
    const store = localStorageStore(storage);
    await store.set("audio", { muted: ["sfx"] });
    expect(storage.getItem("velvet-audio:audio")).toBe('{"muted":["sfx"]}');
    await expect(store.get("audio")).resolves.toEqual({ muted: ["sfx"] });
    await expect(store.get("missing")).resolves.toBeNull();
  });

  it("reads a value that is not JSON as null, and survives a refused write", async () => {
    const storage = memoryStorage();
    storage.setItem("velvet-audio:audio", "{ broken");
    const store = localStorageStore(storage);
    await expect(store.get("audio")).resolves.toBeNull();
    const refusing: Storage = { ...storage, setItem: () => { throw new Error("quota exceeded"); } };
    await expect(localStorageStore(refusing).set("audio", {})).resolves.toBeUndefined();
  });
});

describe("defaultSettingsStore", () => {
  it("is null without a bridge or localStorage", () => {
    vi.stubGlobal("velvet", undefined);
    vi.stubGlobal("localStorage", undefined);
    expect(defaultSettingsStore()).toBeNull();
  });

  it("uses the bridge's settings in Velvet Desktop", () => {
    const memory = createMemorySettings();
    vi.stubGlobal("velvet", fakeBridge(memory.store));
    expect(defaultSettingsStore()).toBe(memory.store);
  });

  it("uses localStorage in a browser", async () => {
    const items = new Map<string, string>();
    const storage: Storage = {
      length: 0,
      key: () => null,
      getItem: (key: string) => items.get(key) ?? null,
      setItem: (key: string, value: string) => {
        items.set(key, value);
      },
      removeItem: () => undefined,
      clear: () => undefined,
    };
    vi.stubGlobal("localStorage", storage);
    const store = defaultSettingsStore();
    expect(store).not.toBeNull();
    await store?.set("audio", { volumes: { sfx: 0.5 } });
    expect(items.get("velvet-audio:audio")).toBe('{"volumes":{"sfx":0.5}}');
  });
});

/** A bridge with every member of the contract, whose settings are the given store. */
function fakeBridge(settings: AudioSettingsStore) {
  const files = new Map<string, string>();
  return {
    platform: "desktop" as const,
    game: { id: "demo", title: "Demo", version: "0.1.0" },
    saves: {
      list: async () => [...files.keys()],
      read: async (slot: string) => files.get(slot) ?? null,
      write: async (slot: string, text: string) => {
        files.set(slot, text);
      },
      remove: async (slot: string) => {
        files.delete(slot);
      },
    },
    settings,
    window: {
      setFullscreen: async () => undefined,
      isFullscreen: async () => false,
      close: async () => undefined,
    },
    log: () => undefined,
  };
}
