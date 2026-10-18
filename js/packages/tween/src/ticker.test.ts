// Tests for the ticker: it moves its playables by the time it is given, it follows the game clock's
// pause and speed, it runs the frame-end callbacks, it ends playables by target or tag, it reports
// when it becomes busy, and the real-time driver runs with the playables. A fake scheduler stands in
// for animation frames, and the core loop drives a ticker in one test.

import { GameClock, GameLoop, type FrameScheduler } from "@velvet/core";
import { afterEach, describe, expect, it } from "vitest";
import { createRealtimeDriver, sharedTicker, Ticker, type Playable } from "./ticker";

/** A scheduler whose frames run only when the test asks, at the time the test gives. */
class FakeScheduler implements FrameScheduler {
  private nextHandle = 1;
  private readonly callbacks = new Map<number, (timeMs: number) => void>();

  request(callback: (timeMs: number) => void): number {
    const handle = this.nextHandle;
    this.nextHandle += 1;
    this.callbacks.set(handle, callback);
    return handle;
  }

  cancel(handle: number): void {
    this.callbacks.delete(handle);
  }

  /** The number of frames waiting to run. */
  get pending(): number {
    return this.callbacks.size;
  }

  /** Runs the frames that wait now, with `timeMs` as their time. */
  runFrame(timeMs: number): void {
    const batch = [...this.callbacks.values()];
    this.callbacks.clear();
    for (const callback of batch) callback(timeMs);
  }
}

/** A playable that records every step it is given. It ends after a number of steps, if asked to. */
class Recorder implements Playable {
  readonly steps: number[] = [];
  readonly target: unknown;
  readonly tag: string | undefined;
  killed = false;
  private readonly endAfterSteps: number;

  constructor(options: { target?: unknown; tag?: string; endAfterSteps?: number } = {}) {
    this.target = options.target;
    this.tag = options.tag;
    this.endAfterSteps = options.endAfterSteps ?? Number.POSITIVE_INFINITY;
  }

  get isDone(): boolean {
    return this.killed || this.steps.length >= this.endAfterSteps;
  }

  advance(dt: number): void {
    this.steps.push(dt);
  }

  kill(): void {
    this.killed = true;
  }
}

afterEach(() => {
  sharedTicker().clear();
});

describe("Ticker moves its playables", () => {
  it("gives every playable the step it is advanced by, and counts the time", () => {
    const ticker = new Ticker();
    const first = new Recorder();
    const second = new Recorder();
    ticker.add(first);
    ticker.add(second);
    ticker.advance(0.5);
    expect(first.steps).toEqual([0.5]);
    expect(second.steps).toEqual([0.5]);
    expect(ticker.time).toBe(0.5);
  });

  it("takes a playable off once it is done", () => {
    const ticker = new Ticker();
    ticker.add(new Recorder({ endAfterSteps: 1 }));
    expect(ticker.size).toBe(1);
    ticker.advance(0.1);
    expect(ticker.size).toBe(0);
  });

  it("runs a playable once, even when it is added twice", () => {
    const ticker = new Ticker();
    const recorder = new Recorder();
    ticker.add(recorder);
    ticker.add(recorder);
    ticker.advance(0.1);
    expect(recorder.steps).toEqual([0.1]);
  });

  it("does not run a playable that another one adds during the same step", () => {
    const ticker = new Ticker();
    const late = new Recorder();
    const adder: Playable = {
      target: undefined,
      tag: undefined,
      isDone: false,
      advance: () => {
        ticker.add(late);
      },
      kill: () => undefined,
    };
    ticker.add(adder);
    ticker.advance(0.1);
    expect(late.steps).toEqual([]);
    ticker.advance(0.1);
    expect(late.steps).toEqual([0.1]);
  });

  it("refuses a negative or infinite step", () => {
    const ticker = new Ticker();
    expect(() => ticker.advance(-1)).toThrow(RangeError);
    expect(() => ticker.advance(Number.POSITIVE_INFINITY)).toThrow(RangeError);
  });
});

describe("Ticker and the game clock", () => {
  it("stops playables while the clock is paused, and runs them again after resume", () => {
    const clock = new GameClock();
    const ticker = new Ticker({ clock });
    const recorder = new Recorder();
    ticker.add(recorder);
    clock.pause();
    ticker.advance(1);
    expect(recorder.steps).toEqual([]);
    expect(ticker.time).toBe(0);
    clock.resume();
    ticker.advance(1);
    expect(recorder.steps).toEqual([1]);
  });

  it("runs playables at the clock's speed", () => {
    const clock = new GameClock();
    clock.setSpeed(2);
    const ticker = new Ticker({ clock });
    const recorder = new Recorder();
    ticker.add(recorder);
    ticker.advance(0.5);
    expect(recorder.steps).toEqual([1]);
    expect(ticker.time).toBe(1);
  });

  it("still runs the frame-end callbacks while the clock is paused", () => {
    const clock = new GameClock();
    const ticker = new Ticker({ clock });
    let flushes = 0;
    ticker.onFrameEnd(() => {
      flushes += 1;
    });
    clock.pause();
    ticker.advance(1);
    expect(flushes).toBe(1);
  });

  it("keeps an unscaled ticker moving while the game is paused", () => {
    const clock = new GameClock();
    const motion = new Ticker({ clock });
    const ui = new Ticker();
    const menu = new Recorder();
    ui.add(menu);
    clock.pause();
    motion.advance(0.25);
    ui.advance(0.25);
    expect(menu.steps).toEqual([0.25]);
  });

  it("follows the core loop: the loop's steps move the clock and the ticker together", () => {
    const scheduler = new FakeScheduler();
    const clock = new GameClock();
    const motion = new Ticker({ clock });
    const recorder = new Recorder();
    motion.add(recorder);
    const loop = new GameLoop({
      rate: 10,
      scheduler,
      update: (dt) => {
        clock.advance(dt);
        motion.advance(dt);
      },
    });
    loop.start();
    scheduler.runFrame(0);
    scheduler.runFrame(100);
    scheduler.runFrame(200);
    clock.pause();
    scheduler.runFrame(300);
    expect(recorder.steps).toEqual([0.1, 0.1]);
    expect(clock.elapsed).toBeCloseTo(0.2, 12);
    loop.stop();
  });
});

