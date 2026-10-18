// Tests for the settings backends and the choice between them: the backend passed in first, then the
// bridge's settings, then localStorage under the prefix, then memory. Also the localStorage and memory
// backends themselves.

import { afterEach, describe, expect, it, vi } from "vitest";
import {
  chooseSettingsBackend,
  createLocalStorageSettings,
  createMemorySettings,
  type SettingsBackend,
} from "./settings-backend";
import { FakeStorage, createFakeBridge } from "./test-support";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("chooseSettingsBackend", () => {
  it("uses the backend that is passed in before the bridge or the storage", () => {
    const { bridge } = createFakeBridge();
    vi.stubGlobal("velvet", bridge);
    vi.stubGlobal("localStorage", new FakeStorage());
    const explicit = createMemorySettings();
    const chosen = chooseSettingsBackend({ explicit, prefix: "test:" });
    expect(chosen.kind).toBe("custom");
    expect(chosen.backend).toBe(explicit);
  });

  it("uses the bridge's settings in Velvet Desktop", () => {
    const { bridge } = createFakeBridge();
    vi.stubGlobal("velvet", bridge);
    const chosen = chooseSettingsBackend({ prefix: "test:" });
    expect(chosen.kind).toBe("bridge");
    expect(chosen.backend).toBe(bridge.settings);
  });

  it("uses localStorage under the prefix in a browser", async () => {
    vi.stubGlobal("velvet", undefined);
    const storage = new FakeStorage();
    vi.stubGlobal("localStorage", storage);
    const chosen = chooseSettingsBackend({ prefix: "test:" });
    expect(chosen.kind).toBe("localStorage");
    await chosen.backend.set("key", 7);
    expect(storage.getItem("test:key")).toBe("7");
  });

  it("uses memory when there is neither a bridge nor localStorage", async () => {
    vi.stubGlobal("velvet", undefined);
    vi.stubGlobal("localStorage", undefined);
    const chosen = chooseSettingsBackend({ prefix: "test:" });
    expect(chosen.kind).toBe("memory");
    await chosen.backend.set("key", "kept");
    await expect(chosen.backend.get("key")).resolves.toBe("kept");
  });

  it("uses the bridge or the storage that is passed in, in place of the runtime's", () => {
    vi.stubGlobal("velvet", undefined);
    const bridgeSettings = createMemorySettings();
    expect(chooseSettingsBackend({ bridge: { settings: bridgeSettings }, prefix: "t:" }).backend).toBe(bridgeSettings);
    const storage = new FakeStorage();
    expect(chooseSettingsBackend({ storage, prefix: "t:" }).kind).toBe("localStorage");
  });

  it("skips the bridge and the storage when they are given as null", () => {
    const { bridge } = createFakeBridge();
    vi.stubGlobal("velvet", bridge);
    vi.stubGlobal("localStorage", new FakeStorage());
    expect(chooseSettingsBackend({ bridge: null, storage: null, prefix: "t:" }).kind).toBe("memory");
  });
});

describe("createLocalStorageSettings", () => {
  it("keeps each value as JSON under the prefix, and reads null for a key with nothing in it", async () => {
    const storage = new FakeStorage();
    const backend = createLocalStorageSettings(storage, "x:");
    await expect(backend.get("a")).resolves.toBeNull();
    await backend.set("a", { on: true });
    expect(storage.getItem("x:a")).toBe('{"on":true}');
    await expect(backend.get("a")).resolves.toEqual({ on: true });
  });

  it("clears a key when null is stored in it", async () => {
    const backend = createLocalStorageSettings(new FakeStorage(), "x:");
    await backend.set("a", 1);
    await backend.set("a", null);
    await expect(backend.get("a")).resolves.toBeNull();
  });

  it("returns a text that is not JSON as the text, so that its reader can tell it is damaged", async () => {
    const storage = new FakeStorage();
    storage.setItem("x:a", "{ broken");
    await expect(createLocalStorageSettings(storage, "x:").get("a")).resolves.toBe("{ broken");
  });

  it("lets a refused write reject, so the caller can decide what it means", async () => {
    const refusing = new FakeStorage();
    refusing.setItem = () => {
      throw new Error("quota exceeded");
    };
    await expect(createLocalStorageSettings(refusing, "x:").set("a", 1)).rejects.toThrow("quota exceeded");
  });
});

describe("createMemorySettings", () => {
  it("keeps the values of one backend, and gives null for a key that was never set", async () => {
    const backend: SettingsBackend = createMemorySettings();
    await expect(backend.get("missing")).resolves.toBeNull();
    await backend.set("a", [1, 2]);
    await expect(backend.get("a")).resolves.toEqual([1, 2]);
    expect(await createMemorySettings().get("a")).toBeNull();
  });
});
