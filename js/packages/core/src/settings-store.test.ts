// Tests for the settings store: the defaults and the typed reads, the saved values that load in and their
// types, the envelope and its migrations, the debounced and the immediate writes, the subscriptions, the
// fallbacks (the bridge, localStorage, memory and a storage that refuses), and the checks of the options.

import { afterEach, describe, expect, it, vi } from "vitest";
import type { SettingsBackend } from "./settings-backend";
import { createSettingsStore, SETTINGS_FORMAT, type SettingsStore, type SettingsStoreOptions } from "./settings-store";
import type { MigrationMap } from "./save-format";
import type { StorageLike } from "./save-backends";
import { FakeStorage, createFakeBridge } from "./test-support";

interface Prefs {
  volume: number;
  language: string | null;
  muted: boolean;
  keys: string[];
}

const defaults: Prefs = { volume: 1, language: null, muted: false, keys: [] };

/** A backend that keeps its values in a map, and records every write. */
function memoryBackend(initial: Record<string, unknown> = {}) {
  const values = new Map<string, unknown>(Object.entries(initial));
  const writes: Array<{ key: string; value: unknown }> = [];
  const backend: SettingsBackend = {
    get: async (key) => values.get(key) ?? null,
    set: async (key, value) => {
      writes.push({ key, value });
      values.set(key, value);
    },
  };
  return { backend, values, writes };
}

/** A saved envelope, as the store writes it. */
function envelope(version: number, values: Record<string, unknown>): Record<string, unknown> {
  return { format: SETTINGS_FORMAT, version, values };
}

/** Lets the promises that a timer or a write started run to the end. */
async function settle(): Promise<void> {
  for (let turn = 0; turn < 10; turn += 1) await Promise.resolve();
}

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("reading the saved values", () => {
  it("gives the defaults before load, and the saved values after it", async () => {
    const memory = memoryBackend({
      game: envelope(1, { volume: 0.4, language: "en", muted: true, keys: ["a"] }),
    });
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    expect(store.get("volume")).toBe(1);
    await store.load();
    expect(store.get("volume")).toBe(0.4);
    expect(store.get("language")).toBe("en");
    expect(store.get("muted")).toBe(true);
    expect(store.get("keys")).toEqual(["a"]);
    expect(store.backendKind).toBe("custom");
  });

  it("keeps the default of a value that the saved data lacks", async () => {
    const memory = memoryBackend({ game: envelope(1, { volume: 0.4 }) });
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    await store.load();
    expect(store.get("language")).toBeNull();
    expect(store.get("muted")).toBe(false);
  });

  it("ignores a saved value of the wrong type, and keeps the default", async () => {
    const memory = memoryBackend({ game: envelope(1, { volume: "loud", muted: "yes", keys: "a" }) });
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    await store.load();
    expect(store.get("volume")).toBe(1);
    expect(store.get("muted")).toBe(false);
    expect(store.get("keys")).toEqual([]);
  });

  it("takes any value for a default of null, as the default gives no type", async () => {
    const memory = memoryBackend({ game: envelope(1, { language: 3 }) });
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    await store.load();
    expect(store.get("language")).toBe(3);
  });

  it("reads a plain object saved before the envelope existed as version 1", async () => {
    const memory = memoryBackend({ game: { volume: 0.2, muted: true } });
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    await store.load();
    expect(store.get("volume")).toBe(0.2);
    expect(store.get("muted")).toBe(true);
  });

  it("brings an envelope of an older version up to date with the migrations", async () => {
    const memory = memoryBackend({ game: envelope(1, { volume: 0.5, muted: true }) });
    const migrate: MigrationMap = {
      1: (values) => ({ ...(values as Record<string, unknown>), language: "es" }),
    };
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 2, defaults, migrate, backend: memory.backend });
    await store.load();
    expect(store.get("volume")).toBe(0.5);
    expect(store.get("muted")).toBe(true);
    expect(store.get("language")).toBe("es");
  });

  it("uses the defaults when a migration step is missing, or the saved values come from a newer build", async () => {
    const older = memoryBackend({ game: envelope(1, { volume: 0.5 }) });
    const missing = createSettingsStore<Prefs>({
      namespace: "game",
      version: 3,
      defaults,
      migrate: { 2: (values) => values },
      backend: older.backend,
    });
    await missing.load();
    expect(missing.get("volume")).toBe(1);

    const newer = memoryBackend({ game: envelope(9, { volume: 0.5 }) });
    const future = createSettingsStore<Prefs>({ namespace: "game", version: 2, defaults, backend: newer.backend });
    await future.load();
    expect(future.get("volume")).toBe(1);
  });

  it("uses the defaults for a value that belongs to another format", async () => {
    const memory = memoryBackend({ game: { format: "velvet-save", version: 1, values: { volume: 0.5 } } });
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    await store.load();
    expect(store.get("volume")).toBe(1);
  });

  it("keeps the defaults when the storage refuses the read, and load still resolves", async () => {
    const backend: SettingsBackend = {
      get: () => Promise.reject(new Error("the bridge is closed")),
      set: async () => undefined,
    };
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend });
    await expect(store.load()).resolves.toBeUndefined();
    expect(store.get("volume")).toBe(1);
  });

  it("keeps a value that was set before load finished, and still loads the other saved values", async () => {
    const memory = memoryBackend({ game: envelope(1, { volume: 0.4, language: "en" }) });
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    store.set("volume", 0.9);
    await store.load();
    expect(store.get("volume")).toBe(0.9);
    expect(store.get("language")).toBe("en");
  });

  it("gives the same promise to repeated calls of load", async () => {
    const memory = memoryBackend({ game: envelope(1, { volume: 0.4 }) });
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    const first = store.load();
    expect(store.load()).toBe(first);
    await first;
  });
});

