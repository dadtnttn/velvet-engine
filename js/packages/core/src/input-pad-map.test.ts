// Tests for gamepads in the action map: the standard mapping read once per frame, thresholds and
// deadzones, pad indexes, hot-plug, the last device used, analog values and rumble. The pads are
// synthetic: a stubbed navigator.getGamepads returns objects that the test changes between frames.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ActionMap, type GamepadChange, type InputDevice } from "./input";
import { FakePads, fakePad } from "./input-test-support";

type Action = "confirm" | "move" | "fire" | "steer" | "up" | "left" | "quit";

const pads = new FakePads();

beforeEach(() => {
  pads.set([]);
  pads.install();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

/** A key event as the browser sends it, carrying the fields the map reads. */
function keyEvent(type: "keydown" | "keyup", code: string): Event {
  return Object.assign(new Event(type), { code, repeat: false });
}

/** A pointer press on the left button. */
function pointerDown(): Event {
  return Object.assign(new Event("pointerdown"), { button: 0, clientX: 0, clientY: 0 });
}

describe("gamepad buttons", () => {
  it("reports a press at the frame that reads the pad, and holds it until a frame reads the release", () => {
    const map = new ActionMap<Action>([{ action: "confirm", control: { type: "pad-button", button: "a" } }]);
    pads.set([fakePad(0, { buttons: { 0: 1 } })]);
    map.endFrame();
    expect(map.pressed("confirm")).toBe(true);
    expect(map.held("confirm")).toBe(true);

    map.endFrame();
    expect(map.pressed("confirm")).toBe(false);
    expect(map.held("confirm")).toBe(true);

    pads.set([fakePad(0)]);
    map.endFrame();
    expect(map.released("confirm")).toBe(true);
    expect(map.held("confirm")).toBe(false);
  });

  it("reads the pads only when a frame ends, not when they change", () => {
    const map = new ActionMap<Action>([{ action: "confirm", control: { type: "pad-button", button: "a" } }]);
    pads.set([fakePad(0, { buttons: { 0: 1 } })]);
    expect(map.held("confirm")).toBe(false);
    map.endFrame();
    expect(map.held("confirm")).toBe(true);
  });

  it("uses a trigger as a button at its threshold, and reads its analog pull below it", () => {
    const map = new ActionMap<Action>([
      { action: "fire", control: { type: "pad-button", button: "rightTrigger", threshold: 0.6 } },
    ]);
    pads.set([fakePad(0, { buttons: { 7: 0.4 } })]);
    map.endFrame();
    expect(map.held("fire")).toBe(false);
    expect(map.value("fire")).toBeCloseTo(0.4);

    pads.set([fakePad(0, { buttons: { 7: 0.8 } })]);
    map.endFrame();
    expect(map.held("fire")).toBe(true);
    expect(map.value("fire")).toBeCloseTo(0.8);
  });

  it("reads a pad index for one pad, and an unset index for every pad", () => {
    const map = new ActionMap<Action>([
      { action: "confirm", control: { type: "pad-button", button: "a", pad: 1 } },
      { action: "fire", control: { type: "pad-button", button: "a" } },
    ]);
    pads.set([fakePad(0, { buttons: { 0: 1 } }), fakePad(1)]);
    map.endFrame();
    expect(map.held("confirm")).toBe(false);
    expect(map.held("fire")).toBe(true);

    pads.set([fakePad(0), fakePad(1, { buttons: { 0: 1 } })]);
    map.endFrame();
    expect(map.held("confirm")).toBe(true);
    expect(map.held("fire")).toBe(true);
  });

  it("reads a d-pad button like any other button", () => {
    const map = new ActionMap<Action>([{ action: "up", control: { type: "pad-button", button: "dpadUp" } }]);
    pads.set([fakePad(0, { buttons: { 12: 1 } })]);
    map.endFrame();
    expect(map.held("up")).toBe(true);
  });
});

describe("sticks and axes", () => {
  it("feeds an axis component with the axis, its default deadzone removed", () => {
    const map = new ActionMap<Action>([
      { action: "move", control: { type: "pad-axis", axis: "leftX" }, axisComponent: "x" },
    ]);
    pads.set([fakePad(0, { axes: [0.1, 0, 0, 0] })]);
    map.endFrame();
    expect(map.axis1("move")).toBe(0);

    pads.set([fakePad(0, { axes: [0.7, 0, 0, 0] })]);
    map.endFrame();
    expect(map.axis1("move")).toBeCloseTo(0.625); // (0.7 - 0.2) / 0.8
    expect(map.held("move")).toBe(true);

    pads.set([fakePad(0, { axes: [-1, 0, 0, 0] })]);
    map.endFrame();
    expect(map.axis1("move")).toBeCloseTo(-1);
  });

  it("uses the deadzone of each binding", () => {
    const map = new ActionMap<Action>([
      { action: "move", control: { type: "pad-axis", axis: "leftX", deadzone: 0.5 }, axisComponent: "x" },
      { action: "steer", control: { type: "pad-axis", axis: "leftX" }, axisComponent: "x" },
    ]);
    pads.set([fakePad(0, { axes: [0.4, 0, 0, 0] })]);
    map.endFrame();
    expect(map.axis1("move")).toBe(0);
    expect(map.axis1("steer")).toBeCloseTo(0.25); // (0.4 - 0.2) / 0.8
  });

  it("reads a stick direction as a button, with a round deadzone", () => {
    const map = new ActionMap<Action>([{ action: "up", control: { type: "pad-stick", stick: "left", direction: "up" } }]);
    pads.set([fakePad(0, { axes: [0, -0.9, 0, 0] })]);
    map.endFrame();
    expect(map.held("up")).toBe(true);

    pads.set([fakePad(0, { axes: [0, 0.9, 0, 0] })]);
    map.endFrame();
    expect(map.held("up")).toBe(false);

    pads.set([fakePad(0, { axes: [0.1, -0.1, 0, 0] })]);
    map.endFrame();
    expect(map.held("up")).toBe(false);
  });

  it("inverts an axis with a negative scale", () => {
    const map = new ActionMap<Action>([{ action: "left", control: { type: "pad-axis", axis: "leftX" }, scale: -1 }]);
    pads.set([fakePad(0, { axes: [-0.8, 0, 0, 0] })]);
    map.endFrame();
    expect(map.held("left")).toBe(true);
    expect(map.value("left")).toBeCloseTo(0.75);

    pads.set([fakePad(0, { axes: [0.8, 0, 0, 0] })]);
    map.endFrame();
    expect(map.held("left")).toBe(false);
    expect(map.value("left")).toBeCloseTo(-0.75);
  });
});

describe("analog values", () => {
  it("gives the key's scale while it is down, and zero when it is up", () => {
    const target = new EventTarget();
    const map = new ActionMap<Action>([{ action: "steer", control: { type: "key", key: "KeyQ" }, scale: 0.3 }]);
    map.attach(target);
    target.dispatchEvent(keyEvent("keydown", "KeyQ"));
    expect(map.value("steer")).toBeCloseTo(0.3);
    expect(map.held("steer")).toBe(false);
    target.dispatchEvent(keyEvent("keyup", "KeyQ"));
    expect(map.value("steer")).toBe(0);
  });

  it("reads zero for a consumed action", () => {
    const map = new ActionMap<Action>([{ action: "fire", control: { type: "pad-button", button: "rightTrigger" } }]);
    pads.set([fakePad(0, { buttons: { 7: 1 } })]);
    map.endFrame();
    map.consume("fire");
    expect(map.value("fire")).toBe(0);
  });
});

describe("hot-plug and the last device", () => {
  it("reports connections and disconnections, and a removed pad releases its buttons", () => {
    const map = new ActionMap<Action>([{ action: "confirm", control: { type: "pad-button", button: "a" } }]);
    const changes: GamepadChange[] = [];
    map.onGamepadChange((change) => changes.push(change));

    pads.set([fakePad(0, { id: "Pad A", buttons: { 0: 1 } })]);
    map.endFrame();
    expect(changes).toEqual([{ type: "connected", index: 0, id: "Pad A" }]);
    expect(map.held("confirm")).toBe(true);
    expect(map.connectedGamepads()).toEqual([{ index: 0, id: "Pad A" }]);

    pads.set([]);
    map.endFrame();
    expect(changes[1]).toEqual({ type: "disconnected", index: 0, id: "Pad A" });
    expect(map.held("confirm")).toBe(false);
    expect(map.released("confirm")).toBe(true);
    expect(map.connectedGamepads()).toEqual([]);
  });

  it("stops reporting to a listener that was removed", () => {
    const map = new ActionMap<Action>();
    const changes: GamepadChange[] = [];
    const stop = map.onGamepadChange((change) => changes.push(change));
    stop();
    pads.set([fakePad(0)]);
    map.endFrame();
    expect(changes).toEqual([]);
  });

  it("tracks the last device used, and reports only the changes", () => {
    const target = new EventTarget();
    const map = new ActionMap<Action>([
      { action: "confirm", control: { type: "pad-button", button: "a" } },
      { action: "fire", control: { type: "key", key: "KeyF" } },
    ]);
    map.attach(target);
    const used: InputDevice[] = [];
    map.onDeviceChange((device) => used.push(device));
    expect(map.lastDevice()).toBe("keyboard");

    target.dispatchEvent(pointerDown());
    expect(map.lastDevice()).toBe("mouse");

    pads.set([fakePad(0, { buttons: { 0: 1 } })]);
    map.endFrame();
    expect(map.lastDevice()).toBe("gamepad");

    target.dispatchEvent(keyEvent("keydown", "KeyF"));
    expect(map.lastDevice()).toBe("keyboard");
    map.endFrame();
    expect(used).toEqual(["mouse", "gamepad", "keyboard"]);
    map.detach();
  });

  it("counts a stick pushed past half way as the gamepad in use", () => {
    const map = new ActionMap<Action>();
    pads.set([fakePad(0, { axes: [0.9, 0, 0, 0] })]);
    map.endFrame();
    expect(map.lastDevice()).toBe("gamepad");
  });
});

describe("rumble", () => {
  it("plays a dual-rumble effect on the first pad that can vibrate", async () => {
    const playEffect = vi.fn(async () => undefined);
    pads.set([fakePad(0), fakePad(1, { vibrationActuator: { playEffect } })]);
    const map = new ActionMap<Action>();
    map.endFrame();
    await expect(map.rumble({ durationMs: 300, strong: 1, weak: 0.25 })).resolves.toBe(true);
    expect(playEffect).toHaveBeenCalledWith("dual-rumble", {
      duration: 300,
      strongMagnitude: 1,
      weakMagnitude: 0.25,
      startDelay: 0,
    });
  });

  it("uses the pad that the option names", async () => {
    const playEffect = vi.fn(async () => undefined);
    pads.set([fakePad(2, { vibrationActuator: { playEffect } })]);
    const map = new ActionMap<Action>();
    map.endFrame();
    await expect(map.rumble({ pad: 1 })).resolves.toBe(false);
    await expect(map.rumble({ pad: 2 })).resolves.toBe(true);
  });

  it("resolves to false with no pad, with a pad that cannot vibrate, and when the browser refuses", async () => {
    const map = new ActionMap<Action>();
    await expect(map.rumble()).resolves.toBe(false);

    pads.set([fakePad(0)]);
    map.endFrame();
    await expect(map.rumble()).resolves.toBe(false);

    const refuse = vi.fn(async () => {
      throw new Error("refused");
    });
    pads.set([fakePad(0, { vibrationActuator: { playEffect: refuse } })]);
    map.endFrame();
    await expect(map.rumble()).resolves.toBe(false);
    expect(refuse).toHaveBeenCalledTimes(1);
  });
});

describe("pads and the navigator", () => {
  it("keeps the keys working when the page has no gamepad API", () => {
    vi.unstubAllGlobals();
    const target = new EventTarget();
    const map = new ActionMap<Action>([{ action: "quit", control: { type: "key", key: "KeyQ" } }]);
    map.attach(target);
    map.endFrame();
    target.dispatchEvent(keyEvent("keydown", "KeyQ"));
    expect(map.held("quit")).toBe(true);
    expect(map.connectedGamepads()).toEqual([]);
    map.detach();
  });
});
