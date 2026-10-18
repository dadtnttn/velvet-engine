import { ActionMap } from "@velvet/core";
import { afterEach, describe, expect, it } from "vitest";
import { TestingError } from "./errors";
import { FakeGamepads, readNavigatorPads } from "./fake-pads";

const providers: FakeGamepads[] = [];

/** A provider that is installed for one test, and removed afterwards. */
function providerFor(): FakeGamepads {
  const provider = new FakeGamepads();
  providers.push(provider);
  return provider;
}

afterEach(() => {
  for (const provider of providers.splice(0)) provider.clear();
  // Each test installs and removes its own provider, so nothing is left on the navigator.
  expect(Reflect.has(navigator, "getGamepads")).toBe(false);
});

describe("FakeGamepads.install", () => {
  it("makes navigator.getGamepads return the pads, and removes them when restored", () => {
    const provider = providerFor();
    const restore = provider.install();
    expect(provider.installed).toBe(true);
    provider.connect(0, { id: "Test pad" });
    const getGamepads = Reflect.get(navigator, "getGamepads") as () => Array<{ index: number } | null>;
    expect(getGamepads()[0]?.index).toBe(0);
    restore();
    expect(provider.installed).toBe(false);
    expect(Reflect.has(navigator, "getGamepads")).toBe(false);
  });

  it("keeps the other provider's pads when an earlier one is removed first", () => {
    const first = providerFor();
    const second = providerFor();
    const restoreFirst = first.install();
    const restoreSecond = second.install();
    second.connect(1, { id: "Second" });
    const getGamepads = Reflect.get(navigator, "getGamepads") as () => Array<{ id: string } | null>;
    expect(getGamepads()[1]?.id).toBe("Second");
    restoreFirst();
    expect(getGamepads()[1]?.id).toBe("Second");
    restoreSecond();
    expect(Reflect.has(navigator, "getGamepads")).toBe(false);
  });

  it("does nothing when installed twice, and restores once", () => {
    const provider = providerFor();
    const restore = provider.install();
    expect(provider.install()).toBe(restore);
    restore();
    restore();
    expect(Reflect.has(navigator, "getGamepads")).toBe(false);
  });
});

describe("the pads that an ActionMap reads", () => {
  it("presses a button binding while the pad button is down, and releases it after", () => {
    const provider = providerFor();
    const restore = provider.install();
    try {
      const map = new ActionMap<"fire">([{ action: "fire", control: { type: "pad-button", button: "a" } }]);
      provider.connect(0);
      provider.setButton(0, "a", 1);
      // The pads are read inside endFrame, so the press is an edge of that frame.
      map.endFrame();
      expect(map.pressed("fire")).toBe(true);
      expect(map.held("fire")).toBe(true);
      map.endFrame();
      expect(map.pressed("fire")).toBe(false);
      expect(map.held("fire")).toBe(true);
      provider.setButton(0, "a", 0);
      map.endFrame();
      expect(map.released("fire")).toBe(true);
      expect(map.held("fire")).toBe(false);
    } finally {
      restore();
    }
  });

  it("reads an axis after its deadzone, and a stick as a vector", () => {
    const provider = providerFor();
    const restore = provider.install();
    try {
      const map = new ActionMap<"move">([
        { action: "move", control: { type: "pad-axis", axis: "leftX" }, axisComponent: "x" },
        { action: "move", control: { type: "pad-axis", axis: "leftY" }, axisComponent: "y" },
      ]);
      provider.connect(0);
      provider.setStick(0, "left", 1, 0);
      map.endFrame();
      expect(map.axis2("move")).toEqual({ x: 1, y: 0 });
      provider.setStick(0, "left", 0.1, 0);
      map.endFrame();
      expect(map.axis2("move").x).toBe(0);
    } finally {
      restore();
    }
  });

  it("sees a disconnected pad as released", () => {
    const provider = providerFor();
    const restore = provider.install();
    try {
      const map = new ActionMap<"fire">([{ action: "fire", control: { type: "pad-button", button: "b" } }]);
      provider.connect(1);
      provider.setButton(1, "b", 1);
      map.endFrame();
      expect(map.held("fire")).toBe(true);
      provider.disconnect(1);
      map.endFrame();
      expect(map.released("fire")).toBe(true);
    } finally {
      restore();
    }
  });
});

describe("FakeGamepads state", () => {
  it("refuses a pad index out of range, a value out of range, and a change to a pad that is not connected", () => {
    const provider = providerFor();
    expect(() => provider.connect(16)).toThrow(TestingError);
    provider.connect(0);
    expect(() => provider.setButton(0, "a", 1.5)).toThrow(/from 0 to 1/);
    expect(() => provider.setAxis(0, "leftX", 2)).toThrow(/from -1 to 1/);
    expect(() => provider.setButton(2, "a", 1)).toThrow(/not connected/);
  });

  it("gives the state of a pad, and nothing for an empty place", () => {
    const provider = providerFor();
    provider.connect(0, { id: "Pad A", buttons: [{ pressed: true, value: 1 }] });
    const state = provider.state(0);
    expect(state?.id).toBe("Pad A");
    expect(state?.buttons[0]).toEqual({ pressed: true, value: 1 });
    expect(provider.state(1)).toBeNull();
    provider.disconnect(0);
    expect(provider.state(0)).toBeNull();
  });

  it("reads the connected pads that the navigator reports, with copies of their values", () => {
    const provider = providerFor();
    const restore = provider.install();
    try {
      provider.connect(0, { id: "First", axes: [0.5, 0, 0, 0] });
      provider.connect(2, { id: "Third" });
      const pads = readNavigatorPads();
      expect(pads.map((pad) => pad.index)).toEqual([0, 2]);
      expect(pads[0]?.axes[0]).toBe(0.5);
      expect(pads[0]?.buttons).toHaveLength(17);
      provider.disconnect(0);
      expect(readNavigatorPads().map((pad) => pad.index)).toEqual([2]);
    } finally {
      restore();
    }
  });

  it("reads no pads where there is no navigator.getGamepads", () => {
    expect(readNavigatorPads()).toEqual([]);
  });
});
