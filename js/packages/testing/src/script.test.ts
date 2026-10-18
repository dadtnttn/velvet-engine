import { ActionMap } from "@velvet/core";
import { afterEach, describe, expect, it } from "vitest";
import { createDriver, type Driver } from "./driver";
import { TestingError } from "./errors";
import { InputScript } from "./script";

const drivers: Driver[] = [];

/** A driver with a loop that records, for each frame, what the test asks about. */
function traced<Seen>(read: (driver: Driver) => Seen): { driver: Driver; seen: Array<{ frame: number; value: Seen }> } {
  const driver = createDriver();
  drivers.push(driver);
  const seen: Array<{ frame: number; value: Seen }> = [];
  driver
    .createLoop({
      update: () => {
        seen.push({ frame: driver.frame, value: read(driver) });
      },
    })
    .start();
  return { driver, seen };
}

afterEach(() => {
  for (const driver of drivers.splice(0)) driver.dispose();
});

describe("the script builder", () => {
  it("refuses a frame that is not a whole number from 0 up", () => {
    const script = new InputScript();
    expect(() => script.at(-1)).toThrow(TestingError);
    expect(() => script.at(1.5)).toThrow(/whole number/);
  });

  it("counts its size, and its last frame", () => {
    const script = new InputScript();
    script.at(4).tap("KeyA");
    script.at(9).wheel(120);
    expect(script.size).toBe(3);
    expect(script.endFrame).toBe(9);
    expect(script.actions().map((action) => action.frame)).toEqual([4, 5, 9]);
  });
});

describe("keys", () => {
  it("holds a key for the given frames, from its frame", () => {
    const keys = new ActionMap<"jump">([{ action: "jump", control: { type: "key", key: "Space" } }]);
    const { driver, seen } = traced(() => keys.held("jump"));
    keys.attach(driver.target);
    const script = new InputScript();
    script.at(2).press("Space", { holdFrames: 3 });
    driver.play(script);
    driver.step(7);
    expect(seen.filter((entry) => entry.value).map((entry) => entry.frame)).toEqual([2, 3, 4]);
  });

  it("taps a key for one frame", () => {
    const keys = new ActionMap<"jump">([{ action: "jump", control: { type: "key", key: "Space" } }]);
    const { driver, seen } = traced(() => keys.held("jump"));
    keys.attach(driver.target);
    const script = new InputScript();
    script.at(1).tap("Space");
    driver.play(script);
    driver.step(4);
    expect(seen.filter((entry) => entry.value).map((entry) => entry.frame)).toEqual([1]);
  });

  it("refuses a press that holds for no frames", () => {
    const script = new InputScript();
    expect(() => script.at(0).press("Space", { holdFrames: 0 })).toThrow(/from 1 up/);
  });
});

describe("the pointer", () => {
  it("clicks at a logical position: the pointer moves there, and the button is held for two frames", () => {
    const keys = new ActionMap<"fire">([{ action: "fire", control: { type: "mouse", button: "left" } }]);
    const { driver, seen } = traced(() => ({ held: keys.held("fire"), at: keys.pointer() }));
    keys.attach(driver.target);
    const script = new InputScript();
    script.at(3).click({ x: 100, y: 50 });
    driver.play(script);
    driver.step(7);
    const held = seen.filter((entry) => entry.value.held).map((entry) => entry.frame);
    expect(held).toEqual([3, 4]);
    expect(seen[3]?.value.at).toEqual({ x: 100, y: 50 });
  });

  it("drags in even steps from one position to another, and lets go at the end", () => {
    const { driver, seen } = traced((current) => ({ at: current.pointerPosition }));
    const keys = new ActionMap<"fire">([{ action: "fire", control: { type: "mouse", button: "left" } }]);
    keys.attach(driver.target);
    const script = new InputScript();
    script.at(0).drag({ x: 0, y: 0 }, { x: 400, y: 200 }, 4);
    driver.play(script);
    // A drag takes frames + 3 frames: move, press, four moves, release.
    driver.step(7);
    expect(seen[1]?.value.at).toEqual({ x: 0, y: 0 });
    expect(seen[2]?.value.at).toEqual({ x: 100, y: 50 });
    expect(seen[5]?.value.at).toEqual({ x: 400, y: 200 });
    expect(driver.pointerPosition).toEqual({ x: 400, y: 200 });
  });

  it("holds the button through a drag", () => {
    const keys = new ActionMap<"fire">([{ action: "fire", control: { type: "mouse", button: "left" } }]);
    const { driver, seen } = traced(() => keys.held("fire"));
    keys.attach(driver.target);
    const script = new InputScript();
    script.at(0).drag({ x: 0, y: 0 }, { x: 10, y: 10 }, 3);
    driver.play(script);
    driver.step(6);
    // The button goes down on frame 1, after the first move, and comes up on frame 5, after the last move.
    const heldFrames = seen.filter((entry) => entry.value).map((entry) => entry.frame);
    expect(heldFrames).toEqual([1, 2, 3, 4]);
  });

  it("refuses a drag with no steps", () => {
    expect(() => new InputScript().at(0).drag({ x: 0, y: 0 }, { x: 1, y: 1 }, 0)).toThrow(/from 1 up/);
  });
});

