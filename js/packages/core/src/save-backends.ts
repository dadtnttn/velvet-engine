// Where save texts are kept. A backend stores the JSON text of each slot under its slot
// name. The save store above it adds the envelope and the checks.
//
// Three backends exist. The Velvet Desktop bridge is used in the desktop release. localStorage
// serves browser development. Memory serves tests and throwaway runs. The bridge's `saves`
// object already has the backend shape, so it is used as it is.

import type { VelvetBridge } from "./bridge";

/** Stores save texts by slot name. Every method is asynchronous. */
export type SaveBackend = VelvetBridge["saves"];

/** The part of the Web Storage API that the localStorage backend uses. */
export type StorageLike = Pick<Storage, "getItem" | "setItem" | "removeItem" | "key" | "length">;

/** Starts every localStorage key. The full key is this prefix, the game id, a colon and the slot. */
const KEY_PREFIX = "velvet-save:";

/** The backend of a running Velvet Desktop: the bridge's own `saves` object. */
export function createBridgeBackend(bridge: VelvetBridge): SaveBackend {
  return bridge.saves;
}

/** A backend that keeps saves in memory. Nothing survives a reload. */
export function createMemoryBackend(): SaveBackend {
  const texts = new Map<string, string>();
  return {
    async list() {
      return [...texts.keys()];
    },
    async read(slot) {
      return texts.get(slot) ?? null;
    },
    async write(slot, text) {
      texts.set(slot, text);
    },
    async remove(slot) {
      texts.delete(slot);
    },
  };
}

/**
 * A backend on top of localStorage, or any object with the same methods. Keys carry the game
 * id, so two games on one origin never see each other's saves.
 */
export function createLocalStorageBackend(storage: StorageLike, game: string): SaveBackend {
  const prefix = `${KEY_PREFIX}${game}:`;
  return {
    async list() {
      const slots: string[] = [];
      for (let index = 0; index < storage.length; index += 1) {
        const key = storage.key(index);
        if (key !== null && key.startsWith(prefix)) slots.push(key.slice(prefix.length));
      }
      return slots;
    },
    async read(slot) {
      return storage.getItem(prefix + slot);
    },
    async write(slot, text) {
      storage.setItem(prefix + slot, text);
    },
    async remove(slot) {
      storage.removeItem(prefix + slot);
    },
  };
}

/**
 * The browser's localStorage, when it can be used. Returns null when the global is missing,
 * when the browser blocks storage (reading it throws), or when the object lacks the storage
 * methods (Node.js provides a placeholder).
 */
export function getLocalStorage(): StorageLike | null {
  try {
    const candidate: unknown = Reflect.get(globalThis, "localStorage");
    return isStorageLike(candidate) ? candidate : null;
  } catch {
    return null;
  }
}

/** True when `value` has the storage methods and a numeric length. */
function isStorageLike(value: unknown): value is StorageLike {
  if (typeof value !== "object" || value === null) return false;
  const methodsPresent = ["getItem", "setItem", "removeItem", "key"].every(
    (name) => typeof Reflect.get(value, name) === "function",
  );
  return methodsPresent && typeof Reflect.get(value, "length") === "number";
}
