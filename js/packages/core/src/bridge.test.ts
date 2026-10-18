// Tests for the bridge helpers: the slot-name rule, the shape check, and the lookup of
// window.velvet.

import { afterEach, describe, expect, it, vi } from "vitest";
import { SLOT_PATTERN, getBridge, isValidSlot, isVelvetBridge } from "./bridge";
import { createFakeBridge } from "./test-support";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("SLOT_PATTERN and isValidSlot", () => {
  it("accepts letters, digits, underscores and dashes, up to 64 characters", () => {
    for (const slot of ["slot1", "Auto_save-2", "a", "a".repeat(64)]) {
      expect(SLOT_PATTERN.test(slot)).toBe(true);
      expect(isValidSlot(slot)).toBe(true);
    }
  });

  it("rejects empty names, separators, dots, spaces, accents and names over 64 characters", () => {
    for (const slot of ["", "a/b", "..", "a.json", "a b", "niño", "a".repeat(65)]) {
      expect(SLOT_PATTERN.test(slot)).toBe(false);
      expect(isValidSlot(slot)).toBe(false);
    }
  });
});

describe("isVelvetBridge", () => {
  it("accepts an object with the whole contract", () => {
    const { bridge } = createFakeBridge();
    expect(isVelvetBridge(bridge)).toBe(true);
  });

  it("rejects values that are not objects", () => {
    for (const value of [null, undefined, "desktop", 3, []]) {
      expect(isVelvetBridge(value)).toBe(false);
    }
  });

  it("rejects another platform", () => {
    const { bridge } = createFakeBridge();
    const candidate: unknown = { ...bridge, platform: "browser" };
    expect(isVelvetBridge(candidate)).toBe(false);
  });

  it("rejects a bridge that lacks a save method, a game field or the log function", () => {
    const { bridge } = createFakeBridge();
    const withoutRemove: unknown = {
      ...bridge,
      saves: { list: bridge.saves.list, read: bridge.saves.read, write: bridge.saves.write },
    };
    const withoutTitle: unknown = { ...bridge, game: { id: "demo", version: "0.1.0" } };
    const withoutLog: unknown = { ...bridge, log: "not a function" };
    expect(isVelvetBridge(withoutRemove)).toBe(false);
    expect(isVelvetBridge(withoutTitle)).toBe(false);
    expect(isVelvetBridge(withoutLog)).toBe(false);
  });
});

describe("getBridge", () => {
  it("returns null when window.velvet is missing", () => {
    vi.stubGlobal("velvet", undefined);
    expect(getBridge()).toBeNull();
  });

  it("returns the injected bridge", () => {
    const { bridge } = createFakeBridge();
    vi.stubGlobal("velvet", bridge);
    expect(getBridge()).toBe(bridge);
  });

  it("ignores a global that only partly looks like the bridge", () => {
    vi.stubGlobal("velvet", { platform: "desktop" });
    expect(getBridge()).toBeNull();
  });
});
