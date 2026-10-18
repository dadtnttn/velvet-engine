// Saving and loading: the round trip, the order of writes, the storage that refuses, the merge of a load,
// the debounce of the automatic saves (with fake timers), the three places the progress can go, and the
// dev-only reset.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createAchievements, type Achievements, type AchievementsOptions } from "./index";
import { MemoryBackend, MemoryStorage, manualClock, readFixture } from "./test-support";

const fixtureText = readFixture("achievements.json");
const KEY = "achievements";

/** A saved progress document, as the runtime writes it. */
function savedDocument(counters: Record<string, number>, unlocked: Record<string, number> = {}): object {
  return { format: "velvet-achievements", version: 1, kind: "progress", counters, unlocked };
}

/** A store on the fixture definitions, on the given storage and clock. */
function storeOn(backend: MemoryBackend, clock = manualClock(1000), extra: Partial<AchievementsOptions> = {}): Achievements {
  return createAchievements({ definitions: fixtureText, backend, clock: clock.read, ...extra });
}

/** Lets the promises that a timer started run to the end, so that their writes have happened. */
async function flushPromises(): Promise<void> {
  for (let index = 0; index < 100; index += 1) await Promise.resolve();
}

describe("saving and loading", () => {
  it("loads back the counters and the unlocks of a saved run, with their times", async () => {
    const backend = new MemoryBackend();
    const clock = manualClock(500);
    const first = storeOn(backend, clock);
    first.unlock("prologue");
    clock.now = 700;
    first.increment("gold", 120);
    expect(await first.save()).toBe(true);
    expect(backend.peek(KEY)).toEqual(savedDocument({ kills: 0, gold: 120 }, { prologue: 500 }));

    const second = storeOn(backend, manualClock(9000));
    expect(await second.load()).toEqual({ status: "loaded", issues: [] });
    expect(second.counter("gold")).toBe(120);
    expect(second.isUnlocked("prologue")).toBe(true);
    expect(second.list().find((entry) => entry.id === "prologue")?.unlocked_at).toBe(500);
  });

  it("writes in the order of the calls, so the last state set is the last state written", async () => {
    const backend = new MemoryBackend();
    const store = storeOn(backend);
    store.increment("gold", 1);
    const first = store.save();
    store.increment("gold", 1);
    const second = store.save();
    expect(await Promise.all([first, second])).toEqual([true, true]);
    expect(backend.peek(KEY)).toMatchObject({ counters: { gold: 2 } });
  });

  it("resolves false when the storage refuses a write, and never throws", async () => {
    const backend = new MemoryBackend();
    backend.failWrites = true;
    const store = storeOn(backend);
    store.increment("gold", 1);
    expect(await store.save()).toBe(false);
    expect(store.counter("gold")).toBe(1);
  });

  it("reports an unavailable storage on load, and the store carries on from its own progress", async () => {
    const backend = new MemoryBackend();
    backend.failReads = true;
    const store = storeOn(backend);
    expect(await store.load()).toEqual({ status: "unavailable", issues: [] });
    store.increment("gold", 3);
    expect(store.counter("gold")).toBe(3);
  });

  it("writes nothing while the storage refuses the read, and reads again once it answers", async () => {
    const backend = new MemoryBackend();
    backend.put(KEY, savedDocument({ kills: 0, gold: 7 }));
    backend.failReads = true;
    const store = storeOn(backend);
    expect(await store.load()).toEqual({ status: "unavailable", issues: [] });
    store.increment("gold", 1);
    expect(await store.save()).toBe(false);
    expect(backend.writes).toBe(0);

    backend.failReads = false;
    expect(await store.save()).toBe(true);
    expect(store.counter("gold")).toBe(7);
    expect(backend.peek(KEY)).toMatchObject({ counters: { gold: 7 } });
  });

  it("reads the save once: repeated calls to load share one result", async () => {
    const store = storeOn(new MemoryBackend());
    const first = store.load();
    expect(store.load()).toBe(first);
    expect(await first).toEqual({ status: "fresh", issues: [] });
  });

  it("keeps a count made before the load, and takes the larger of the two counts", async () => {
    const backend = new MemoryBackend();
    backend.put(KEY, savedDocument({ kills: 0, gold: 5 }));
    const store = storeOn(backend);
    store.increment("gold", 3);
    expect(await store.load()).toMatchObject({ status: "loaded" });
    expect(store.counter("gold")).toBe(5);

    const other = new MemoryBackend();
    other.put(KEY, savedDocument({ kills: 0, gold: 1 }));
    const later = storeOn(other);
    later.increment("gold", 3);
    await later.load();
    expect(later.counter("gold")).toBe(3);
  });

  it("waits for the load when it saves first, so the saved counters are not replaced", async () => {
    const backend = new MemoryBackend();
    backend.put(KEY, savedDocument({ kills: 0, gold: 40 }));
    const store = storeOn(backend);
    expect(await store.save()).toBe(true);
    expect(backend.peek(KEY)).toMatchObject({ counters: { gold: 40 } });
  });

  it("announces the unlocks that a load makes, but not the ones the save already held", async () => {
    const backend = new MemoryBackend();
    backend.put(KEY, savedDocument({ kills: 1 }, { prologue: 300 }));
    const hooked: string[] = [];
    const store = storeOn(backend, manualClock(2000), {
      onUnlock: (unlock) => {
        hooked.push(unlock.id);
      },
    });
    const events: string[] = [];
    store.on("unlocked", (unlock) => {
      events.push(unlock.id);
    });
    await store.load();
    expect(hooked).toEqual(["first_blood"]);
    expect(events).toEqual(["first_blood"]);
    expect(store.list().find((entry) => entry.id === "first_blood")?.unlocked_at).toBe(2000);
    expect(store.list().find((entry) => entry.id === "prologue")?.unlocked_at).toBe(300);
  });

  it("drops the counters and the achievements that the definitions do not know, from the next save on", async () => {
    const backend = new MemoryBackend();
    backend.put(KEY, savedDocument({ kills: 0, ghost: 9 }, { ghost_ach: 10 }));
    const store = storeOn(backend);
    await store.load();
    await store.save();
    expect(backend.peek(KEY)).toEqual(savedDocument({ kills: 0, gold: 0 }));
  });
});