describe("the pads", () => {
  it("presses a pad button for its frames, at its value", () => {
    const { driver, seen } = traced((current) => current.pads.state(0)?.buttons[0]?.value ?? -1);
    driver.pads.connect(0);
    const script = new InputScript();
    script.at(1).padButton("a", { value: 0.5, holdFrames: 2 });
    driver.play(script);
    driver.step(5);
    expect(seen.map((entry) => entry.value)).toEqual([0, 0.5, 0.5, 0, 0]);
  });

  it("ramps an axis in even steps over its frames, and reaches the value on the last", () => {
    const { driver, seen } = traced((current) => current.pads.axis(0, "leftX"));
    driver.pads.connect(0);
    const script = new InputScript();
    script.at(1).padAxis("leftX", 1, { frames: 4 });
    driver.play(script);
    driver.step(6);
    expect(seen.map((entry) => entry.value)).toEqual([0, 0, 0.25, 0.5, 0.75, 1]);
  });

  it("pushes a stick and returns it to rest after its hold, or keeps it pushed without one", () => {
    const { driver, seen } = traced((current) => current.pads.axis(0, "leftY"));
    driver.pads.connect(0);
    const script = new InputScript();
    script.at(1).padStick("left", 0, -0.5, { holdFrames: 2 });
    script.at(6).padStick("left", 0, 0.5);
    driver.play(script);
    driver.step(8);
    expect(seen.map((entry) => entry.value)).toEqual([0, -0.5, -0.5, 0, 0, 0, 0.5, 0.5]);
  });

  it("connects and disconnects a pad at its frames", () => {
    const { driver, seen } = traced((current) => current.pads.state(2) !== null);
    const script = new InputScript();
    script.at(1).padConnect({ pad: 2, id: "Late pad" });
    script.at(3).padDisconnect(2);
    driver.play(script);
    driver.step(4);
    expect(seen.map((entry) => entry.value)).toEqual([false, true, true, false]);
  });
});

describe("playing a script", () => {
  it("counts its frames from the frame where it starts, and says when it is done", () => {
    const { driver, seen } = traced(() => undefined);
    driver.step(5);
    const script = new InputScript();
    script.at(2).run(() => undefined);
    script.at(3).run(() => undefined);
    // Played at frame 5, so the actions are due at frames 7 and 8.
    const run = driver.play(script);
    expect(run.done).toBe(false);
    driver.step(2);
    expect(run.done).toBe(false);
    driver.step(1);
    expect(run.done).toBe(false);
    driver.step(1);
    expect(run.done).toBe(true);
    expect(seen).toHaveLength(9);
  });

  it("drops the actions that have not run when it is cancelled", () => {
    const { driver } = traced(() => undefined);
    const ran: number[] = [];
    const script = new InputScript();
    script.at(0).run(() => ran.push(0));
    script.at(5).run(() => ran.push(5));
    const run = driver.play(script);
    driver.step(2);
    run.cancel();
    expect(run.done).toBe(true);
    driver.step(6);
    expect(ran).toEqual([0]);
  });

  it("finishes at once for an empty script", () => {
    const { driver } = traced(() => undefined);
    expect(driver.play(new InputScript()).done).toBe(true);
  });
});
