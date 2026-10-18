// The game's settings in a browser: the language that the player chose is still there after a restart,
// kept in localStorage under the game's namespace, and nothing else is written.

import { afterEach, describe, expect, it, vi } from "vitest";
import { createGameSettings } from "./platform";

/** An in-memory storage with the methods of Web Storage. */
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

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("the game's settings in a browser", () => {
  it("keeps the language between runs, under the game's namespace in localStorage", async () => {
    vi.stubGlobal("velvet", undefined);
    const storage = memoryStorage();
    vi.stubGlobal("localStorage", storage);

    const first = createGameSettings();
    await first.load();
    expect(first.get("language")).toBeNull();

    first.set("language", "en");
    await expect(first.save()).resolves.toBe(true);
    expect(storage.getItem("velvet-settings:hello-cards")).toContain('"language":"en"');

    const second = createGameSettings();
    await second.load();
    expect(second.get("language")).toBe("en");
  });
});
