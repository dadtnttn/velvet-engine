// Test doubles shared by the save and bridge tests: an in-memory stand-in for Web Storage,
// a fake Velvet Desktop bridge, and a helper that captures the error an action throws.
// Only the tests import this file. The package index does not export it.

import type { VelvetBridge } from "./bridge";
import type { StorageLike } from "./save-backends";

/** An in-memory object with the Web Storage methods that the localStorage backend uses. */
export class FakeStorage implements StorageLike {
  private readonly items = new Map<string, string>();

  get length(): number {
    return this.items.size;
  }

  key(index: number): string | null {
    return [...this.items.keys()][index] ?? null;
  }

  getItem(key: string): string | null {
    return this.items.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    this.items.set(key, value);
  }

  removeItem(key: string): void {
    this.items.delete(key);
  }

  /** Every raw key in the storage, in insertion order. */
  keys(): string[] {
    return [...this.items.keys()];
  }
}

/** A fake bridge, with the save texts that it holds. */
export interface FakeBridge {
  readonly bridge: VelvetBridge;
  readonly files: Map<string, string>;
}

/** Builds a bridge that keeps its save files in memory. */
export function createFakeBridge(): FakeBridge {
  const files = new Map<string, string>();
  const settings = new Map<string, unknown>();
  const bridge: VelvetBridge = {
    platform: "desktop",
    game: { id: "demo", title: "Demo", version: "0.1.0" },
    saves: {
      list: async () => [...files.keys()],
      read: async (slot) => files.get(slot) ?? null,
      write: async (slot, text) => {
        files.set(slot, text);
      },
      remove: async (slot) => {
        files.delete(slot);
      },
    },
    settings: {
      get: async (key) => settings.get(key),
      set: async (key, value) => {
        settings.set(key, value);
      },
    },
    window: {
      setFullscreen: async () => undefined,
      isFullscreen: async () => false,
      close: async () => undefined,
    },
    log: () => undefined,
  };
  return { bridge, files };
}

/**
 * Runs `action` and returns the error that it throws, or that its promise rejects with.
 * Fails the test when nothing is thrown.
 */
export async function errorOf(action: () => unknown): Promise<unknown> {
  try {
    await action();
  } catch (error) {
    return error;
  }
  throw new Error("expected the action to throw, but it returned normally");
}
