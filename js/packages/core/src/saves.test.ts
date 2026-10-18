// Tests for the save store: envelopes, migrations, the checks on load, slot names and the
// choice of backend.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createMemoryBackend, type SaveBackend } from "./save-backends";
import { SAVE_FORMAT, encodeEnvelope, type SaveEnvelope } from "./save-format";
import { createSaveStore } from "./saves";
import { FakeStorage, createFakeBridge, errorOf } from "./test-support";

const NOW = "2026-10-18T10:00:00.000Z";

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date(NOW));
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

/** Writes an envelope straight into a backend, the way an older build would have. */
async function writeEnvelope(backend: SaveBackend, slot: string, envelope: SaveEnvelope): Promise<void> {
  await backend.write(slot, encodeEnvelope(envelope));
}

describe("createSaveStore saving and loading", () => {
  it("round-trips data and records when it was saved", async () => {
    const store = createSaveStore({ game: "demo", version: 1, backend: createMemoryBackend() });
    await store.save("slot_1", { gold: 42, flags: ["intro"] });
    expect(await store.load("slot_1")).toEqual({
      slot: "slot_1",
      savedAt: NOW,
      version: 1,
      data: { gold: 42, flags: ["intro"] },
    });
  });

  it("writes the versioned envelope as JSON", async () => {
    const backend = createMemoryBackend();
    const store = createSaveStore({ game: "demo", version: 2, backend });
    await store.save("slot_1", { gold: 1 });
    const parsed: unknown = JSON.parse(String(await backend.read("slot_1")));
    expect(parsed).toEqual({ format: SAVE_FORMAT, version: 2, game: "demo", savedAt: NOW, data: { gold: 1 } });
  });

  it("returns null for an empty slot", async () => {
    const store = createSaveStore({ game: "demo", version: 1, backend: createMemoryBackend() });
    expect(await store.load("nothing_here")).toBeNull();
  });

  it("lists the slots in sorted order and skips names that are not valid slots", async () => {
    const backend = createMemoryBackend();
    const store = createSaveStore({ game: "demo", version: 1, backend });
    await store.save("b", 1);
    await store.save("a", 2);
    await backend.write("not valid!", "stray");
    expect(await store.list()).toEqual(["a", "b"]);
  });

  it("removes a slot, and removing an empty slot is not an error", async () => {
    const store = createSaveStore({ game: "demo", version: 1, backend: createMemoryBackend() });
    await store.save("slot_1", 1);
    await store.remove("slot_1");
    await store.remove("slot_1");
    expect(await store.load("slot_1")).toBeNull();
  });

  it("exposes the game id and the version", () => {
    const store = createSaveStore({ game: "demo", version: 4, backend: createMemoryBackend() });
    expect(store.game).toBe("demo");
    expect(store.version).toBe(4);
  });
});

describe("createSaveStore migrations and checks on load", () => {
  it("applies the migrations in order, from the saved version up to the current one", async () => {
    const backend = createMemoryBackend();
    const store = createSaveStore({
      game: "demo",
      version: 3,
      backend,
      migrations: {
        1: (data) => (data as number) + 10,
        2: (data) => (data as number) * 2,
      },
    });
    await writeEnvelope(backend, "old", {
      format: SAVE_FORMAT,
      version: 1,
      game: "demo",
      savedAt: "2026-01-01T00:00:00.000Z",
      data: 1,
    });
    expect(await store.load("old")).toEqual({
      slot: "old",
      savedAt: "2026-01-01T00:00:00.000Z",
      version: 3,
      data: 22,
    });
  });

  it("refuses a save written by a newer version", async () => {
    const backend = createMemoryBackend();
    const store = createSaveStore({ game: "demo", version: 3, backend });
    await writeEnvelope(backend, "future", { format: SAVE_FORMAT, version: 4, game: "demo", savedAt: NOW, data: {} });
    expect(await errorOf(() => store.load("future"))).toMatchObject({ code: "newer-version" });
  });

  it("refuses a save that belongs to another game", async () => {
    const backend = createMemoryBackend();
    const store = createSaveStore({ game: "demo", version: 1, backend });
    await writeEnvelope(backend, "foreign", { format: SAVE_FORMAT, version: 1, game: "other", savedAt: NOW, data: 1 });
    expect(await errorOf(() => store.load("foreign"))).toMatchObject({ code: "wrong-game" });
  });

  it("reports text that is not a save envelope as corrupt", async () => {
    const backend = createMemoryBackend();
    const store = createSaveStore({ game: "demo", version: 1, backend });
    await backend.write("broken", "{not json");
    await backend.write("odd", "[1, 2]");
    expect(await errorOf(() => store.load("broken"))).toMatchObject({ code: "corrupt" });
    expect(await errorOf(() => store.load("odd"))).toMatchObject({ code: "corrupt" });
  });

  it("reports a migration that is missing", async () => {
    const backend = createMemoryBackend();
    const store = createSaveStore({ game: "demo", version: 3, backend, migrations: { 2: (data) => data } });
    await writeEnvelope(backend, "old", { format: SAVE_FORMAT, version: 1, game: "demo", savedAt: NOW, data: 1 });
    expect(await errorOf(() => store.load("old"))).toMatchObject({ code: "missing-migration" });
  });
});