describe("writing the values", () => {
  it("saves once after the debounce, however many values were set", async () => {
    vi.useFakeTimers();
    const memory = memoryBackend();
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend, debounceMs: 250 });
    await store.load();
    store.set("volume", 0.2);
    store.set("volume", 0.3);
    store.set("muted", true);

    await vi.advanceTimersByTimeAsync(249);
    await settle();
    expect(memory.writes).toHaveLength(0);

    await vi.advanceTimersByTimeAsync(1);
    await settle();
    expect(memory.writes).toHaveLength(1);
    expect(memory.writes[0]).toEqual({
      key: "game",
      value: envelope(1, { volume: 0.3, language: null, muted: true, keys: [] }),
    });
  });

  it("saves at once when asked, and the pending debounce does not write again", async () => {
    vi.useFakeTimers();
    const memory = memoryBackend();
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    store.set("volume", 0.5);
    await expect(store.save()).resolves.toBe(true);
    expect(memory.writes).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(1000);
    await settle();
    expect(memory.writes).toHaveLength(1);
  });

  it("writes nothing when a set gives the value that the store already holds", async () => {
    vi.useFakeTimers();
    const memory = memoryBackend();
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    store.set("volume", 1);
    await vi.advanceTimersByTimeAsync(1000);
    await settle();
    expect(memory.writes).toHaveLength(0);
  });

  it("waits for the load, so a save made before it does not replace the stored values", async () => {
    const memory = memoryBackend({ game: envelope(1, { volume: 0.4, language: "en", muted: true, keys: [] }) });
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    store.set("muted", false);
    await expect(store.save()).resolves.toBe(true);
    expect(memory.writes[0]?.value).toEqual(envelope(1, { volume: 0.4, language: "en", muted: false, keys: [] }));
  });

  it("resolves false when the storage refuses the write, and never throws into the game", async () => {
    vi.useFakeTimers();
    const backend: SettingsBackend = {
      get: async () => null,
      set: () => Promise.reject(new Error("quota exceeded")),
    };
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend });
    expect(() => store.set("volume", 0.5)).not.toThrow();
    // The debounced save runs on its own timer: a refused write there must not become an unhandled rejection.
    await vi.advanceTimersByTimeAsync(1000);
    await settle();
    await expect(store.save()).resolves.toBe(false);
  });
});

