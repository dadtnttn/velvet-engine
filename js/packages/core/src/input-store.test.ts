// Tests for the bindings file: serialization, parsing and migration of the versioned file, applying a
// file to a map, and the store that keeps the file in the bridge's settings, in localStorage, in memory,
// or in a backend that the game passes in. The fixtures in docs/formats/fixtures/input-bindings are the
// conformance cases that the format document describes.

import { readFileSync } from "node:fs";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ActionMap } from "./input";
import {
  BINDINGS_FORMAT,
  BINDINGS_KEY,
  applyBindings,
  createBindingsStore,
  parseBindings,
  serializeBindings,
  type BindingsFile,
  type SettingsBackend,
} from "./input-store";
import type { Migration, MigrationMap } from "./save-format";
import { SaveError } from "./save-error";
import { FakeStorage, createFakeBridge } from "./test-support";

type Action = "confirm" | "move" | "attack" | "zoom" | "pause";

/** The actions that the game lists in these tests. */
const KNOWN: readonly Action[] = ["confirm", "move", "attack", "zoom", "pause"];

const FIXTURES = new URL("../../../../docs/formats/fixtures/input-bindings/", import.meta.url);

/** The text of a fixture file. */
function readFixture(name: string): string {
  return readFileSync(new URL(name, FIXTURES), "utf8");
}

