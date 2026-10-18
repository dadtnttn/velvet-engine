// Tests for the action map. The events are synthetic: plain Event objects that carry the
// fields the handlers read (code, button, clientX, deltaY and so on). They are dispatched on
// an EventTarget, so the attach and detach paths run for real.

import { describe, expect, it } from "vitest";
import { ActionMap, markHandled, type ActionMapOptions, type Binding } from "./input";

type Action = "confirm" | "attack" | "menu" | "zoom" | "move" | "half";

function keyEvent(type: "keydown" | "keyup", code: string): Event {
  return Object.assign(new Event(type), { code, repeat: false });
}

/** A key event that can be cancelled, as a real keyboard event is. */
function cancelableKeyEvent(type: "keydown" | "keyup", code: string): Event {
  return Object.assign(new Event(type, { cancelable: true }), { code, repeat: false });
}

function pointerEvent(
  type: "pointerdown" | "pointerup" | "pointermove" | "pointercancel",
  fields: { button?: number; clientX?: number; clientY?: number } = {},
): Event {
  return Object.assign(new Event(type), { button: 0, clientX: 0, clientY: 0, ...fields });
}

function wheelEvent(fields: { deltaX?: number; deltaY?: number; deltaMode?: number }): Event {
  return Object.assign(new Event("wheel"), { deltaX: 0, deltaY: 0, deltaMode: 0, ...fields });
}

/** A map attached to a fresh target, so that a test can dispatch events on it. */
function attachedMap(
  bindings: Binding<Action>[],
  options?: ActionMapOptions,
): { target: EventTarget; map: ActionMap<Action> } {
  const target = new EventTarget();
  const map = new ActionMap<Action>(bindings, options);
  map.attach(target);
  return { target, map };
}

const enter: Binding<Action> = { action: "confirm", control: { type: "key", key: "Enter" } };

describe("ActionMap keys", () => {
  it("reports a key press as pressed and held in the frame it arrives", () => {
    const { target, map } = attachedMap([enter]);
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    expect(map.pressed("confirm")).toBe(true);
    expect(map.held("confirm")).toBe(true);
    expect(map.released("confirm")).toBe(false);
    map.endFrame();
    expect(map.pressed("confirm")).toBe(false);
    expect(map.held("confirm")).toBe(true);
  });

  it("reports a key release as released and no longer held", () => {
    const { target, map } = attachedMap([enter]);
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    map.endFrame();
    target.dispatchEvent(keyEvent("keyup", "Enter"));
    expect(map.released("confirm")).toBe(true);
    expect(map.held("confirm")).toBe(false);
    expect(map.pressed("confirm")).toBe(false);
  });

  it("reports a tap between two frames as pressed and released in the same frame", () => {
    const { target, map } = attachedMap([enter]);
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    target.dispatchEvent(keyEvent("keyup", "Enter"));
    expect(map.pressed("confirm")).toBe(true);
    expect(map.released("confirm")).toBe(true);
    expect(map.held("confirm")).toBe(false);
    map.endFrame();
    expect(map.pressed("confirm")).toBe(false);
    expect(map.released("confirm")).toBe(false);
  });

  it("does not press the action again for a repeated keydown", () => {
    const { target, map } = attachedMap([enter]);
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    map.endFrame();
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    expect(map.pressed("confirm")).toBe(false);
    expect(map.held("confirm")).toBe(true);
  });

  it("counts two keys bound to one action as one action", () => {
    const { target, map } = attachedMap([
      enter,
      { action: "confirm", control: { type: "key", key: "Space" } },
    ]);
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    expect(map.pressed("confirm")).toBe(true);
    map.endFrame();

    target.dispatchEvent(keyEvent("keydown", "Space"));
    expect(map.pressed("confirm")).toBe(false);
    expect(map.held("confirm")).toBe(true);

    target.dispatchEvent(keyEvent("keyup", "Enter"));
    expect(map.released("confirm")).toBe(false);
    expect(map.held("confirm")).toBe(true);

    target.dispatchEvent(keyEvent("keyup", "Space"));
    expect(map.released("confirm")).toBe(true);
    expect(map.held("confirm")).toBe(false);
  });

  it("ignores keys that have no binding", () => {
    const { target, map } = attachedMap([enter]);
    target.dispatchEvent(keyEvent("keydown", "KeyQ"));
    expect(map.pressed("confirm")).toBe(false);
    expect(map.held("confirm")).toBe(false);
  });
});

