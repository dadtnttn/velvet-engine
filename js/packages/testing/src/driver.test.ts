import { ActionMap, GameClock, GameLoop } from "@velvet/core";
import { afterEach, describe, expect, it } from "vitest";
import { createDriver, type Driver, type DriverOptions } from "./driver";
import { TestingError } from "./errors";
import { InputScript } from "./script";
import { createStageMap } from "./stage";

/** Every driver that a test made, so that each one is disposed when the test ends. */
const drivers: Driver[] = [];

/** A driver that the test disposes afterwards. */
function driverFor(options?: DriverOptions): Driver {
  const driver = createDriver(options);
  drivers.push(driver);
  return driver;
}

afterEach(() => {
  for (const driver of drivers.splice(0)) driver.dispose();
});

/** The error code of the testing error that an action throws, or "none" when it throws nothing else. */
function codeOf(action: () => void): string {
  try {
    action();
  } catch (error) {
    return error instanceof TestingError ? error.code : "other";
  }
  return "none";
}

describe("frames and updates", () => {
  it("runs one update per step, after the loop's time origin", () => {
    const driver = driverFor();
    let updates = 0;
    const loop = driver.createLoop({
      update: () => {
        updates += 1;
      },
    });
    loop.start();
    driver.step(1);
    expect(updates).toBe(1);
    driver.step(59);
    expect(updates).toBe(60);
    expect(driver.frame).toBe(60);
  });

  it("converts seconds into whole frames with advance", () => {
    const driver = driverFor();
    let updates = 0;
    driver.createLoop({ update: () => (updates += 1) }).start();
    expect(driver.advance(1)).toBe(60);
    expect(updates).toBe(60);
    expect(driver.seconds).toBeCloseTo(1, 9);
    expect(() => driver.advance(-1)).toThrow(/0 seconds or more/);
  });

  it("runs the loop's fixed step, so the time passed is the step length for every update", () => {
    const driver = driverFor({ rate: 30 });
    const steps: number[] = [];
    driver.createLoop({ rate: 30, update: (dt) => steps.push(dt) }).start();
    driver.step(10);
    expect(steps).toHaveLength(10);
    for (const step of steps) expect(step).toBeCloseTo(1 / 30, 12);
  });

  it("refuses a loop at another rate, or one that brings its own scheduler", () => {
    const driver = driverFor();
    expect(() => driver.createLoop({ rate: 30, update: () => undefined })).toThrow(/does not match the driver rate/);
    expect(() => driver.createLoop({ scheduler: driver.scheduler, update: () => undefined })).toThrow(/do not pass one/);
  });

  it("refuses a rate that cannot run", () => {
    expect(() => createDriver({ rate: 0 })).toThrow(TestingError);
    expect(() => createDriver({ rate: 5000 })).toThrow(/at most 1000/);
  });

  it("reports a driven loop that was started inside a frame, since its first frame has no update", () => {
    const driver = driverFor();
    const late = driver.createLoop({ update: () => undefined });
    let started = false;
    driver.createLoop({
      update: () => {
        if (started) return;
        started = true;
        late.start();
      },
    }).start();
    expect(() => driver.step(2)).toThrow(/ran 0 updates in frame 1/);
  });

  it("does not count a stopped driven loop, while another loop keeps the frames going", () => {
    const driver = driverFor();
    const first = driver.createLoop({ update: () => undefined });
    first.start();
    let ticks = 0;
    new GameLoop({ scheduler: driver.scheduler, update: () => (ticks += 1) }).start();
    driver.step(2);
    first.stop();
    expect(() => driver.step(2)).not.toThrow();
    expect(ticks).toBeGreaterThanOrEqual(3);
  });

  it("starts a loop that begins while another callback waits, on its first counted frame", () => {
    const driver = driverFor();
    driver.takeOverFrames();
    // A page callback that keeps asking for frames, as a ticker does.
    let tickerFrames = 0;
    const ticker = (): void => {
      tickerFrames += 1;
      requestAnimationFrame(ticker);
    };
    requestAnimationFrame(ticker);
    driver.step(3);
    let updates = 0;
    const loop = driver.createLoop({ update: () => (updates += 1) });
    loop.start();
    driver.step(4);
    expect(updates).toBe(4);
    // The ticker runs once at its origin, three times in the first step, and four times in the second.
    expect(tickerFrames).toBe(8);
  });

  it("refuses to step when no loop is scheduled, and after the loop stops", () => {
    const driver = driverFor();
    expect(() => driver.step(1)).toThrow(/no frame is scheduled/);
    const loop = driver.createLoop({ update: () => undefined });
    loop.start();
    driver.step(2);
    loop.stop();
    expect(() => driver.step(1)).toThrow(TestingError);
  });

  it("primes a loop that is started again, so it does not lose a step", () => {
    const driver = driverFor();
    let updates = 0;
    const loop = driver.createLoop({ update: () => (updates += 1) });
    loop.start();
    driver.step(5);
    loop.stop();
    loop.start();
    driver.step(5);
    expect(updates).toBe(10);
  });
});