/** The code of the SaveError that an action throws, or null when it throws another error or nothing. */
function saveErrorCode(action: () => unknown): string | null {
  try {
    action();
    return null;
  } catch (error) {
    return error instanceof SaveError ? error.code : null;
  }
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("serializeBindings", () => {
  it("writes every slot of every action, with an empty slot as null and the default fields left out", () => {
    const map = new ActionMap<Action>([
      { action: "confirm", control: { type: "key", key: "Enter" } },
      { action: "confirm", control: { type: "key", key: "Space" }, slot: 2 },
      { action: "move", control: { type: "key", key: "KeyW" }, axisComponent: "y" },
      { action: "move", control: { type: "key", key: "KeyS" }, axisComponent: "y", scale: -1, context: "gameplay" },
    ]);
    expect(serializeBindings(map)).toEqual({
      format: BINDINGS_FORMAT,
      version: 1,
      actions: [
        {
          action: "confirm",
          slots: [{ control: { type: "key", key: "Enter" } }, null, { control: { type: "key", key: "Space" } }],
        },
        {
          action: "move",
          slots: [
            { control: { type: "key", key: "KeyW" }, axisComponent: "y" },
            { control: { type: "key", key: "KeyS" }, axisComponent: "y", scale: -1, context: "gameplay" },
          ],
        },
      ],
    });
  });

  it("writes a file that reads back to the same file", () => {
    const map = new ActionMap<Action>([
      { action: "attack", control: { type: "pad-button", button: "rightTrigger", threshold: 0.6 } },
      { action: "zoom", control: { type: "pad-stick", stick: "right", direction: "up", deadzone: 0.3 } },
    ]);
    const file = serializeBindings(map);
    expect(parseBindings(JSON.stringify(file))).toEqual(file);
  });
});

describe("parseBindings", () => {
  it("reads the text of a file and the object that the text holds in the same way", () => {
    const text = readFixture("v1.json");
    expect(parseBindings(text)).toEqual(parseBindings(JSON.parse(text)));
  });

  it("reads the fixture with its scale, axis, context and deadzones", () => {
    const file = parseBindings(readFixture("v1.json"));
    expect(file.actions.map((entry) => entry.action)).toEqual(["confirm", "move", "attack", "zoom", "retired-action"]);
    expect(file.actions[1]?.slots[1]).toEqual({
      control: { type: "key", key: "KeyS" },
      axisComponent: "y",
      scale: -1,
    });
    expect(file.actions[1]?.slots[2]).toEqual({
      control: { type: "pad-axis", axis: "leftX", deadzone: 0.25 },
      axisComponent: "x",
    });
    expect(file.actions[2]?.slots[1]?.context).toBe("gameplay");
  });

  it("refuses text that is not JSON, and a file of another format", () => {
    expect(saveErrorCode(() => parseBindings("{"))).toBe("corrupt");
    expect(saveErrorCode(() => parseBindings({ format: "velvet-save", version: 1, actions: [] }))).toBe("corrupt");
    expect(saveErrorCode(() => parseBindings(null))).toBe("corrupt");
  });

  it("refuses a file from a newer build", () => {
    expect(saveErrorCode(() => parseBindings(readFixture("newer-version.json")))).toBe("newer-version");
  });

  it("refuses a control that is not valid, a duplicate action, and a field of the wrong type", () => {
    expect(saveErrorCode(() => parseBindings(readFixture("invalid-control.json")))).toBe("corrupt");

    const twice = {
      format: BINDINGS_FORMAT,
      version: 1,
      actions: [
        { action: "confirm", slots: [] },
        { action: "confirm", slots: [] },
      ],
    };
    expect(saveErrorCode(() => parseBindings(twice))).toBe("corrupt");

    const textScale = {
      format: BINDINGS_FORMAT,
      version: 1,
      actions: [{ action: "confirm", slots: [{ control: { type: "key", key: "Enter" }, scale: "2" }] }],
    };
    expect(saveErrorCode(() => parseBindings(textScale))).toBe("corrupt");

    const badAxis = {
      format: BINDINGS_FORMAT,
      version: 1,
      actions: [{ action: "confirm", slots: [{ control: { type: "key", key: "Enter" }, axisComponent: "z" }] }],
    };
    expect(saveErrorCode(() => parseBindings(badAxis))).toBe("corrupt");
  });

  it("refuses an action without a name, and slots that are not a list", () => {
    const noName = { format: BINDINGS_FORMAT, version: 1, actions: [{ action: "", slots: [] }] };
    const notList = { format: BINDINGS_FORMAT, version: 1, actions: [{ action: "confirm", slots: "Enter" }] };
    expect(saveErrorCode(() => parseBindings(noName))).toBe("corrupt");
    expect(saveErrorCode(() => parseBindings(notList))).toBe("corrupt");
  });
});

describe("migration", () => {
  it("migrates an older file with the migrations that are passed in", () => {
    const file = parseBindings(JSON.parse(readFixture("v1.json")) as unknown, { version: 2, migrations: { 1: toVersionTwo } });
    expect(file.version).toBe(2);
    expect(file.actions[0]?.slots[0]?.context).toBe("menu");
    expect(file.actions[2]?.slots[1]?.context).toBe("gameplay");
  });

  it("refuses an older file when a step has no migration", () => {
    const migrations: MigrationMap = { 2: toVersionTwo };
    expect(saveErrorCode(() => parseBindings(readFixture("v1.json"), { version: 3, migrations }))).toBe(
      "missing-migration",
    );
  });
});

describe("applyBindings", () => {
  it("sets the slots of the listed actions, and counts the slots of an unlisted action as ignored", () => {
    const map = new ActionMap<Action>([{ action: "attack", control: { type: "key", key: "KeyX" } }]);
    const result = applyBindings(map, parseBindings(readFixture("v1.json")), KNOWN);
    expect(result).toEqual({ applied: 9, ignored: 1 });
    expect(map.slotsOf("attack").map((binding) => binding?.control)).toEqual([
      { type: "mouse", button: "left" },
      { type: "pad-button", button: "rightTrigger", threshold: 0.6 },
    ]);
    expect(map.slotsOf("pause")).toEqual([]);
  });

  it("clears the slots of a listed action that the file leaves empty", () => {
    const map = new ActionMap<Action>([{ action: "confirm", control: { type: "key", key: "Enter" } }]);
    const file: BindingsFile = { format: BINDINGS_FORMAT, version: 1, actions: [{ action: "confirm", slots: [] }] };
    expect(applyBindings(map, file, KNOWN)).toEqual({ applied: 0, ignored: 0 });
    expect(map.slotsOf("confirm")).toEqual([]);
  });

  it("keeps the conflicts that a file brings, for the game to see in conflicts()", () => {
    const map = new ActionMap<Action>([
      { action: "attack", control: { type: "key", key: "KeyZ" } },
      { action: "confirm", control: { type: "key", key: "Enter" } },
    ]);
    const file: BindingsFile = {
      format: BINDINGS_FORMAT,
      version: 1,
      actions: [{ action: "confirm", slots: [{ control: { type: "key", key: "KeyZ" } }] }],
    };
    applyBindings(map, file, KNOWN);
    expect(map.conflicts()).toHaveLength(1);
  });

  it("gives a fresh map the same slots that the saved map had", () => {
    const source = new ActionMap<Action>([
      { action: "confirm", control: { type: "key", key: "Enter" } },
      { action: "confirm", control: { type: "key", key: "Space" } },
      { action: "move", control: { type: "key", key: "KeyW" }, axisComponent: "y" },
      { action: "attack", control: { type: "mouse", button: "left" }, context: "gameplay" },
    ]);
    source.rebind("confirm", 1, { action: "confirm", control: { type: "pad-button", button: "a", pad: 1 } });
    source.clearSlot("move", 0);
    source.rebind("move", 2, { action: "move", control: { type: "pad-axis", axis: "leftX" }, axisComponent: "x" });

    const target = new ActionMap<Action>();
    applyBindings(target, parseBindings(JSON.stringify(serializeBindings(source))), KNOWN);
    for (const action of KNOWN) expect(target.slotsOf(action)).toEqual(source.slotsOf(action));
  });
});

describe("createBindingsStore", () => {
  it("keeps the file in the bridge's settings when the desktop host is present", async () => {
    const { bridge } = createFakeBridge();
    vi.stubGlobal("velvet", bridge);
    const store = createBindingsStore();
    expect(store.backendKind).toBe("bridge");
    expect(store.key).toBe(BINDINGS_KEY);

    const map = new ActionMap<Action>([{ action: "confirm", control: { type: "key", key: "Enter" } }]);
    await store.save(map);
    expect(await bridge.settings.get(BINDINGS_KEY)).toMatchObject({ format: BINDINGS_FORMAT, version: 1 });

    const fresh = new ActionMap<Action>();
    await expect(store.load(fresh, KNOWN)).resolves.toEqual({ applied: 1, ignored: 0 });
    expect(fresh.slotsOf("confirm")[0]?.control).toEqual({ type: "key", key: "Enter" });
  });

  it("falls back to localStorage, as JSON text under the game's settings prefix", async () => {
    vi.stubGlobal("velvet", undefined);
    const storage = new FakeStorage();
    vi.stubGlobal("localStorage", storage);
    const store = createBindingsStore();
    expect(store.backendKind).toBe("localStorage");

    const map = new ActionMap<Action>([{ action: "pause", control: { type: "key", key: "Escape" } }]);
    await store.save(map);
    expect(storage.keys()).toEqual([`velvet-input:${BINDINGS_KEY}`]);
    expect(JSON.parse(storage.getItem(`velvet-input:${BINDINGS_KEY}`) ?? "null")).toMatchObject({ format: BINDINGS_FORMAT });

    const fresh = new ActionMap<Action>();
    await expect(store.load(fresh, KNOWN)).resolves.toEqual({ applied: 1, ignored: 0 });
  });

  it("keeps the file in memory when there is neither a bridge nor localStorage", async () => {
    vi.stubGlobal("velvet", undefined);
    vi.stubGlobal("localStorage", undefined);
    const store = createBindingsStore();
    expect(store.backendKind).toBe("memory");
    const map = new ActionMap<Action>([{ action: "pause", control: { type: "key", key: "Escape" } }]);
    await store.save(map);
    await expect(store.load(new ActionMap<Action>(), KNOWN)).resolves.toEqual({ applied: 1, ignored: 0 });
  });

  it("uses the settings backend that is passed in", async () => {
    const values = new Map<string, unknown>();
    const settings: SettingsBackend = {
      get: async (key) => values.get(key) ?? null,
      set: async (key, value) => {
        values.set(key, value);
      },
    };
    const store = createBindingsStore({ settings, key: "custom.bindings" });
    expect(store.backendKind).toBe("custom");
    await store.save(new ActionMap<Action>([{ action: "pause", control: { type: "key", key: "KeyP" } }]));
    expect(values.has("custom.bindings")).toBe(true);
  });

  it("forgets the saved bindings on clear, and reads nothing from an empty slot", async () => {
    vi.stubGlobal("velvet", undefined);
    vi.stubGlobal("localStorage", new FakeStorage());
    const store = createBindingsStore();
    await expect(store.load(new ActionMap<Action>(), KNOWN)).resolves.toBeNull();
    await store.save(new ActionMap<Action>([{ action: "pause", control: { type: "key", key: "KeyP" } }]));
    await store.clear();
    await expect(store.load(new ActionMap<Action>(), KNOWN)).resolves.toBeNull();
  });

  it("refuses a saved file that is corrupt, and reads an older one through its migrations", async () => {
    const corrupt: SettingsBackend = { get: async () => "not json", set: async () => undefined };
    const broken = createBindingsStore({ settings: corrupt });
    await expect(broken.load(new ActionMap<Action>(), KNOWN)).rejects.toBeInstanceOf(SaveError);

    const older = JSON.parse(readFixture("v1.json")) as unknown;
    const stored: SettingsBackend = { get: async () => older, set: async () => undefined };
    const store = createBindingsStore({ settings: stored, version: 2, migrations: { 1: toVersionTwo } });
    const map = new ActionMap<Action>();
    await store.load(map, KNOWN);
    expect(map.slotsOf("confirm")[0]?.context).toBe("menu");
  });

  it("writes the version it was given", async () => {
    let written: unknown = null;
    const capture: SettingsBackend = {
      get: async () => null,
      set: async (_key, value) => {
        written = value;
      },
    };
    const store = createBindingsStore({ settings: capture, version: 2 });
    await store.save(new ActionMap<Action>());
    expect(written).toMatchObject({ format: BINDINGS_FORMAT, version: 2 });
  });

  it("rejects an empty key and a version below 1", () => {
    expect(() => createBindingsStore({ key: " " })).toThrow(RangeError);
    expect(() => createBindingsStore({ version: 0 })).toThrow(RangeError);
  });
});

/** The version 1 to version 2 migration that the store tests use: menu context on every slot. */
const toVersionTwo: Migration = (data) => {
  const file = data as { actions: Array<{ action: string; slots: Array<Record<string, unknown> | null> }> };
  return {
    format: BINDINGS_FORMAT,
    version: 2,
    actions: file.actions.map((entry) => ({
      action: entry.action,
      slots: entry.slots.map((slot) => (slot === null ? null : { context: "menu", ...slot })),
    })),
  };
};