describe("subscriptions", () => {
  it("runs a listener when a value changes, and not when it stays the same", () => {
    const memory = memoryBackend();
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    const seen: number[] = [];
    store.subscribe("volume", (value) => {
      seen.push(value);
    });
    store.set("volume", 0.5);
    store.set("volume", 0.5);
    store.set("volume", 0.7);
    expect(seen).toEqual([0.5, 0.7]);
  });

  it("runs a listener when load gives a value, with the value it gave", async () => {
    const memory = memoryBackend({ game: envelope(1, { volume: 0.4 }) });
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    const seen: number[] = [];
    store.subscribe("volume", (value) => {
      seen.push(value);
    });
    await store.load();
    expect(seen).toEqual([0.4]);
  });

  it("stops a listener once its unsubscribe function has run", () => {
    const memory = memoryBackend();
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    let calls = 0;
    const unsubscribe = store.subscribe("muted", () => {
      calls += 1;
    });
    store.set("muted", true);
    unsubscribe();
    store.set("muted", false);
    expect(calls).toBe(1);
  });

  it("still saves a change when a subscriber throws on it, and the error reaches the caller of set", async () => {
    vi.useFakeTimers();
    const memory = memoryBackend();
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend, debounceMs: 250 });
    store.subscribe("volume", () => {
      throw new Error("listener failed");
    });
    expect(() => store.set("volume", 0.4)).toThrow("listener failed");
    expect(store.get("volume")).toBe(0.4);
    await vi.advanceTimersByTimeAsync(300);
    await settle();
    expect(memory.writes).toHaveLength(1);
  });

  it("refuses a name that is not one of the defaults, in set and in subscribe", () => {
    const memory = memoryBackend();
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    expect(() => store.set("unknown" as keyof Prefs, 1)).toThrow(RangeError);
    expect(() => store.subscribe("unknown" as keyof Prefs & string, () => undefined)).toThrow(RangeError);
  });
});

describe("reset", () => {
  it("puts every setting back to its default, and tells the subscribers of the values that changed", () => {
    const memory = memoryBackend();
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    store.set("volume", 0.3);
    store.set("language", "es");
    const volumes: number[] = [];
    let mutedCalls = 0;
    store.subscribe("volume", (value) => {
      volumes.push(value);
    });
    store.subscribe("muted", () => {
      mutedCalls += 1;
    });
    store.reset();
    expect(store.get("volume")).toBe(1);
    expect(store.get("language")).toBeNull();
    expect(volumes).toEqual([1]);
    expect(mutedCalls).toBe(0);
  });

  it("puts one setting back when a key is given, and saves after the debounce", async () => {
    vi.useFakeTimers();
    const memory = memoryBackend();
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    store.set("volume", 0.3);
    store.set("muted", true);
    await vi.advanceTimersByTimeAsync(1000);
    const before = memory.writes.length;
    store.reset("volume");
    expect(store.get("volume")).toBe(1);
    expect(store.get("muted")).toBe(true);
    await vi.advanceTimersByTimeAsync(1000);
    await settle();
    expect(memory.writes.length).toBe(before + 1);
    expect(memory.writes.at(-1)?.value).toEqual(envelope(1, { volume: 1, language: null, muted: true, keys: [] }));
  });

  it("keeps a reset value when the saved values arrive later", async () => {
    const memory = memoryBackend({ game: envelope(1, { volume: 0.2 }) });
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    store.reset("volume");
    await store.load();
    expect(store.get("volume")).toBe(1);
  });

  it("refuses a key that is not a setting", () => {
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memoryBackend().backend });
    expect(() => store.reset("nope" as keyof Prefs)).toThrow(RangeError);
  });
});

/** Settings with a fixed set of values (a language and a speed), and a number that has no list. */
interface Choiced {
  language: string;
  speed: string;
  volume: number;
}

const choiced: Choiced = { language: "es", speed: "normal", volume: 1 };
const choices = { language: ["es", "en"], speed: ["slow", "normal", "fast"] };

