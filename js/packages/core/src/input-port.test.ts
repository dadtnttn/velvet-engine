// Tests for the rebinding port: the slots that a UI lists, a capture into a slot with the outcome, the
// axis and context of a slot that a capture keeps, and the pass-through of clear and reset. The type
// import below checks at compile time that the port has the shape of @velvet/ui's RebindPort.

import { describe, expect, it, vi } from "vitest";
import type { RebindPort as PanelPort } from "../../ui/src/rebind-panel";
import { ActionMap } from "./input";
import { createRebindPort } from "./input-port";
import { FakePads, fakePad, manualScheduler } from "./input-test-support";

type Action = "confirm" | "move" | "attack";

/** The compile-time check: the core port must be a valid UI port for the same actions. */
const asPanelPort = (map: ActionMap<Action>): PanelPort<Action> => createRebindPort(map);

const pads = new FakePads();

/** A key event as the browser sends it. */
function keyEvent(code: string): Event {
  return Object.assign(new Event("keydown"), { code, repeat: false });
}

describe("slots", () => {
  it("lists the slots of an action with the display of each one, and null for an empty one", () => {
    const map = new ActionMap<Action>([
      { action: "confirm", control: { type: "key", key: "Enter" } },
      { action: "confirm", control: { type: "pad-button", button: "a" }, slot: 2 },
    ]);
    const port = asPanelPort(map);
    expect(port.slots("confirm")).toEqual([
      { slot: 0, display: { device: "keyboard", code: "Enter", labelKey: "input.key.Enter" } },
      { slot: 1, display: null },
      { slot: 2, display: { device: "gamepad", code: "a", labelKey: "input.pad.a" } },
    ]);
    expect(port.slots("attack")).toEqual([]);
  });
});

describe("capture", () => {
  it("binds the next key into the slot and reports the outcome with the display", async () => {
    const target = new EventTarget();
    const map = new ActionMap<Action>([{ action: "confirm", control: { type: "key", key: "Enter" } }]);
    const port = createRebindPort(map, { target, scheduler: manualScheduler() });
    const controller = new AbortController();
    const pending = port.capture("confirm", 0, {
      devices: ["keyboard"],
      timeoutMs: 5000,
      policy: "swap",
      signal: controller.signal,
    });
    target.dispatchEvent(keyEvent("KeyQ"));
    await expect(pending).resolves.toEqual({
      status: "bound",
      conflicts: [],
      display: { device: "keyboard", code: "KeyQ", labelKey: "input.key.KeyQ" },
    });
    expect(map.slotsOf("confirm")[0]?.control).toEqual({ type: "key", key: "KeyQ" });
  });

  it("applies the policy of the request, and reports the slot that was in conflict", async () => {
    const target = new EventTarget();
    const map = new ActionMap<Action>([
      { action: "attack", control: { type: "key", key: "KeyZ" } },
      { action: "confirm", control: { type: "key", key: "Enter" } },
    ]);
    const port = createRebindPort(map, { target, scheduler: manualScheduler() });
    const controller = new AbortController();
    const pending = port.capture("confirm", 0, {
      devices: ["keyboard"],
      timeoutMs: 5000,
      policy: "refuse",
      signal: controller.signal,
    });
    target.dispatchEvent(keyEvent("KeyZ"));
    await expect(pending).resolves.toMatchObject({ status: "refused", conflicts: [{ action: "attack", slot: 0 }] });
    expect(map.slotsOf("confirm")[0]?.control).toEqual({ type: "key", key: "Enter" });
  });

  it("keeps the axis component and the context of the slot, and takes the push of a stick as the scale", async () => {
    pads.set([fakePad(0)]);
    pads.install();
    try {
      const scheduler = manualScheduler();
      const map = new ActionMap<Action>([
        { action: "move", control: { type: "key", key: "KeyW" }, axisComponent: "y", context: "gameplay" },
      ]);
      const port = createRebindPort(map, { scheduler, target: null });
      const controller = new AbortController();
      const pending = port.capture("move", 0, {
        devices: ["gamepad"],
        timeoutMs: 5000,
        policy: "swap",
        signal: controller.signal,
      });
      pads.set([fakePad(0, { axes: [0, -0.9, 0, 0] })]);
      scheduler.run();
      await expect(pending).resolves.toMatchObject({
        status: "bound",
        display: { device: "gamepad", code: "leftY", labelKey: "input.pad.axis.leftY", negative: true },
      });
      expect(map.slotsOf("move")[0]).toMatchObject({
        control: { type: "pad-axis", axis: "leftY" },
        scale: -1,
        axisComponent: "y",
        context: "gameplay",
      });
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it("resolves with null when the capture is cancelled through its signal", async () => {
    const target = new EventTarget();
    const map = new ActionMap<Action>([{ action: "confirm", control: { type: "key", key: "Enter" } }]);
    const port = createRebindPort(map, { target, scheduler: manualScheduler() });
    const controller = new AbortController();
    const pending = port.capture("confirm", 0, {
      devices: ["keyboard"],
      timeoutMs: 5000,
      policy: "swap",
      signal: controller.signal,
    });
    controller.abort();
    await expect(pending).resolves.toBeNull();
    expect(map.slotsOf("confirm")[0]?.control).toEqual({ type: "key", key: "Enter" });
  });
});

describe("clear and reset", () => {
  it("passes clear, resetAction and resetAll through to the map", () => {
    const map = new ActionMap<Action>([
      { action: "confirm", control: { type: "key", key: "Enter" } },
      { action: "attack", control: { type: "mouse", button: "left" } },
    ]);
    const port = createRebindPort(map);
    port.clear("confirm", 0);
    expect(map.slotsOf("confirm")).toEqual([]);

    map.rebind("attack", 0, { action: "attack", control: { type: "key", key: "KeyX" } });
    port.resetAction("attack");
    expect(map.slotsOf("attack")[0]?.control).toEqual({ type: "mouse", button: "left" });

    port.clear("attack", 0);
    port.resetAll();
    expect(map.slotsOf("confirm")[0]?.control).toEqual({ type: "key", key: "Enter" });
    expect(map.slotsOf("attack")[0]?.control).toEqual({ type: "mouse", button: "left" });
  });
});
