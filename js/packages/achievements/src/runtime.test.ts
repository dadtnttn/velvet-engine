// The runtime rules that the shared cases do not reach: the order of a cascade, the errors of listeners
// and of the clock, the gallery order of a custom file, the options, and the progress of the goals.

import { describe, expect, it } from "vitest";
import { AchievementError, createAchievements, readAchievements, type AchievementUnlock } from "./index";
import { MemoryBackend, expectCode, manualClock, readFixture } from "./test-support";

const fixtureText = readFixture("achievements.json");

/** A store on the fixture definitions, with a memory backend, a manual clock and no debounce wait. */
function fixtureStore(options: { dev?: boolean; clock?: () => number; onUnlock?: (unlock: AchievementUnlock) => void } = {}) {
  return createAchievements({
    definitions: fixtureText,
    backend: new MemoryBackend(),
    clock: options.clock ?? manualClock(1000).read,
    dev: options.dev,
    onUnlock: options.onUnlock,
  });
}

describe("cascades", () => {
  it("reaches the dependents in a fixed order: the counter's goals, then each unlock's dependents in file order", () => {
    const unlocks: AchievementUnlock[] = [];
    const store = createAchievements({
      definitions: {
        format: "velvet-achievements",
        version: 1,
        kind: "definitions",
        counters: [{ id: "a", persist: true }],
        achievements: [
          { id: "x", name: "k.x", description: "k.x.d", icon: "x", goal: { counter: "a", at_least: 1 } },
          { id: "y", name: "k.y", description: "k.y.d", icon: "y", goal: { counter: "a", at_least: 1 } },
          { id: "z", name: "k.z", description: "k.z.d", icon: "z", goal: { all: ["x", "y"] } },
          { id: "w", name: "k.w", description: "k.w.d", icon: "w", goal: { any: ["x", "y"], count: 1 } },
          { id: "v", name: "k.v", description: "k.v.d", icon: "v", goal: { all: ["w", "z"] } },
        ],
      },
      backend: new MemoryBackend(),
      clock: () => 5,
      onUnlock: (unlock) => {
        unlocks.push(unlock);
      },
    });
    store.increment("a", 1);
    expect(unlocks.map((unlock) => unlock.id)).toEqual(["x", "y", "z", "w", "v"]);
    expect(unlocks.every((unlock) => unlock.time === 5)).toBe(true);
    expect(store.unlockedCount).toBe(5);
  });

  it("completes an all goal when its last part unlocks, and reports the parts' progress on the way", () => {
    const store = fixtureStore();
    store.increment("kills", 1);
    expect(store.progress("butcher")).toEqual({ current: 1, target: 2, fraction: 0.5 });
    store.increment("kills", 39);
    expect(store.isUnlocked("butcher")).toBe(true);
  });

  it("unlocks a manual achievement with its dependents, and then the dependents' goals are checked", () => {
    const store = fixtureStore();
    store.unlock("prologue");
    expect(store.isUnlocked("good_company")).toBe(false);
    store.increment("gold", 500);
    expect(store.isUnlocked("good_company")).toBe(true);
  });
});

describe("events and their errors", () => {
  it("sends every event even when a listener throws, and rethrows the first error after them", () => {
    const store = fixtureStore();
    const seen: string[] = [];
    store.on("unlocked", () => {
      throw new Error("a listener broke");
    });
    store.on("changed", () => {
      seen.push("changed");
    });
    expect(() => store.increment("kills", 1)).toThrow("a listener broke");
    expect(seen).toEqual(["changed"]);
    expect(store.counter("kills")).toBe(1);
    expect(store.isUnlocked("first_blood")).toBe(true);
  });

  it("calls the hook and then the event, and an error of the hook does not stop the event", () => {
    const order: string[] = [];
    const store = fixtureStore({
      onUnlock: () => {
        order.push("hook");
        throw new Error("the bridge failed");
      },
    });
    store.on("unlocked", (unlock) => {
      order.push(`event ${unlock.id}`);
    });
    expect(() => store.increment("kills", 1)).toThrow("the bridge failed");
    expect(order).toEqual(["hook", "event first_blood"]);
  });

  it("delivers once and removes a listener with once and off", () => {
    const store = fixtureStore();
    let onceCount = 0;
    let offCount = 0;
    const onceListener = (): void => {
      onceCount += 1;
    };
    const offListener = (): void => {
      offCount += 1;
    };
    store.once("changed", onceListener);
    store.on("changed", offListener);
    store.off("changed", offListener);
    store.increment("gold", 1);
    store.increment("gold", 1);
    expect(onceCount).toBe(1);
    expect(offCount).toBe(0);
  });

  it("sends the unlocks, then the progress notices in step order, then changed", () => {
    const store = fixtureStore();
    const order: string[] = [];
    store.on("unlocked", (unlock) => {
      order.push(`unlocked ${unlock.id}`);
    });
    store.on("progress", (notice) => {
      order.push(`progress ${notice.id} ${notice.step}`);
    });
    store.on("changed", () => {
      order.push("changed");
    });
    store.increment("kills", 30);
    expect(order).toEqual([
      "unlocked first_blood",
      "progress veteran 0.25",
      "progress veteran 0.5",
      "progress veteran 0.75",
      "changed",
    ]);
  });
});