describe("choices", () => {
  it("keeps a saved value that is one of its choices, and refuses one that is not, with the default", async () => {
    const memory = memoryBackend({ game: envelope(1, { language: "en", speed: "turbo", volume: 0.5 }) });
    const onInvalid = vi.fn();
    const store = createSettingsStore<Choiced>({ namespace: "game", version: 1, defaults: choiced, choices, onInvalid, backend: memory.backend });
    await store.load();
    expect(store.get("language")).toBe("en");
    expect(store.get("speed")).toBe("normal");
    expect(store.get("volume")).toBe(0.5);
    expect(onInvalid).toHaveBeenCalledTimes(1);
    expect(onInvalid).toHaveBeenCalledWith("speed", "turbo");
  });

  it("reports each refused value once, when the values are read, and not again on a second load", async () => {
    const memory = memoryBackend({ game: envelope(1, { language: "fr", speed: "warp" }) });
    const onInvalid = vi.fn();
    const store = createSettingsStore<Choiced>({ namespace: "game", version: 1, defaults: choiced, choices, onInvalid, backend: memory.backend });
    await store.load();
    await store.load();
    expect(onInvalid.mock.calls).toEqual([
      ["language", "fr"],
      ["speed", "warp"],
    ]);
  });

  it("does not report a value of the wrong type, which is still ignored as before", async () => {
    const memory = memoryBackend({ game: envelope(1, { speed: 3 }) });
    const onInvalid = vi.fn();
    const store = createSettingsStore<Choiced>({ namespace: "game", version: 1, defaults: choiced, choices, onInvalid, backend: memory.backend });
    await store.load();
    expect(store.get("speed")).toBe("normal");
    expect(onInvalid).not.toHaveBeenCalled();
  });

  it("writes the default in place of a refused value at the next save", async () => {
    const memory = memoryBackend({ game: envelope(1, { language: "fr", speed: "fast" }) });
    const store = createSettingsStore<Choiced>({ namespace: "game", version: 1, defaults: choiced, choices, backend: memory.backend });
    await store.load();
    await expect(store.save()).resolves.toBe(true);
    expect(memory.writes[0]).toEqual({ key: "game", value: envelope(1, { language: "es", speed: "fast", volume: 1 }) });
  });

  it("takes null in the list when the default is null, and refuses any other value outside the list", async () => {
    const memory = memoryBackend({ game: envelope(1, { language: "fr" }) });
    const onInvalid = vi.fn();
    const store = createSettingsStore<Prefs>({
      namespace: "game",
      version: 1,
      defaults,
      choices: { language: [null, "es", "en"] },
      onInvalid,
      backend: memory.backend,
    });
    await store.load();
    expect(store.get("language")).toBeNull();
    expect(onInvalid).toHaveBeenCalledWith("language", "fr");
    store.set("language", "en");
    expect(store.get("language")).toBe("en");
    expect(() => store.set("language", "fr")).toThrow(RangeError);
  });

  it("applies a migrated value that is inside the choices, and refuses one that the migration leaves outside", async () => {
    const memory = memoryBackend({ game: envelope(1, { speed: "medium", language: "en" }) });
    const migrate: MigrationMap = {
      1: (values) => {
        const old = values as Record<string, unknown>;
        return { ...old, speed: old["speed"] === "medium" ? "normal" : old["speed"] };
      },
    };
    const onInvalid = vi.fn();
    const store = createSettingsStore<Choiced>({ namespace: "game", version: 2, defaults: choiced, migrate, choices, onInvalid, backend: memory.backend });
    await store.load();
    expect(store.get("speed")).toBe("normal");
    expect(store.get("language")).toBe("en");
    expect(onInvalid).not.toHaveBeenCalled();

    const kept = memoryBackend({ game: envelope(1, { speed: "medium" }) });
    const unchanged = vi.fn();
    const stale = createSettingsStore<Choiced>({
      namespace: "game",
      version: 2,
      defaults: choiced,
      migrate: { 1: (values) => values },
      choices,
      onInvalid: unchanged,
      backend: kept.backend,
    });
    await stale.load();
    expect(stale.get("speed")).toBe("normal");
    expect(unchanged).toHaveBeenCalledWith("speed", "medium");
  });

  it("throws a RangeError for a set outside the choices, and leaves the value, the subscribers and the saves alone", async () => {
    vi.useFakeTimers();
    const memory = memoryBackend();
    const store = createSettingsStore<Choiced>({ namespace: "game", version: 1, defaults: choiced, choices, backend: memory.backend, debounceMs: 250 });
    const listener = vi.fn();
    store.subscribe("speed", listener);
    await store.load();
    expect(() => store.set("speed", "turbo")).toThrow('"turbo" is not a choice of "speed" in "game"');
    expect(store.get("speed")).toBe("normal");
    expect(listener).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1000);
    await settle();
    expect(memory.writes).toHaveLength(0);
  });

  it("does not mark a refused set, so that the saved value still loads", async () => {
    const memory = memoryBackend({ game: envelope(1, { speed: "fast" }) });
    const store = createSettingsStore<Choiced>({ namespace: "game", version: 1, defaults: choiced, choices, backend: memory.backend });
    expect(() => store.set("speed", "turbo")).toThrow(RangeError);
    await store.load();
    expect(store.get("speed")).toBe("fast");
  });

  it("accepts a set of a value in the choices, and saves it", async () => {
    const memory = memoryBackend();
    const store = createSettingsStore<Choiced>({ namespace: "game", version: 1, defaults: choiced, choices, backend: memory.backend });
    store.set("speed", "fast");
    expect(store.get("speed")).toBe("fast");
    await expect(store.save()).resolves.toBe(true);
    expect(memory.writes[0]).toEqual({ key: "game", value: envelope(1, { language: "es", speed: "fast", volume: 1 }) });
  });
});

