// Tests for rebinding on the action map: slots, the conflict policies (swap, refuse and allow), the
// contexts that keep conflicts apart, clearing, the defaults that resetAction and resetAll restore, and
// the conflict groups that a map reports.

import { describe, expect, it } from "vitest";
import { ActionMap, type Binding, type Control } from "./input";

type Action = "confirm" | "attack" | "move" | "pause";

/** A key event as the browser sends it, carrying the fields the map reads. */
function keyEvent(type: "keydown" | "keyup", code: string): Event {
  return Object.assign(new Event(type), { code, repeat: false });
}

/** A key binding for an action. */
function key(action: Action, code: string, extra: Partial<Binding<Action>> = {}): Binding<Action> {
  return { action, control: { type: "key", key: code }, ...extra };
}

describe("slots", () => {
  it("lists the bindings of an action in slot order, with an empty slot as null", () => {
    const map = new ActionMap<Action>([
      key("confirm", "Enter"),
      key("confirm", "Space", { slot: 2 }),
    ]);
    expect(map.slotsOf("confirm").map((binding) => (binding === null ? null : binding.control))).toEqual([
      { type: "key", key: "Enter" },
      null,
      { type: "key", key: "Space" },
    ]);
    expect(map.slotsOf("pause")).toEqual([]);
  });

  it("numbers the bindings of an action in the order they are bound", () => {
    const map = new ActionMap<Action>([key("confirm", "Enter"), key("confirm", "Space")]);
    const slots = map.slotsOf("confirm");
    expect(slots[0]?.slot).toBe(0);
    expect(slots[1]?.slot).toBe(1);
    expect(slots[1]?.control).toEqual({ type: "key", key: "Space" });
  });

  it("refuses a bind into a slot that is taken", () => {
    const map = new ActionMap<Action>([key("confirm", "Enter")]);
    expect(() => map.bind(key("confirm", "Space", { slot: 0 }))).toThrow(RangeError);
  });

  it("keeps the context and the axis of a binding in its slot", () => {
    const map = new ActionMap<Action>([
      { action: "move", control: { type: "key", key: "KeyW" }, axisComponent: "y", context: "gameplay" },
    ]);
    expect(map.slotsOf("move")[0]).toMatchObject({ scale: 1, axisComponent: "y", context: "gameplay" });
  });
});

describe("rebind without a conflict", () => {
  it("binds a free control into a slot, and the old control stops working", () => {
    const target = new EventTarget();
    const map = new ActionMap<Action>([key("confirm", "Enter")]);
    map.attach(target);
    const outcome = map.rebind("confirm", 0, key("confirm", "KeyZ"));
    expect(outcome).toEqual({ status: "bound", conflicts: [] });

    target.dispatchEvent(keyEvent("keydown", "Enter"));
    expect(map.held("confirm")).toBe(false);
    target.dispatchEvent(keyEvent("keyup", "Enter"));
    target.dispatchEvent(keyEvent("keydown", "KeyZ"));
    expect(map.held("confirm")).toBe(true);
    map.detach();
  });

  it("adds a slot past the last one, with the slots between left empty", () => {
    const map = new ActionMap<Action>([key("confirm", "Enter")]);
    map.rebind("confirm", 3, key("confirm", "KeyZ"));
    expect(map.slotsOf("confirm").map((binding) => binding?.slot)).toEqual([0, undefined, undefined, 3]);
  });

  it("uses the scale and axis of the binding it is given", () => {
    const map = new ActionMap<Action>([key("move", "KeyW", { axisComponent: "y" })]);
    map.rebind("move", 0, key("move", "KeyI", { axisComponent: "y", scale: 1 }));
    expect(map.slotsOf("move")[0]).toMatchObject({ control: { type: "key", key: "KeyI" }, axisComponent: "y" });
  });
});

