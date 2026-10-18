// Tests for the save backends: memory, localStorage, the bridge's own saves object, and the
// lookup of localStorage itself.

import { afterEach, describe, expect, it, vi } from "vitest";
import { createBridgeBackend, createLocalStorageBackend, createMemoryBackend, getLocalStorage } from "./save-backends";
import { FakeStorage, createFakeBridge } from "./test-support";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("memory backend", () => {
  it("keeps texts by slot until they are removed", async () => {
    const backend = createMemoryBackend();
    await backend.write("b", "2");
    await backend.write("a", "1");
    expect((await backend.list()).sort()).toEqual(["a", "b"]);
    expect(await backend.read("a")).toBe("1");
    expect(await backend.read("missing")).toBeNull();
    await backend.remove("a");
    expect(await backend.read("a")).toBeNull();
    await backend.remove("never_written");
  });
});

describe("localStorage backend", () => {
  it("namespaces its keys by game and lists only that game's slots", async () => {
    const storage = new FakeStorage();
    const demo = createLocalStorageBackend(storage, "demo");
    const other = createLocalStorageBackend(storage, "other");
    storage.setItem("theme", "dark");
    await demo.write("slot_1", "{}");
    await other.write("slot_2", "{}");
    expect(storage.keys()).toEqual(["theme", "velvet-save:demo:slot_1", "velvet-save:other:slot_2"]);
    expect(await demo.list()).toEqual(["slot_1"]);
    expect(await demo.read("slot_1")).toBe("{}");
    await demo.remove("slot_1");
    expect(await demo.list()).toEqual([]);
    expect(await demo.read("slot_1")).toBeNull();
  });
});

describe("bridge backend", () => {
  it("is the bridge's own saves object", () => {
    const { bridge } = createFakeBridge();
    expect(createBridgeBackend(bridge)).toBe(bridge.saves);
  });
});

describe("getLocalStorage", () => {
  it("returns null when there is no localStorage", () => {
    vi.stubGlobal("localStorage", undefined);
    expect(getLocalStorage()).toBeNull();
  });

  it("returns an object that has the storage methods", () => {
    const storage = new FakeStorage();
    vi.stubGlobal("localStorage", storage);
    expect(getLocalStorage()).toBe(storage);
  });

  it("returns null for an object that lacks the storage methods", () => {
    vi.stubGlobal("localStorage", { length: 0 });
    expect(getLocalStorage()).toBeNull();
  });

  it("returns null when the browser blocks storage", () => {
    const original = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      get() {
        throw new Error("storage is blocked");
      },
    });
    try {
      expect(getLocalStorage()).toBeNull();
    } finally {
      if (original !== undefined) {
        Object.defineProperty(globalThis, "localStorage", original);
      } else {
        Reflect.deleteProperty(globalThis, "localStorage");
      }
    }
  });
});