describe("where the values are kept", () => {
  it("keeps them in the bridge's settings under the namespace, when the desktop host is present", async () => {
    const { bridge } = createFakeBridge();
    vi.stubGlobal("velvet", bridge);
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults });
    expect(store.backendKind).toBe("bridge");
    store.set("volume", 0.25);
    await expect(store.save()).resolves.toBe(true);
    expect(await bridge.settings.get("game")).toEqual(envelope(1, { volume: 0.25, language: null, muted: false, keys: [] }));
  });

  it("keeps them in localStorage, as JSON under velvet-settings and the namespace, in a browser", async () => {
    vi.stubGlobal("velvet", undefined);
    const storage = new FakeStorage();
    const store = createSettingsStore<Prefs>({ namespace: "audio", version: 1, defaults, storage });
    expect(store.backendKind).toBe("localStorage");
    store.set("muted", true);
    await store.save();
    expect(storage.keys()).toEqual(["velvet-settings:audio"]);
    expect(JSON.parse(storage.getItem("velvet-settings:audio") ?? "null")).toEqual(envelope(1, { volume: 1, language: null, muted: true, keys: [] }));

    const again = createSettingsStore<Prefs>({ namespace: "audio", version: 1, defaults, storage });
    await again.load();
    expect(again.get("muted")).toBe(true);
  });

  it("keeps them in memory when there is neither a bridge nor localStorage", async () => {
    vi.stubGlobal("velvet", undefined);
    vi.stubGlobal("localStorage", undefined);
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults });
    expect(store.backendKind).toBe("memory");
    store.set("volume", 0.3);
    await expect(store.save()).resolves.toBe(true);
    await store.load();
    expect(store.get("volume")).toBe(0.3);
  });

  it("skips the bridge and the storage when they are given as null", () => {
    const { bridge } = createFakeBridge();
    vi.stubGlobal("velvet", bridge);
    vi.stubGlobal("localStorage", new FakeStorage());
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, bridge: null, storage: null });
    expect(store.backendKind).toBe("memory");
  });

  it("does not throw when the browser's storage is blocked, and keeps the defaults", async () => {
    vi.stubGlobal("velvet", undefined);
    const blocked: StorageLike = {
      length: 0,
      key: () => null,
      getItem: () => {
        throw new Error("storage is blocked");
      },
      setItem: () => {
        throw new Error("quota exceeded");
      },
      removeItem: () => undefined,
    };
    const store = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, storage: blocked });
    await expect(store.load()).resolves.toBeUndefined();
    expect(store.get("volume")).toBe(1);
    store.set("volume", 0.5);
    await expect(store.save()).resolves.toBe(false);
  });

  it("keeps the values of two namespaces apart in one store", async () => {
    const memory = memoryBackend();
    const audio = createSettingsStore<Prefs>({ namespace: "audio", version: 1, defaults, backend: memory.backend });
    const game = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    audio.set("volume", 0.1);
    game.set("volume", 0.6);
    await audio.save();
    await game.save();

    const reloadedAudio = createSettingsStore<Prefs>({ namespace: "audio", version: 1, defaults, backend: memory.backend });
    const reloadedGame = createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, backend: memory.backend });
    await reloadedAudio.load();
    await reloadedGame.load();
    expect(reloadedAudio.get("volume")).toBe(0.1);
    expect(reloadedGame.get("volume")).toBe(0.6);
    expect([...memory.values.keys()].sort()).toEqual(["audio", "game"]);
  });
});

