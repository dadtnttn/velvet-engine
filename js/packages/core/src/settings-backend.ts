// Where settings values are kept: the settings of the Velvet Desktop bridge, localStorage (as JSON
// text), or memory. A backend stores one JSON value under each key. The bindings store and the
// settings store both choose their backend here, so the order of the choice and its fallbacks are
// the same everywhere.
//
// The backends do not catch errors. A storage that refuses a write makes the write reject, and the
// caller decides what that means: the settings store reports it as a result, and the bindings store
// lets it reach the game.

import { getBridge } from "./bridge";
import { getLocalStorage, type StorageLike } from "./save-backends";

/** Stores JSON values by key. The bridge's `settings` object has this shape. */
export interface SettingsBackend {
  /** Reads a value. Resolves to null (or undefined) when nothing is stored under the key. */
  get(key: string): Promise<unknown>;
  /** Stores a JSON value under the key. Storing null clears it. */
  set(key: string, value: unknown): Promise<void>;
}

/** Where a backend keeps its values: the bridge, localStorage, memory, or a backend that the game passed in. */
export type SettingsBackendKind = "bridge" | "localStorage" | "memory" | "custom";

/**
 * What {@link chooseSettingsBackend} may use. An option that is left out means the runtime's own choice;
 * `null` means that the runtime's choice is not wanted.
 */
export interface SettingsBackendChoice {
  /** A backend that the game passes in. It is used before anything else. */
  readonly explicit?: SettingsBackend;
  /** The bridge whose settings hold the values. Default: the bridge of Velvet Desktop, when it is there. */
  readonly bridge?: { readonly settings: SettingsBackend } | null;
  /** The web storage. Default: localStorage, when the browser allows it. */
  readonly storage?: StorageLike | null;
  /** The prefix of the localStorage keys, which keeps one package's values apart from another's. */
  readonly prefix: string;
}

/** The backend that {@link chooseSettingsBackend} picked, and its kind. */
export interface ChosenSettingsBackend {
  readonly kind: SettingsBackendKind;
  readonly backend: SettingsBackend;
}

/**
 * Picks the backend: the one passed in, else the bridge's settings, else localStorage, else memory. Choosing
 * touches no storage.
 */
export function chooseSettingsBackend(choice: SettingsBackendChoice): ChosenSettingsBackend {
  if (choice.explicit !== undefined) return { kind: "custom", backend: choice.explicit };
  const bridge = choice.bridge === undefined ? getBridge() : choice.bridge;
  if (bridge !== null) return { kind: "bridge", backend: bridge.settings };
  const storage = choice.storage === undefined ? getLocalStorage() : choice.storage;
  if (storage !== null) return { kind: "localStorage", backend: createLocalStorageSettings(storage, choice.prefix) };
  return { kind: "memory", backend: createMemorySettings() };
}

/**
 * Settings kept as JSON text in web storage, under the prefix and the key. A text that is not JSON is
 * returned as it is, so that the reader can tell it is damaged.
 */
export function createLocalStorageSettings(storage: StorageLike, prefix: string): SettingsBackend {
  return {
    async get(key) {
      const text = storage.getItem(prefix + key);
      if (text === null) return null;
      try {
        const parsed: unknown = JSON.parse(text);
        return parsed;
      } catch {
        return text;
      }
    },
    async set(key, value) {
      storage.setItem(prefix + key, JSON.stringify(value));
    },
  };
}

/** Settings kept in memory. Nothing survives a reload. */
export function createMemorySettings(): SettingsBackend {
  const values = new Map<string, unknown>();
  return {
    async get(key) {
      return values.get(key) ?? null;
    },
    async set(key, value) {
      values.set(key, value);
    },
  };
}