describe("the storage that the progress goes to", () => {
  it("keeps the progress in memory when neither a bridge nor a storage is there", async () => {
    const store = createAchievements({ definitions: fixtureText, bridge: null, storage: null });
    expect(store.backendKind).toBe("memory");
    store.increment("gold", 1);
    expect(await store.save()).toBe(true);
  });

  it("keeps the progress in localStorage under the prefix and the key", async () => {
    const storage = new MemoryStorage();
    const store = createAchievements({ definitions: fixtureText, bridge: null, storage });
    expect(store.backendKind).toBe("localStorage");
    store.increment("gold", 4);
    expect(await store.save()).toBe(true);
    const text = storage.text("velvet-achievements:achievements");
    expect(JSON.parse(text ?? "null")).toMatchObject({ kind: "progress", counters: { gold: 4 } });
  });

  it("reports text that is not JSON in localStorage as a damaged save, and the store starts empty", async () => {
    const storage = new MemoryStorage();
    storage.setItem("velvet-achievements:achievements", "{ not json");
    const store = createAchievements({ definitions: fixtureText, bridge: null, storage });
    expect(await store.load()).toMatchObject({ status: "damaged", issues: [{ code: "structure" }] });
    expect(store.counter("gold")).toBe(0);
  });

  it("uses the bridge's settings when the bridge is given", async () => {
    const backend = new MemoryBackend();
    const store = createAchievements({ definitions: fixtureText, bridge: { settings: backend }, storage: null });
    expect(store.backendKind).toBe("bridge");
    store.increment("gold", 2);
    expect(await store.save()).toBe(true);
    expect(backend.peek(KEY)).toMatchObject({ counters: { gold: 2 } });
  });

  it("keeps two games on one storage apart when they use different keys", async () => {
    const backend = new MemoryBackend();
    const first = storeOn(backend, manualClock(1000), { key: "game-a" });
    const second = storeOn(backend, manualClock(1000), { key: "game-b" });
    first.increment("gold", 7);
    second.increment("gold", 9);
    await first.save();
    await second.save();
    const reloaded = storeOn(backend, manualClock(1000), { key: "game-a" });
    await reloaded.load();
    expect(reloaded.counter("gold")).toBe(7);
  });
});

describe("the automatic saves", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("saves a change after the debounce, once the changes stop", async () => {
    const backend = new MemoryBackend();
    const store = storeOn(backend);
    store.increment("gold", 1);
    await vi.advanceTimersByTimeAsync(249);
    expect(backend.writes).toBe(0);
    await vi.advanceTimersByTimeAsync(1);
    await flushPromises();
    expect(backend.writes).toBe(1);
  });

  it("a debounce of 0 saves at once", async () => {
    const backend = new MemoryBackend();
    const store = storeOn(backend, manualClock(1000), { debounceMs: 0 });
    store.increment("gold", 1);
    await vi.advanceTimersByTimeAsync(0);
    await flushPromises();
    expect(backend.writes).toBe(1);
  });

  it("save() cancels the pending save, so the storage takes one write", async () => {
    const backend = new MemoryBackend();
    const store = storeOn(backend);
    store.increment("gold", 1);
    expect(await store.save()).toBe(true);
    await vi.advanceTimersByTimeAsync(1000);
    await flushPromises();
    expect(backend.writes).toBe(1);
  });
});

describe("resetAll", () => {
  it("clears the counters and the unlocks, and the progress notices come again", () => {
    const store = storeOn(new MemoryBackend(), manualClock(1000), { dev: true });
    const steps: number[] = [];
    store.on("progress", (notice) => {
      steps.push(notice.step);
    });
    store.increment("kills", 10);
    expect(steps).toEqual([0.25]);
    store.resetAll();
    expect(store.counter("kills")).toBe(0);
    expect(store.unlockedCount).toBe(0);
    store.increment("kills", 10);
    expect(steps).toEqual([0.25, 0.25]);
  });
});