describe("checks of the options", () => {
  it("refuses a namespace that is empty, too long, or has other characters", () => {
    for (const namespace of ["", " ", "has space", "a/b", "x".repeat(65), ".hidden"]) {
      expect(() => createSettingsStore<Prefs>({ namespace, version: 1, defaults, backend: memoryBackend().backend })).toThrow(RangeError);
    }
    expect(() => createSettingsStore<Prefs>({ namespace: "x".repeat(64), version: 1, defaults, backend: memoryBackend().backend })).not.toThrow();
  });

  it("refuses a version that is not a positive whole number", () => {
    for (const version of [0, -1, 1.5]) {
      expect(() => createSettingsStore<Prefs>({ namespace: "game", version, defaults, backend: memoryBackend().backend })).toThrow(RangeError);
    }
  });

  it("refuses a migration that does not start from an older version, or is not a function", () => {
    const backend = memoryBackend().backend;
    expect(() =>
      createSettingsStore<Prefs>({ namespace: "game", version: 2, defaults, migrate: { 2: (values) => values }, backend }),
    ).toThrow(RangeError);
    expect(() =>
      createSettingsStore<Prefs>({
        namespace: "game",
        version: 2,
        defaults,
        migrate: { 1: "not a function" as unknown as MigrationMap[number] },
        backend,
      }),
    ).toThrow(RangeError);
  });

  it("refuses a negative debounce", () => {
    expect(() => createSettingsStore<Prefs>({ namespace: "game", version: 1, defaults, debounceMs: -1, backend: memoryBackend().backend })).toThrow(RangeError);
  });

  it("refuses choices that name no setting, an empty list, a list without the default, or a value that is not a list", () => {
    const backend = memoryBackend().backend;
    const withChoices = (value: unknown) =>
      createSettingsStore<Choiced>({
        namespace: "game",
        version: 1,
        defaults: choiced,
        choices: value as unknown as SettingsStoreOptions<Choiced>["choices"],
        backend,
      });
    expect(() => withChoices({ colour: ["red"] })).toThrow(RangeError);
    expect(() => withChoices({ speed: [] })).toThrow(RangeError);
    expect(() => withChoices({ speed: ["slow", "fast"] })).toThrow(RangeError);
    expect(() => withChoices({ speed: "normal" })).toThrow(RangeError);
    expect(() => withChoices(choices)).not.toThrow();
  });

  it("refuses an onInvalid that is not a function, and types a choice as a value of its setting", () => {
    const backend = memoryBackend().backend;
    expect(() =>
      createSettingsStore<Choiced>({
        namespace: "game",
        version: 1,
        defaults: choiced,
        onInvalid: "log" as unknown as (key: string, value: unknown) => void,
        backend,
      }),
    ).toThrow(RangeError);
    expect(() =>
      createSettingsStore<Choiced>({
        namespace: "game",
        version: 1,
        defaults: choiced,
        // @ts-expect-error a list of numbers is not a list of the language's values
        choices: { language: [1, 2] },
        backend,
      }),
    ).toThrow(RangeError);
  });

  it("returns a store that names its namespace and version", () => {
    const store: SettingsStore<Prefs> = createSettingsStore<Prefs>({ namespace: "game", version: 4, defaults, backend: memoryBackend().backend });
    expect(store.namespace).toBe("game");
    expect(store.version).toBe(4);
  });
});
