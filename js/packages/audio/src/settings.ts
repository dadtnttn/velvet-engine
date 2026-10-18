// Saved settings of the mix: the volume of each bus and the buses that are muted. In Velvet Desktop
// the bridge keeps them; in a browser, localStorage does; otherwise they last for the session only.
// A value that is not what this build expects is ignored, so an old or damaged setting never stops
// the game from starting.

import { getBridge, getLocalStorage, type StorageLike } from "@velvet/core";
import type { AudioSettingsStore } from "./api";
import { isRecord } from "./guards";

/** The key under which the mix is stored. */
export const AUDIO_SETTINGS_KEY = "audio";

/** The start of each localStorage key, so that the audio settings do not clash with the game's. */
const STORAGE_PREFIX = "velvet-audio:";

/** The mix as it is saved: a volume per bus (master included), and the names of the muted buses. */
export interface StoredMix {
  /** The volume of each bus, from 0 to 1. */
  readonly volumes: Readonly<Record<string, number>>;
  /** The names of the buses that are muted. */
  readonly muted: readonly string[];
}

/**
 * Reads a saved mix. Numbers are clamped to 0 to 1, and anything of the wrong type is dropped. The
 * names are not checked here: the engine ignores the buses it does not have.
 * @param value - what the settings store returned for the audio key.
 */
export function readStoredMix(value: unknown): StoredMix {
  const volumes: Record<string, number> = {};
  const muted: string[] = [];
  if (!isRecord(value)) return { volumes, muted };
  const storedVolumes = value.volumes;
  if (isRecord(storedVolumes)) {
    for (const [name, volume] of Object.entries(storedVolumes)) {
      if (typeof volume === "number" && Number.isFinite(volume)) {
        volumes[name] = Math.min(1, Math.max(0, volume));
      }
    }
  }
  if (Array.isArray(value.muted)) {
    const items: readonly unknown[] = value.muted;
    for (const item of items) {
      if (typeof item === "string" && !muted.includes(item)) muted.push(item);
    }
  }
  return { volumes, muted };
}

/**
 * A settings store that keeps each value as JSON in web storage. A read that fails, or a value that
 * is not JSON, gives null; a write that the browser refuses is dropped.
 * @param storage - the storage to write to, such as `window.localStorage`.
 */
export function localStorageStore(storage: StorageLike): AudioSettingsStore {
  return {
    async get(key: string): Promise<unknown> {
      try {
        const text = storage.getItem(STORAGE_PREFIX + key);
        if (text === null) return null;
        const value: unknown = JSON.parse(text);
        return value;
      } catch {
        return null;
      }
    },
    async set(key: string, value: unknown): Promise<void> {
      try {
        storage.setItem(STORAGE_PREFIX + key, JSON.stringify(value));
      } catch {
        // The browser refused the write (full or blocked storage): the mix lasts for this session.
      }
    },
  };
}

/**
 * The settings store of this runtime: the bridge's settings in Velvet Desktop, else localStorage in
 * a browser, else null (no saving).
 */
export function defaultSettingsStore(): AudioSettingsStore | null {
  const bridge = getBridge();
  if (bridge !== null) return bridge.settings;
  const storage = getLocalStorage();
  return storage === null ? null : localStorageStore(storage);
}