describe("frame-end callbacks", () => {
  it("run after the playables move, once per advance", () => {
    const ticker = new Ticker();
    const order: string[] = [];
    ticker.add({
      target: undefined,
      tag: undefined,
      isDone: false,
      advance: () => {
        order.push("move");
      },
      kill: () => undefined,
    });
    ticker.onFrameEnd(() => {
      order.push("write");
    });
    ticker.advance(0.1);
    expect(order).toEqual(["move", "write"]);
  });

  it("stops calling a callback once the returned function is called", () => {
    const ticker = new Ticker();
    let calls = 0;
    const stop = ticker.onFrameEnd(() => {
      calls += 1;
    });
    ticker.advance(0.1);
    stop();
    ticker.advance(0.1);
    expect(calls).toBe(1);
  });
});

describe("ending playables by target or tag", () => {
  it("ends the playables that have the target", () => {
    const ticker = new Ticker();
    const panel = {};
    const onPanel = new Recorder({ target: panel });
    const elsewhere = new Recorder({ target: {} });
    ticker.add(onPanel);
    ticker.add(elsewhere);
    ticker.killTweensOf(panel);
    expect(onPanel.killed).toBe(true);
    expect(elsewhere.killed).toBe(false);
  });

  it("ends the playables that have the tag", () => {
    const ticker = new Ticker();
    const tagged = new Recorder({ tag: "menu" });
    const other = new Recorder({ tag: "world" });
    ticker.add(tagged);
    ticker.add(other);
    ticker.killTag("menu");
    expect(tagged.killed).toBe(true);
    expect(other.killed).toBe(false);
  });

  it("clear ends every playable", () => {
    const ticker = new Ticker();
    const first = new Recorder();
    const second = new Recorder();
    ticker.add(first);
    ticker.add(second);
    ticker.clear();
    expect(first.killed && second.killed).toBe(true);
  });
});

describe("onActive", () => {
  it("runs when a playable joins an idle ticker, and not for later joins", () => {
    let activations = 0;
    const ticker = new Ticker({
      onActive: () => {
        activations += 1;
      },
    });
    ticker.add(new Recorder({ endAfterSteps: 1 }));
    ticker.add(new Recorder());
    expect(activations).toBe(1);
    ticker.advance(0.1);
    expect(ticker.size).toBe(1);
  });

  it("runs again after the ticker became idle", () => {
    let activations = 0;
    const ticker = new Ticker({
      onActive: () => {
        activations += 1;
      },
    });
    ticker.add(new Recorder({ endAfterSteps: 1 }));
    ticker.advance(0.1);
    ticker.add(new Recorder({ endAfterSteps: 1 }));
    expect(activations).toBe(2);
  });
});

describe("the real-time driver", () => {
  it("advances the ticker by the time between frames", () => {
    const scheduler = new FakeScheduler();
    const ticker = new Ticker();
    const recorder = new Recorder();
    ticker.add(recorder);
    const driver = createRealtimeDriver(ticker, { scheduler });
    driver.start();
    expect(scheduler.pending).toBe(1);
    // The first frame only sets the time origin, so it moves nothing.
    scheduler.runFrame(0);
    expect(recorder.steps).toEqual([]);
    scheduler.runFrame(100);
    expect(recorder.steps).toEqual([0.1]);
  });

  it("cuts a long gap to its largest step", () => {
    const scheduler = new FakeScheduler();
    const ticker = new Ticker();
    const recorder = new Recorder();
    ticker.add(recorder);
    const driver = createRealtimeDriver(ticker, { scheduler, maxStep: 0.05 });
    driver.start();
    scheduler.runFrame(0);
    scheduler.runFrame(5000);
    expect(recorder.steps).toEqual([0.05]);
  });

  it("stops by itself when the ticker has nothing left to run", () => {
    const scheduler = new FakeScheduler();
    const ticker = new Ticker();
    ticker.add(new Recorder({ endAfterSteps: 2 }));
    const driver = createRealtimeDriver(ticker, { scheduler });
    driver.start();
    scheduler.runFrame(0);
    scheduler.runFrame(100);
    expect(ticker.size).toBe(1);
    expect(scheduler.pending).toBe(1);
    scheduler.runFrame(200);
    expect(ticker.size).toBe(0);
    expect(scheduler.pending).toBe(0);
    expect(driver.running).toBe(false);
  });

  it("stop cancels the frame that waits, and start does nothing while frames run", () => {
    const scheduler = new FakeScheduler();
    const ticker = new Ticker();
    ticker.add(new Recorder());
    const driver = createRealtimeDriver(ticker, { scheduler });
    driver.start();
    driver.start();
    expect(scheduler.pending).toBe(1);
    driver.stop();
    expect(scheduler.pending).toBe(0);
    expect(driver.running).toBe(false);
  });

  it("refuses a step that is not positive", () => {
    expect(() => createRealtimeDriver(new Ticker(), { maxStep: 0 })).toThrow(RangeError);
  });
});

describe("sharedTicker", () => {
  it("is the same ticker every time", () => {
    expect(sharedTicker()).toBe(sharedTicker());
  });
});