describe("conflict policies", () => {
  it("refuses a conflict and leaves every slot as it was", () => {
    const map = new ActionMap<Action>([key("attack", "KeyZ"), key("confirm", "Enter")]);
    const outcome = map.rebind("confirm", 0, key("confirm", "KeyZ"), { policy: "refuse" });
    expect(outcome).toEqual({ status: "refused", conflicts: [{ action: "attack", slot: 0 }] });
    expect(map.slotsOf("confirm")[0]?.control).toEqual({ type: "key", key: "Enter" });
    expect(map.slotsOf("attack")[0]?.control).toEqual({ type: "key", key: "KeyZ" });
  });

  it("allows a conflict and keeps both actions on the control", () => {
    const target = new EventTarget();
    const map = new ActionMap<Action>([key("attack", "KeyZ"), key("confirm", "Enter")]);
    map.attach(target);
    const outcome = map.rebind("confirm", 0, key("confirm", "KeyZ"), { policy: "allow" });
    expect(outcome).toEqual({ status: "allowed", conflicts: [{ action: "attack", slot: 0 }] });
    target.dispatchEvent(keyEvent("keydown", "KeyZ"));
    expect(map.held("attack")).toBe(true);
    expect(map.held("confirm")).toBe(true);
    map.detach();
  });

  it("swaps by default: the other action takes the control that this slot held", () => {
    const target = new EventTarget();
    const map = new ActionMap<Action>([key("attack", "KeyZ"), key("confirm", "Enter")]);
    map.attach(target);
    const outcome = map.rebind("confirm", 0, key("confirm", "KeyZ"));
    expect(outcome).toEqual({ status: "swapped", conflicts: [{ action: "attack", slot: 0 }] });
    expect(map.slotsOf("attack")[0]?.control).toEqual({ type: "key", key: "Enter" });
    expect(map.slotsOf("confirm")[0]?.control).toEqual({ type: "key", key: "KeyZ" });

    target.dispatchEvent(keyEvent("keydown", "Enter"));
    expect(map.held("attack")).toBe(true);
    map.detach();
  });

  it("swaps into an empty slot by clearing the other action's slot", () => {
    const map = new ActionMap<Action>([key("attack", "KeyZ"), key("confirm", "Enter")]);
    map.clearSlot("confirm", 0);
    const outcome = map.rebind("confirm", 0, key("confirm", "KeyZ"), { policy: "swap" });
    expect(outcome.status).toBe("swapped");
    expect(map.slotsOf("attack")).toEqual([]);
    expect(map.slotsOf("confirm")[0]?.control).toEqual({ type: "key", key: "KeyZ" });
  });

  it("gives the other action the control and keeps its own scale and axis", () => {
    const map = new ActionMap<Action>([
      key("move", "KeyA", { axisComponent: "x", scale: -1 }),
      key("attack", "KeyJ"),
    ]);
    map.rebind("attack", 0, key("attack", "KeyA"));
    expect(map.slotsOf("move")[0]).toMatchObject({
      control: { type: "key", key: "KeyJ" },
      scale: -1,
      axisComponent: "x",
    });
  });

  it("keeps the conflicts of different contexts apart", () => {
    const map = new ActionMap<Action>([
      key("attack", "KeyZ", { context: "gameplay" }),
      key("confirm", "Enter", { context: "menu" }),
    ]);
    expect(map.rebind("confirm", 0, key("confirm", "KeyZ", { context: "menu" }))).toEqual({
      status: "bound",
      conflicts: [],
    });
    expect(map.rebind("confirm", 0, key("confirm", "KeyZ", { context: "gameplay" })).status).toBe("swapped");
  });

  it("rejects a binding for another action", () => {
    const map = new ActionMap<Action>([key("confirm", "Enter")]);
    expect(() => map.rebind("confirm", 0, key("attack", "KeyZ"))).toThrow(RangeError);
  });
});

describe("rebind checks", () => {
  it("rejects a slot that is not a whole number from 0 up", () => {
    const map = new ActionMap<Action>([key("confirm", "Enter")]);
    expect(() => map.rebind("confirm", -1, key("confirm", "KeyZ"))).toThrow(RangeError);
    expect(() => map.rebind("confirm", 1.5, key("confirm", "KeyZ"))).toThrow(RangeError);
    expect(() => map.clearSlot("confirm", -1)).toThrow(RangeError);
  });

  it("rejects a control that is not valid, and a scale that is not finite", () => {
    const map = new ActionMap<Action>([key("confirm", "Enter")]);
    const bogus = { type: "joystick", key: "x" } as unknown as Control;
    expect(() => map.rebind("confirm", 0, { action: "confirm", control: bogus })).toThrow(RangeError);
    expect(() => map.rebind("confirm", 0, key("confirm", "KeyZ", { scale: Number.NaN }))).toThrow(RangeError);
  });

  it("rejects a context name that is empty or too long, and an axis component that is not x or y", () => {
    const map = new ActionMap<Action>([key("confirm", "Enter")]);
    expect(() => map.rebind("confirm", 0, key("confirm", "KeyZ", { context: "" }))).toThrow(RangeError);
    expect(() => map.rebind("confirm", 0, key("confirm", "KeyZ", { context: "x".repeat(65) }))).toThrow(RangeError);
    const bogusAxis = { action: "confirm", control: { type: "key", key: "KeyZ" }, axisComponent: "z" } as unknown as Binding<Action>;
    expect(() => map.rebind("confirm", 0, bogusAxis)).toThrow(RangeError);
  });
});

