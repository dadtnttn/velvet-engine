// Saved settings of the mix: the volume of each bus and the buses that are muted. The mix is kept by the
// core settings store, under the namespace "audio": in Velvet Desktop in the bridge's settings, in a browser
// in localStorage, and otherwise for the session only. A value that is not what this build expects is
// ignored, so an old or damaged setting never stops the game from starting.

import { SETTINGS_STORAGE_PREFIX, chooseSettingsBackend, createSettingsStore, type SettingsStore } from "@velvet/core";
import type { AudioSettingsStore } from "./api";
import { isRecord } from "./guards";

/** The namespace of the saved mix. It is also the key that holds the mix in the bridge's settings. */
export const AUDIO_SETTINGS_KEY = "audio";

/** The version of the saved mix. A plain object without the store's envelope is read as version 1. */
const MIX_VERSION = 1;

/** The mix as it is saved: a volume per bus (master included), and the names of the muted buses. */
export interface StoredMix {
  /** The volume of each bus, from 0 to 1. */
  readonly volumes: Readonly<Record<string, number>>;
  /** The names of the buses that are muted. */
  readonly muted: readonly string[];
}

/** The values that the mix store keeps, one key each. */
export interface MixValues {
  readonly volumes: Readonly<Record<string, number>>;
  readonly muted: readonly string[];
}

/**
 * The store of the mix. With no settings given it uses the runtime's own store: the bridge's settings in
 * Velvet Desktop, else localStorage, else memory. A custom store is used as it is, and null keeps the mix
 * for this session only.
 * @param settings - the custom store, null for none, or undefined for the runtime's choice.
 */
export function createMixStore(settings: AudioSettingsStore | null | undefined): SettingsStore<MixValues> {
  const defaults: MixValues = { volumes: {}, muted: [] };
  const base = { namespace: AUDIO_SETTINGS_KEY, version: MIX_VERSION, defaults };
  if (settings === null) return createSettingsStore({ ...base, bridge: null, storage: null });
  if (settings !== undefined) return createSettingsStore({ ...base, backend: settings });
  return createSettingsStore(base);
}

/**
 * Reads a saved mix. Numbers are clamped to 0 to 1, and anything of the wrong type is dropped. The
 * names are not checked here: the engine ignores the buses it does not have.
 * @param value - an object with the saved `volumes` and `muted`, as the store gives them.
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
 * The settings store of this runtime: the bridge's settings in Velvet Desktop, else localStorage in a
 * browser, else null (no saving). It keeps the values under the same keys as the engine's own store.
 */
export function defaultSettingsStore(): AudioSettingsStore | null {
  const chosen = chooseSettingsBackend({ prefix: SETTINGS_STORAGE_PREFIX });
  return chosen.kind === "memory" ? null : chosen.backend;
}