describe("ActionMap mouse buttons", () => {
  it("maps the left and right buttons by name, and ignores buttons that are not bound", () => {
    const { target, map } = attachedMap([
      { action: "attack", control: { type: "mouse", button: "left" } },
      { action: "menu", control: { type: "mouse", button: "right" } },
    ]);
    target.dispatchEvent(pointerEvent("pointerdown", { button: 0 }));
    expect(map.pressed("attack")).toBe(true);
    expect(map.pressed("menu")).toBe(false);

    map.endFrame();
    target.dispatchEvent(pointerEvent("pointerup", { button: 0 }));
    expect(map.released("attack")).toBe(true);

    map.endFrame();
    target.dispatchEvent(pointerEvent("pointerdown", { button: 2 }));
    expect(map.pressed("menu")).toBe(true);

    map.endFrame();
    target.dispatchEvent(pointerEvent("pointerdown", { button: 1 }));
    expect(map.pressed("attack")).toBe(false);
    expect(map.pressed("menu")).toBe(false);
  });

  it("releases the mouse on pointercancel but leaves the keys alone", () => {
    const { target, map } = attachedMap([
      enter,
      { action: "attack", control: { type: "mouse", button: "left" } },
    ]);
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    target.dispatchEvent(pointerEvent("pointerdown", { button: 0 }));
    expect(map.held("attack")).toBe(true);
    target.dispatchEvent(pointerEvent("pointercancel"));
    expect(map.held("attack")).toBe(false);
    expect(map.released("attack")).toBe(true);
    expect(map.held("confirm")).toBe(true);
  });
});

describe("ActionMap wheel", () => {
  it("turns a wheel tick into a one-frame pulse", () => {
    const { target, map } = attachedMap([{ action: "zoom", control: { type: "wheel", direction: "up" } }]);
    target.dispatchEvent(wheelEvent({ deltaY: -120 }));
    expect(map.pressed("zoom")).toBe(true);
    expect(map.held("zoom")).toBe(true);
    expect(map.wheelDelta()).toEqual({ x: 0, y: -120 });

    map.endFrame();
    expect(map.pressed("zoom")).toBe(false);
    expect(map.released("zoom")).toBe(true);
    expect(map.held("zoom")).toBe(false);
    expect(map.wheelDelta()).toEqual({ x: 0, y: 0 });

    map.endFrame();
    expect(map.released("zoom")).toBe(false);
  });

  it("maps horizontal scrolling to left and right", () => {
    const { target, map } = attachedMap([{ action: "move", control: { type: "wheel", direction: "right" } }]);
    target.dispatchEvent(wheelEvent({ deltaX: 40 }));
    expect(map.pressed("move")).toBe(true);
    expect(map.wheelDelta()).toEqual({ x: 40, y: 0 });
  });

  it("converts lines and pages to pixels, and adds the ticks up until endFrame", () => {
    const { target, map } = attachedMap([]);
    target.dispatchEvent(wheelEvent({ deltaY: -100 }));
    target.dispatchEvent(wheelEvent({ deltaY: -20 }));
    expect(map.wheelDelta()).toEqual({ x: 0, y: -120 });
    map.endFrame();
    target.dispatchEvent(wheelEvent({ deltaY: 3, deltaMode: 1 })); // 3 lines = 48 px
    target.dispatchEvent(wheelEvent({ deltaY: 1, deltaMode: 2 })); // 1 page = 100 px
    expect(map.wheelDelta()).toEqual({ x: 0, y: 148 });
  });
});

describe("ActionMap pointer", () => {
  it("tracks the pointer position and the movement since the last frame", () => {
    const { target, map } = attachedMap([]);
    expect(map.pointer()).toEqual({ x: 0, y: 0 });

    target.dispatchEvent(pointerEvent("pointermove", { clientX: 10, clientY: 20 }));
    expect(map.pointer()).toEqual({ x: 10, y: 20 });
    expect(map.pointerDelta()).toEqual({ x: 0, y: 0 }); // the first position only sets the origin

    target.dispatchEvent(pointerEvent("pointermove", { clientX: 15, clientY: 25 }));
    expect(map.pointerDelta()).toEqual({ x: 5, y: 5 });

    map.endFrame();
    expect(map.pointerDelta()).toEqual({ x: 0, y: 0 });
    expect(map.pointer()).toEqual({ x: 15, y: 25 });
  });
});