describe("until", () => {
  it("runs frames until the predicate is true, and counts them", () => {
    const driver = driverFor();
    let updates = 0;
    driver.createLoop({ update: () => (updates += 1) }).start();
    const frames = driver.until(() => updates >= 7);
    expect(frames).toBe(7);
    expect(driver.until(() => true)).toBe(0);
  });

  it("gives up with a timeout error when the frame budget runs out", () => {
    const driver = driverFor();
    driver.createLoop({ update: () => undefined }).start();
    expect(codeOf(() => driver.until(() => false, { maxFrames: 5 }))).toBe("timeout");
  });
});

describe("scripted input", () => {
  it("presses a key at its frame, and the map counts it on that frame", () => {
    const driver = driverFor();
    const keys = new ActionMap<"jump">([{ action: "jump", control: { type: "key", key: "Space" } }]);
    keys.attach(driver.target);
    const pressedAt: number[] = [];
    driver.createLoop({
      update: () => {
        if (keys.pressed("jump")) pressedAt.push(driver.frame);
        keys.endFrame();
      },
    }).start();
    const script = new InputScript();
    script.at(5).press("Space", { holdFrames: 2 });
    driver.play(script);
    driver.step(10);
    expect(pressedAt).toEqual([5]);
  });

  it("reaches the map with logical pointer positions converted through the stage", () => {
    const stage = createStageMap({ area: () => ({ left: 0, top: 0, width: 960, height: 540 }) });
    const driver = driverFor({ stage });
    const keys = new ActionMap<"fire">([{ action: "fire", control: { type: "mouse", button: "left" } }]);
    keys.attach(driver.target);
    driver.pointerMove(960, 540);
    expect(keys.pointer()).toEqual({ x: 480, y: 270 });
    expect(driver.pointerPosition).toEqual({ x: 960, y: 540 });
    driver.pointerDown("left");
    expect(keys.held("fire")).toBe(true);
    driver.pointerUp("left");
    expect(keys.held("fire")).toBe(false);
  });

  it("does not count a press that the page has already used", () => {
    const driver = driverFor();
    const keys = new ActionMap<"jump">([{ action: "jump", control: { type: "key", key: "Space" } }]);
    keys.attach(driver.target);
    driver.keyDown("Space", { skipped: true });
    expect(keys.held("jump")).toBe(false);
    driver.keyUp("Space");
    driver.keyDown("Space");
    expect(keys.held("jump")).toBe(true);
  });

  it("sends a wheel tick that a wheel binding counts for one frame", () => {
    const driver = driverFor();
    const keys = new ActionMap<"zoom">([{ action: "zoom", control: { type: "wheel", direction: "up" } }]);
    keys.attach(driver.target);
    driver.wheel(-120);
    expect(keys.pressed("zoom")).toBe(true);
    expect(keys.held("zoom")).toBe(true);
    // The pulse is released at the end of the frame that sees it, so the next frame reads it as released.
    keys.endFrame();
    expect(keys.pressed("zoom")).toBe(false);
    expect(keys.released("zoom")).toBe(true);
    expect(keys.held("zoom")).toBe(false);
  });

  it("releases every held key on a blur", () => {
    const driver = driverFor();
    const keys = new ActionMap<"jump">([{ action: "jump", control: { type: "key", key: "Space" } }]);
    keys.attach(driver.target);
    driver.keyDown("Space");
    driver.blur();
    expect(keys.held("jump")).toBe(false);
  });

  it("refuses a key code that is not a key name", () => {
    const driver = driverFor();
    expect(() => driver.keyDown("Space Bar")).toThrow(/letters or digits/);
  });

  it("gives the pads to the loop through the fake navigator", () => {
    const driver = driverFor();
    const keys = new ActionMap<"fire">([{ action: "fire", control: { type: "pad-button", button: "a" } }]);
    const seen: boolean[] = [];
    driver.createLoop({
      update: () => {
        seen.push(keys.held("fire"));
        keys.endFrame();
      },
    }).start();
    driver.pads.connect(0);
    driver.pads.setButton(0, "a", 1);
    driver.step(2);
    // The map reads the pads at the end of each update, so a change shows from the next update on.
    expect(seen).toEqual([false, true]);
  });
});