describe("the clock", () => {
  it("refuses a clock that gives no finite time, and changes nothing", () => {
    const store = fixtureStore({ clock: () => Number.NaN });
    expectCode(() => store.increment("kills", 1), "argument");
    expect(store.counter("kills")).toBe(0);
    expect(store.unlockedCount).toBe(0);
  });

  it("does not read the clock for a change that does nothing", () => {
    let reads = 0;
    const store = fixtureStore({
      clock: () => {
        reads += 1;
        return 1000;
      },
    });
    store.increment("kills", 0);
    store.setCounter("kills", 0);
    expect(reads).toBe(0);
  });
});

describe("the gallery order", () => {
  it("takes the groups in the order of their first achievement in the file, then order, then the place in the file", () => {
    const store = createAchievements({
      definitions: {
        format: "velvet-achievements",
        version: 1,
        kind: "definitions",
        counters: [],
        achievements: [
          { id: "b_one", name: "k", description: "k", icon: "i", group: "zeta", order: 2, goal: { manual: true } },
          { id: "a_one", name: "k", description: "k", icon: "i", group: "alpha", order: 1, goal: { manual: true } },
          { id: "b_two", name: "k", description: "k", icon: "i", group: "zeta", order: 1, goal: { manual: true } },
          { id: "a_two", name: "k", description: "k", icon: "i", group: "alpha", order: 1, goal: { manual: true } },
        ],
      },
      backend: new MemoryBackend(),
    });
    expect(store.list().map((entry) => entry.id)).toEqual(["b_two", "b_one", "a_one", "a_two"]);
  });

  it("lists the entries of a masked achievement with its hint, and without its text", () => {
    const store = fixtureStore();
    const butcher = store.list().find((entry) => entry.id === "butcher");
    expect(butcher).toMatchObject({
      masked: true,
      name: null,
      description: null,
      icon: null,
      hint: "ach.butcher.hint",
      progress: null,
      unlocked_at: null,
      hidden: true,
    });
    store.unlock("prologue");
    store.increment("kills", 1);
    store.increment("kills", 39);
    expect(store.list().find((entry) => entry.id === "butcher")).toMatchObject({
      masked: false,
      name: "ach.butcher.name",
      hint: null,
      unlocked_at: 1000,
    });
  });
});

describe("progress of each kind of goal", () => {
  it("counts the unlocked parts of an all goal and of an any goal, up to its count", () => {
    const store = fixtureStore();
    store.unlock("prologue");
    store.increment("gold", 500);
    expect(store.progress("good_company")).toEqual({ current: 2, target: 2, fraction: 1 });
    expect(store.progress("marathon")).toEqual({ current: 0, target: 100, fraction: 0 });
  });

  it("gives a manual goal 0 of 1 while locked and 1 of 1 once unlocked", () => {
    const store = fixtureStore();
    expect(store.progress("prologue")).toEqual({ current: 0, target: 1, fraction: 0 });
    store.unlock("prologue");
    expect(store.progress("prologue")).toEqual({ current: 1, target: 1, fraction: 1 });
  });

  it("stops a counter's progress at its goal", () => {
    const store = fixtureStore();
    store.increment("kills", 100);
    expect(store.progress("veteran")).toEqual({ current: 40, target: 40, fraction: 1 });
  });
});

describe("options and the definitions", () => {
  it("accepts the definitions as an object, as a JSON import gives them", () => {
    const store = createAchievements({ definitions: JSON.parse(fixtureText) as object, backend: new MemoryBackend() });
    expect(store.totalCount).toBe(9);
    expect(store.totalPoints).toBe(240);
  });

  it("refuses definitions that break a rule, with the code of the first problem", () => {
    let thrown: unknown;
    try {
      createAchievements({ definitions: { format: "velvet-achievements", version: 1, kind: "definitions", achievements: [{}] }, backend: new MemoryBackend() });
    } catch (error) {
      thrown = error;
    }
    expect(thrown).toBeInstanceOf(AchievementError);
    expect(thrown).toMatchObject({ code: "structure" });
  });

  it("refuses text that is not JSON with the code json", () => {
    expectCode(() => createAchievements({ definitions: "{", backend: new MemoryBackend() }), "json");
  });

  it("checks the debounce and the key", () => {
    expect(() => fixtureStore({})).not.toThrow();
    expect(() => createAchievements({ definitions: fixtureText, backend: new MemoryBackend(), debounceMs: -1 })).toThrow(RangeError);
    expect(() => createAchievements({ definitions: fixtureText, backend: new MemoryBackend(), debounceMs: Number.NaN })).toThrow(RangeError);
    expect(() => createAchievements({ definitions: fixtureText, backend: new MemoryBackend(), key: "" })).toThrow(RangeError);
  });

  it("gives the normal form of the definitions, with the defaults filled in", () => {
    const catalog = readAchievements(JSON.parse(fixtureText) as unknown);
    expect(catalog.achievements.find((def) => def.id === "marathon")).toMatchObject({
      group: "default",
      order: 0,
      hidden: false,
      hint: null,
      progress_steps: [],
    });
  });
});