describe("ActionMap axes", () => {
  const moveBindings: Binding<Action>[] = [
    { action: "move", control: { type: "key", key: "KeyW" }, axisComponent: "y" },
    { action: "move", control: { type: "key", key: "KeyS" }, axisComponent: "y", scale: -1 },
    { action: "move", control: { type: "key", key: "KeyA" }, axisComponent: "x", scale: -1 },
    { action: "move", control: { type: "key", key: "KeyD" }, axisComponent: "x" },
  ];

  it("builds a 2D axis from x and y bindings", () => {
    const { target, map } = attachedMap(moveBindings);
    target.dispatchEvent(keyEvent("keydown", "KeyW"));
    target.dispatchEvent(keyEvent("keydown", "KeyD"));
    expect(map.axis2("move")).toEqual({ x: 1, y: 1 });
    expect(map.axis1("move")).toBe(1);
    expect(map.held("move")).toBe(true);
    expect(map.pressed("move")).toBe(true);

    target.dispatchEvent(keyEvent("keyup", "KeyD"));
    expect(map.axis2("move")).toEqual({ x: 0, y: 1 });
    expect(map.held("move")).toBe(true);

    target.dispatchEvent(keyEvent("keyup", "KeyW"));
    expect(map.held("move")).toBe(false);
    expect(map.released("move")).toBe(true);
  });

  it("clamps each component to [-1, 1]", () => {
    const { target, map } = attachedMap([
      { action: "move", control: { type: "key", key: "KeyW" }, axisComponent: "y" },
      { action: "move", control: { type: "key", key: "ArrowUp" }, axisComponent: "y" },
    ]);
    target.dispatchEvent(keyEvent("keydown", "KeyW"));
    target.dispatchEvent(keyEvent("keydown", "ArrowUp"));
    expect(map.axis2("move")).toEqual({ x: 0, y: 1 });
  });

  it("keeps a digital binding below the threshold inactive, but still reports its value", () => {
    const { target, map } = attachedMap([{ action: "half", control: { type: "key", key: "KeyQ" }, scale: 0.3 }]);
    target.dispatchEvent(keyEvent("keydown", "KeyQ"));
    expect(map.held("half")).toBe(false);
    expect(map.axis1("half")).toBe(0.3);
    expect(map.axis2("half")).toEqual({ x: 0.3, y: 0 });
  });

  it("uses a custom threshold to decide when an action is active", () => {
    const { target, map } = attachedMap(
      [{ action: "half", control: { type: "key", key: "KeyQ" }, scale: 0.3 }],
      { threshold: 0.2 },
    );
    target.dispatchEvent(keyEvent("keydown", "KeyQ"));
    expect(map.held("half")).toBe(true);
  });
});

describe("ActionMap focus and lifecycle", () => {
  it("releases every control when the window loses focus", () => {
    const { target, map } = attachedMap([
      enter,
      { action: "move", control: { type: "key", key: "KeyW" }, axisComponent: "y" },
    ]);
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    target.dispatchEvent(keyEvent("keydown", "KeyW"));
    target.dispatchEvent(new Event("blur"));
    expect(map.held("confirm")).toBe(false);
    expect(map.released("confirm")).toBe(true);
    expect(map.held("move")).toBe(false);
    expect(map.released("move")).toBe(true);
  });

  it("detach releases the held controls and stops listening", () => {
    const { target, map } = attachedMap([enter]);
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    map.detach();
    expect(map.held("confirm")).toBe(false);
    expect(map.released("confirm")).toBe(true);
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    expect(map.held("confirm")).toBe(false);
  });

  it("attaching again moves the listeners to the new target", () => {
    const first = new EventTarget();
    const second = new EventTarget();
    const map = new ActionMap<Action>([enter]);
    map.attach(first);
    map.attach(second);
    first.dispatchEvent(keyEvent("keydown", "Enter"));
    expect(map.held("confirm")).toBe(false);
    second.dispatchEvent(keyEvent("keydown", "Enter"));
    expect(map.held("confirm")).toBe(true);
  });
});

describe("ActionMap bind and unbind", () => {
  it("unbind releases an action that was active", () => {
    const { target, map } = attachedMap([enter]);
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    map.unbind("confirm");
    expect(map.released("confirm")).toBe(true);
    expect(map.held("confirm")).toBe(false);
  });

  it("bind makes an action active at once when its control is already down", () => {
    const { target, map } = attachedMap([]);
    target.dispatchEvent(keyEvent("keydown", "KeyZ"));
    expect(map.held("attack")).toBe(false);
    map.bind({ action: "attack", control: { type: "key", key: "KeyZ" } });
    expect(map.pressed("attack")).toBe(true);
    expect(map.held("attack")).toBe(true);
  });

  it("rejects a scale that is not a finite number, and a threshold that is not positive", () => {
    expect(
      () => new ActionMap<Action>([{ action: "attack", control: { type: "key", key: "KeyZ" }, scale: Number.NaN }]),
    ).toThrow(RangeError);
    expect(() => new ActionMap<Action>([], { threshold: 0 })).toThrow(RangeError);
  });
});

