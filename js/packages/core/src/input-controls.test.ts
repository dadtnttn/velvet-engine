// Tests for the control helpers: the keys that name a control, the display data with its i18n key ids,
// and the strict reading of a control from untrusted data.

import { describe, expect, it } from "vitest";
import { conflictKeyOf, controlKey, describeBinding, readControl, isHandled, markHandled } from "./input-controls";
import type { Control } from "./input";

describe("describeBinding", () => {
  it("gives the device, code and i18n key of a key, a mouse button and a wheel direction", () => {
    expect(describeBinding({ control: { type: "key", key: "Space" } })).toEqual({
      device: "keyboard",
      code: "Space",
      labelKey: "input.key.Space",
    });
    expect(describeBinding({ control: { type: "mouse", button: "left" } })).toEqual({
      device: "mouse",
      code: "left",
      labelKey: "input.mouse.left",
    });
    expect(describeBinding({ control: { type: "wheel", direction: "up" } })).toEqual({
      device: "mouse",
      code: "up",
      labelKey: "input.wheel.up",
    });
  });

  it("gives the i18n key of a pad button, an axis and a stick direction", () => {
    expect(describeBinding({ control: { type: "pad-button", button: "a" } })).toEqual({
      device: "gamepad",
      code: "a",
      labelKey: "input.pad.a",
    });
    expect(describeBinding({ control: { type: "pad-axis", axis: "rightY" } })).toEqual({
      device: "gamepad",
      code: "rightY",
      labelKey: "input.pad.axis.rightY",
    });
    expect(describeBinding({ control: { type: "pad-stick", stick: "left", direction: "up" } })).toEqual({
      device: "gamepad",
      code: "left.up",
      labelKey: "input.pad.stick.left.up",
    });
  });

  it("marks an inverted axis, and leaves the mark off a normal one", () => {
    expect(describeBinding({ control: { type: "pad-axis", axis: "leftX" }, scale: -1 })).toEqual({
      device: "gamepad",
      code: "leftX",
      labelKey: "input.pad.axis.leftX",
      negative: true,
    });
    expect(describeBinding({ control: { type: "pad-axis", axis: "leftX" } })).not.toHaveProperty("negative");
  });
});

describe("controlKey and conflictKeyOf", () => {
  it("names each control, and tells the pads apart by their index", () => {
    expect(controlKey({ type: "key", key: "KeyW" })).toBe("key:KeyW");
    expect(controlKey({ type: "mouse", button: "right" })).toBe("mouse:right");
    expect(controlKey({ type: "wheel", direction: "down" })).toBe("wheel:down");
    expect(controlKey({ type: "pad-button", button: "a" })).toBe("pad-button:*:a");
    expect(controlKey({ type: "pad-button", button: "a", pad: 0 })).toBe("pad-button:0:a");
    expect(controlKey({ type: "pad-stick", stick: "right", direction: "left" })).toBe("pad-stick:*:right:left");
  });

  it("separates the two directions of an axis for conflicts, and not for other controls", () => {
    const axis: Control = { type: "pad-axis", axis: "leftX" };
    expect(conflictKeyOf(axis, 1)).not.toBe(conflictKeyOf(axis, -1));
    expect(conflictKeyOf({ type: "key", key: "KeyA" }, -1)).toBe("key:KeyA");
  });
});

describe("readControl", () => {
  it("reads a valid control into a new object with only the fields of its type", () => {
    const read = readControl({ type: "pad-stick", stick: "right", direction: "down", pad: 1, deadzone: 0.3, extra: "x" });
    expect(read).toEqual({ type: "pad-stick", stick: "right", direction: "down", pad: 1, deadzone: 0.3 });
    expect(read).not.toHaveProperty("extra");
  });

  it("reads the keys, buttons and directions that the standard names allow", () => {
    expect(readControl({ type: "key", key: "F12" })).toEqual({ type: "key", key: "F12" });
    expect(readControl({ type: "mouse", button: "middle" })).toEqual({ type: "mouse", button: "middle" });
    expect(readControl({ type: "wheel", direction: "left" })).toEqual({ type: "wheel", direction: "left" });
    expect(readControl({ type: "pad-button", button: "dpadRight", threshold: 0.7 })).toEqual({
      type: "pad-button",
      button: "dpadRight",
      threshold: 0.7,
    });
    expect(readControl({ type: "pad-axis", axis: "leftY" })).toEqual({ type: "pad-axis", axis: "leftY" });
  });

  it("rejects values that are not controls, or that have a field out of range", () => {
    const bad: unknown[] = [
      null,
      "key",
      { type: "joystick" },
      { type: "key", key: "Key W" },
      { type: "key", key: "" },
      { type: "mouse", button: "back" },
      { type: "wheel", direction: "forward" },
      { type: "pad-button", button: "dpadCenter" },
      { type: "pad-button", button: "a", pad: -1 },
      { type: "pad-button", button: "a", pad: 1.5 },
      { type: "pad-button", button: "a", threshold: 0 },
      { type: "pad-button", button: "a", threshold: 1.5 },
      { type: "pad-axis", axis: "leftZ" },
      { type: "pad-axis", axis: "leftX", deadzone: 1 },
      { type: "pad-axis", axis: "leftX", deadzone: -0.1 },
      { type: "pad-stick", stick: "left", direction: "diagonal" },
      { type: "pad-stick", stick: "middle", direction: "up" },
    ];
    for (const value of bad) {
      expect(() => readControl(value), JSON.stringify(value)).toThrow(RangeError);
    }
  });
});

describe("markHandled and isHandled", () => {
  it("marks one event, and no other", () => {
    const marked = new Event("keydown");
    const other = new Event("keydown");
    expect(isHandled(marked)).toBe(false);
    markHandled(marked);
    expect(isHandled(marked)).toBe(true);
    expect(isHandled(other)).toBe(false);
  });
});