describe("clearing and resetting", () => {
  it("clears a slot, and releases the action when it was held", () => {
    const target = new EventTarget();
    const map = new ActionMap<Action>([key("confirm", "Enter")]);
    map.attach(target);
    target.dispatchEvent(keyEvent("keydown", "Enter"));
    map.clearSlot("confirm", 0);
    expect(map.released("confirm")).toBe(true);
    expect(map.held("confirm")).toBe(false);
    expect(map.slotsOf("confirm")).toEqual([]);
    map.detach();
  });

  it("does nothing when the slot is already empty", () => {
    const map = new ActionMap<Action>([key("confirm", "Enter")]);
    map.clearSlot("confirm", 0);
    map.clearSlot("confirm", 0);
    expect(map.slotsOf("confirm")).toEqual([]);
  });

  it("restores the defaults of one action, and keeps the others as they were", () => {
    const map = new ActionMap<Action>([
      key("confirm", "Enter"),
      key("confirm", "Space"),
      { action: "attack", control: { type: "mouse", button: "left" } },
    ]);
    map.rebind("confirm", 0, key("confirm", "KeyZ"));
    map.rebind("attack", 0, key("attack", "KeyX"));
    map.resetAction("confirm");
    expect(map.slotsOf("confirm").map((binding) => binding?.control)).toEqual([
      { type: "key", key: "Enter" },
      { type: "key", key: "Space" },
    ]);
    expect(map.slotsOf("attack")[0]?.control).toEqual({ type: "key", key: "KeyX" });
  });

  it("restores the defaults of every action, and drops the bindings that were added later", () => {
    const map = new ActionMap<Action>([key("confirm", "Enter"), { action: "attack", control: { type: "mouse", button: "left" } }]);
    map.rebind("confirm", 0, key("confirm", "KeyZ"));
    map.clearSlot("attack", 0);
    map.bind(key("pause", "KeyP"));
    map.resetAll();
    expect(map.slotsOf("confirm")[0]?.control).toEqual({ type: "key", key: "Enter" });
    expect(map.slotsOf("attack")[0]?.control).toEqual({ type: "mouse", button: "left" });
    expect(map.slotsOf("pause")).toEqual([]);
  });

  it("releases a held action when a reset takes its control away", () => {
    const target = new EventTarget();
    const map = new ActionMap<Action>([key("confirm", "Enter")]);
    map.attach(target);
    map.rebind("confirm", 0, key("confirm", "KeyZ"));
    target.dispatchEvent(keyEvent("keydown", "KeyZ"));
    map.resetAll();
    expect(map.released("confirm")).toBe(true);
    map.detach();
  });
});

describe("conflict groups", () => {
  it("lists the actions that share a control in a context", () => {
    const map = new ActionMap<Action>([key("attack", "KeyZ"), key("confirm", "KeyZ")]);
    expect(map.conflicts()).toEqual([
      {
        context: "default",
        control: "key:KeyZ",
        entries: [
          { action: "attack", slot: 0 },
          { action: "confirm", slot: 0 },
        ],
      },
    ]);
  });

  it("does not count two bindings of one action, or the same control in two contexts", () => {
    const map = new ActionMap<Action>([
      key("confirm", "Enter"),
      key("confirm", "Enter", { slot: 1 }),
      key("attack", "KeyZ", { context: "gameplay" }),
      key("pause", "KeyZ", { context: "menu" }),
    ]);
    expect(map.conflicts()).toEqual([]);
  });

  it("is empty once a conflict is resolved", () => {
    const map = new ActionMap<Action>([key("attack", "KeyZ"), key("confirm", "Enter")]);
    map.rebind("confirm", 0, key("confirm", "KeyZ"), { policy: "allow" });
    expect(map.conflicts()).toHaveLength(1);
    map.rebind("confirm", 0, key("confirm", "KeyX"));
    expect(map.conflicts()).toEqual([]);
  });

  it("keeps the two directions of an axis apart, so an inverted stick is not a conflict", () => {
    const map = new ActionMap<Action>([
      { action: "move", control: { type: "pad-axis", axis: "leftX" }, axisComponent: "x" },
      { action: "attack", control: { type: "pad-axis", axis: "leftX" }, scale: -1 },
    ]);
    expect(map.conflicts()).toEqual([]);
  });
});