describe("ActionMap consume", () => {
  it("reads a consumed action as idle until endFrame, then as it really is", () => {
    const { target, map } = attachedMap([enter]);
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    map.consume("confirm");
    expect(map.pressed("confirm")).toBe(false);
    expect(map.held("confirm")).toBe(false);
    expect(map.released("confirm")).toBe(false);

    map.endFrame();
    expect(map.held("confirm")).toBe(true); // the key is still down
    expect(map.pressed("confirm")).toBe(false); // the press belonged to the frame before

    target.dispatchEvent(keyEvent("keyup", "Enter"));
    expect(map.released("confirm")).toBe(true);
    expect(map.held("confirm")).toBe(false);
  });

  it("hides a press that arrives after the consume in the same frame", () => {
    const { target, map } = attachedMap([enter]);
    map.consume("confirm");
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    expect(map.pressed("confirm")).toBe(false);
    expect(map.held("confirm")).toBe(false);
    map.endFrame();
    expect(map.held("confirm")).toBe(true);
  });

  it("hides only the action that was consumed", () => {
    const { target, map } = attachedMap([
      enter,
      { action: "attack", control: { type: "mouse", button: "left" } },
    ]);
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    target.dispatchEvent(pointerEvent("pointerdown", { button: 0 }));
    map.consume("confirm");
    expect(map.pressed("confirm")).toBe(false);
    expect(map.pressed("attack")).toBe(true);
    expect(map.held("attack")).toBe(true);
  });

  it("reads the axes of a consumed action as zero, and restores them at endFrame", () => {
    const { target, map } = attachedMap([
      { action: "move", control: { type: "key", key: "KeyW" }, axisComponent: "y" },
      { action: "move", control: { type: "key", key: "KeyD" }, axisComponent: "x" },
    ]);
    target.dispatchEvent(keyEvent("keydown", "KeyW"));
    target.dispatchEvent(keyEvent("keydown", "KeyD"));
    map.consume("move");
    expect(map.axis2("move")).toEqual({ x: 0, y: 0 });
    expect(map.axis1("move")).toBe(0);
    map.endFrame();
    expect(map.axis2("move")).toEqual({ x: 1, y: 1 });
  });

  it("does not change what the map records for the next frame", () => {
    const { target, map } = attachedMap([enter]);
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    map.consume("confirm");
    map.endFrame();
    // The release that follows is recorded normally, even though the press was consumed.
    target.dispatchEvent(keyEvent("keyup", "Enter"));
    expect(map.released("confirm")).toBe(true);
  });
});

describe("ActionMap handled events", () => {
  it("does not count a keydown that was marked, and the mark does not prevent the default", () => {
    const { target, map } = attachedMap([enter]);
    const down = cancelableKeyEvent("keydown", "Enter");
    markHandled(down);
    target.dispatchEvent(down);
    expect(map.held("confirm")).toBe(false);
    expect(map.pressed("confirm")).toBe(false);
    expect(down.defaultPrevented).toBe(false);
  });

  it("still releases a key that counted, even when its keyup was marked", () => {
    const { target, map } = attachedMap([enter]);
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    expect(map.held("confirm")).toBe(true);
    const up = keyEvent("keyup", "Enter");
    markHandled(up);
    target.dispatchEvent(up);
    expect(map.released("confirm")).toBe(true);
    expect(map.held("confirm")).toBe(false);
  });

  it("does not count a keydown whose default a listener prevented before the map", () => {
    const target = new EventTarget();
    target.addEventListener("keydown", (event) => {
      event.preventDefault();
    });
    const map = new ActionMap<Action>([enter]);
    map.attach(target);
    target.dispatchEvent(cancelableKeyEvent("keydown", "Enter"));
    expect(map.held("confirm")).toBe(false);
  });

  it("counts a keydown whose default a listener prevented only after the map ran", () => {
    const target = new EventTarget();
    const map = new ActionMap<Action>([enter]);
    map.attach(target);
    target.addEventListener("keydown", (event) => {
      event.preventDefault();
    });
    target.dispatchEvent(cancelableKeyEvent("keydown", "Enter"));
    expect(map.held("confirm")).toBe(true);
  });

  it("does not turn a marked wheel tick into a pulse, and keeps the pointer position", () => {
    const { target, map } = attachedMap([{ action: "zoom", control: { type: "wheel", direction: "up" } }]);
    const tick = wheelEvent({ deltaY: -120 });
    markHandled(tick);
    target.dispatchEvent(tick);
    expect(map.pressed("zoom")).toBe(false);
    expect(map.wheelDelta()).toEqual({ x: 0, y: 0 });

    const press = pointerEvent("pointerdown", { button: 0, clientX: 30, clientY: 40 });
    markHandled(press);
    target.dispatchEvent(press);
    expect(map.pointer()).toEqual({ x: 30, y: 40 });
  });
});