describe("createSaveStore slot names, data and configuration", () => {
  it("rejects an invalid slot name in every call", async () => {
    const store = createSaveStore({ game: "demo", version: 1, backend: createMemoryBackend() });
    for (const slot of ["../escape", "", "a b", "x".repeat(65)]) {
      expect(await errorOf(() => store.save(slot, 1))).toMatchObject({ code: "bad-slot" });
      expect(await errorOf(() => store.load(slot))).toMatchObject({ code: "bad-slot" });
      expect(await errorOf(() => store.remove(slot))).toMatchObject({ code: "bad-slot" });
    }
  });

  it("accepts a slot name of 64 characters", async () => {
    const store = createSaveStore({ game: "demo", version: 1, backend: createMemoryBackend() });
    const slot = "x".repeat(64);
    await store.save(slot, 1);
    expect(await store.load(slot)).toMatchObject({ data: 1 });
  });

  it("refuses data that cannot be stored", async () => {
    const store = createSaveStore({ game: "demo", version: 1, backend: createMemoryBackend() });
    expect(await errorOf(() => store.save("slot_1", undefined))).toMatchObject({ code: "bad-data" });
    expect(await errorOf(() => store.save("slot_1", 10n))).toMatchObject({ code: "bad-data" });
  });

  it("rejects an invalid configuration", () => {
    expect(() => createSaveStore({ game: "demo", version: 0 })).toThrow(RangeError);
    expect(() => createSaveStore({ game: "  ", version: 1 })).toThrow(RangeError);
    expect(() => createSaveStore({ game: "demo", version: 3, migrations: { 3: (data) => data } })).toThrow(RangeError);
    expect(() => createSaveStore({ game: "demo", version: 3, migrations: { 0: (data) => data } })).toThrow(RangeError);
  });
});

describe("createSaveStore choice of backend", () => {
  it("uses the desktop bridge when the host provides one", async () => {
    const { bridge, files } = createFakeBridge();
    vi.stubGlobal("velvet", bridge);
    const store = createSaveStore({ game: "demo", version: 1 });
    expect(store.backendKind).toBe("bridge");
    await store.save("slot_1", { gold: 3 });
    expect(files.has("slot_1")).toBe(true);
    expect(await store.load("slot_1")).toMatchObject({ data: { gold: 3 } });
  });

  it("uses localStorage in a browser that has no bridge", async () => {
    const storage = new FakeStorage();
    vi.stubGlobal("localStorage", storage);
    const store = createSaveStore({ game: "demo", version: 1 });
    expect(store.backendKind).toBe("localStorage");
    await store.save("slot_1", { gold: 3 });
    expect(storage.keys()).toEqual(["velvet-save:demo:slot_1"]);
  });

  it("falls back to memory when there is no storage at all", async () => {
    vi.stubGlobal("localStorage", undefined);
    const store = createSaveStore({ game: "demo", version: 1 });
    expect(store.backendKind).toBe("memory");
    await store.save("slot_1", 5);
    expect(await store.load("slot_1")).toMatchObject({ data: 5 });
  });

  it("uses the backend that is passed in", () => {
    const store = createSaveStore({ game: "demo", version: 1, backend: createMemoryBackend() });
    expect(store.backendKind).toBe("custom");
  });
});