describe("listeners", () => {
  it("runs beforeTick after the frame's input and afterTick after its update", () => {
    const driver = driverFor();
    const keys = new ActionMap<"jump">([{ action: "jump", control: { type: "key", key: "Space" } }]);
    keys.attach(driver.target);
    const order: string[] = [];
    driver.createLoop({
      update: () => {
        order.push(keys.held("jump") ? "update:pressed" : "update:idle");
        keys.endFrame();
      },
    }).start();
    driver.addListener({
      beforeTick: (frame) => order.push(`before:${frame}`),
      afterTick: (frame) => order.push(`after:${frame}`),
    });
    const script = new InputScript();
    script.at(1).keyDown("Space");
    driver.play(script);
    driver.step(2);
    expect(order).toEqual(["before:0", "update:idle", "after:0", "before:1", "update:pressed", "after:1"]);
  });

  it("reports every input, in the form a replay stores it", () => {
    const driver = driverFor();
    const reported: string[] = [];
    driver.onInput((event) => reported.push(event.type));
    driver.keyDown("KeyW");
    driver.pointerMove(1, 2);
    driver.wheel(3);
    driver.blur();
    expect(reported).toEqual(["key", "pointer", "wheel", "blur"]);
  });

  it("stops calling a listener once it is removed", () => {
    const driver = driverFor();
    let calls = 0;
    const stop = driver.addListener({ afterTick: () => (calls += 1) });
    driver.createLoop({ update: () => undefined }).start();
    driver.step(2);
    stop();
    driver.step(2);
    expect(calls).toBe(2);
  });
});

describe("the game clock", () => {
  it("advances game time by the clock's speed, so a faster clock needs fewer frames", () => {
    const clock = new GameClock();
    const driver = driverFor({ clock });
    driver.createLoop({ update: (dt) => clock.advance(dt) }).start();
    clock.setSpeed(2);
    expect(driver.advanceGame(1)).toBe(30);
    expect(clock.elapsed).toBeGreaterThanOrEqual(1 - 1e-9);
  });

  it("does not move a paused clock, and says so when the frame budget runs out", () => {
    const clock = new GameClock();
    const driver = driverFor({ clock });
    driver.createLoop({ update: (dt) => clock.advance(dt) }).start();
    clock.pause();
    expect(codeOf(() => driver.advanceGame(1, { maxFrames: 10 }))).toBe("clock-stalled");
  });

  it("needs a clock for advanceGame", () => {
    const driver = driverFor();
    expect(() => driver.advanceGame(1)).toThrow(/needs a clock/);
  });
});

describe("page globals", () => {
  it("points requestAnimationFrame at the driver, and puts the original back when restored", () => {
    const hadFrames = "requestAnimationFrame" in globalThis;
    const driver = driverFor();
    const restore = driver.takeOverFrames();
    let updates = 0;
    // A loop made with the default scheduler asks the page for frames, so the driver runs it.
    const loop = new GameLoop({ update: () => (updates += 1) });
    loop.start();
    driver.step(4);
    expect(updates).toBe(4);
    loop.stop();
    restore();
    expect("requestAnimationFrame" in globalThis).toBe(hadFrames);
  });

  it("removes the fake gamepads and the page takeover on dispose, and then refuses to run", () => {
    const driver = driverFor();
    driver.takeOverFrames();
    driver.dispose();
    expect(Reflect.has(navigator, "getGamepads")).toBe(false);
    expect(codeOf(() => driver.step(1))).toBe("disposed");
    driver.dispose();
  });
});
