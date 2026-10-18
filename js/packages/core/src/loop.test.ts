// Tests for the fixed-step loop. A fake scheduler stands in for requestAnimationFrame, so
// every frame happens exactly when the test runs it.

import { afterEach, describe, expect, it, vi } from "vitest";
import { GameLoop, createFrameScheduler, type FrameScheduler } from "./loop";

/** A scheduler that runs the frames waiting at the moment `runFrame` is called. */
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

  /** Runs the frames that are waiting now, giving them `timeMs` as their timestamp. */
  runFrame(timeMs: number): void {
    const batch = [...this.callbacks.values()];
    this.callbacks.clear();
    for (const callback of batch) callback(timeMs);
  }
}

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("GameLoop fixed steps", () => {
  it("renders on the first frame without stepping", () => {
    const scheduler = new FakeScheduler();
    const updates: number[] = [];
    const renders: number[] = [];
    const loop = new GameLoop({
      rate: 10,
      scheduler,
      update: (dt) => {
        updates.push(dt);
      },
      render: (alpha) => {
        renders.push(alpha);
      },
    });
    loop.start();
    scheduler.runFrame(5000);
    expect(updates).toEqual([]);
    expect(renders).toEqual([0]);
  });

  it("runs whole steps and reports the leftover time as alpha", () => {
    // At 10 Hz a step is 0.1 s. A frame 150 ms in carries 0.15 s: one step, alpha 0.5.
    const scheduler = new FakeScheduler();
    const updates: number[] = [];
    const renders: number[] = [];
    const loop = new GameLoop({
      rate: 10,
      scheduler,
      update: (dt) => {
        updates.push(dt);
      },
      render: (alpha) => {
        renders.push(alpha);
      },
    });
    loop.start();
    scheduler.runFrame(0);
    scheduler.runFrame(150);
    expect(updates).toHaveLength(1);
    expect(updates[0]).toBeCloseTo(0.1);
    expect(renders.at(-1)).toBeCloseTo(0.5);

    // 0.05 s carried plus 0.05 s more makes one more step, and nothing left over.
    scheduler.runFrame(200);
    expect(updates).toHaveLength(2);
    expect(renders.at(-1)).toBeCloseTo(0);
  });

  it("caps the catch-up at maxSteps and drops the rest of the time", () => {
    const scheduler = new FakeScheduler();
    const updates: number[] = [];
    const loop = new GameLoop({
      rate: 10,
      maxSteps: 3,
      scheduler,
      update: (dt) => {
        updates.push(dt);
      },
    });
    loop.start();
    scheduler.runFrame(0);
    scheduler.runFrame(1000); // one second late: only three steps run, the rest is dropped
    expect(updates).toHaveLength(3);
    scheduler.runFrame(1100);
    expect(updates).toHaveLength(4);
  });

  it("steps 60 times per second by default", () => {
    const scheduler = new FakeScheduler();
    const updates: number[] = [];
    const loop = new GameLoop({
      scheduler,
      update: (dt) => {
        updates.push(dt);
      },
    });
    expect(loop.stepSeconds).toBeCloseTo(1 / 60, 12);
    loop.start();
    scheduler.runFrame(0);
    scheduler.runFrame(1000 / 60);
    expect(updates).toHaveLength(1);
    expect(updates[0]).toBeCloseTo(1 / 60, 12);
  });
});

describe("GameLoop start and stop", () => {
  it("stop cancels the pending frame and ignores late frames", () => {
    const scheduler = new FakeScheduler();
    let calls = 0;
    const loop = new GameLoop({
      rate: 10,
      scheduler,
      update: () => {
        calls += 1;
      },
    });
    loop.start();
    expect(loop.running).toBe(true);
    expect(scheduler.pending).toBe(1);
    loop.stop();
    expect(loop.running).toBe(false);
    expect(scheduler.pending).toBe(0);
    scheduler.runFrame(100);
    expect(calls).toBe(0);
  });

  it("start does nothing while running, and stop does nothing while stopped", () => {
    const scheduler = new FakeScheduler();
    const loop = new GameLoop({ scheduler, update: () => undefined });
    loop.start();
    loop.start();
    expect(scheduler.pending).toBe(1);
    loop.stop();
    loop.stop();
    expect(scheduler.pending).toBe(0);
  });

  it("stops at once when update calls stop, and skips the render", () => {
    const scheduler = new FakeScheduler();
    let updates = 0;
    let renders = 0;
    const loop: GameLoop = new GameLoop({
      rate: 10,
      scheduler,
      update: () => {
        updates += 1;
        if (updates === 2) loop.stop();
      },
      render: () => {
        renders += 1;
      },
    });
    loop.start();
    scheduler.runFrame(0);
    scheduler.runFrame(300); // three steps are due; the loop stops during the second
    expect(updates).toBe(2);
    expect(renders).toBe(1);
    expect(scheduler.pending).toBe(0);
  });

  it("restarting does not catch up on the time spent stopped", () => {
    const scheduler = new FakeScheduler();
    const updates: number[] = [];
    const loop = new GameLoop({
      rate: 10,
      scheduler,
      update: (dt) => {
        updates.push(dt);
      },
    });
    loop.start();
    scheduler.runFrame(0);
    scheduler.runFrame(100);
    loop.stop();
    loop.start();
    scheduler.runFrame(5000); // the first frame after a restart only sets the origin
    expect(updates).toHaveLength(1);
    scheduler.runFrame(5100);
    expect(updates).toHaveLength(2);
  });

  it("rejects a rate that is not positive, and a maxSteps that is not a positive integer", () => {
    const update = (): void => undefined;
    expect(() => new GameLoop({ update, rate: 0 })).toThrow(RangeError);
    expect(() => new GameLoop({ update, rate: -5 })).toThrow(RangeError);
    expect(() => new GameLoop({ update, rate: Number.NaN })).toThrow(RangeError);
    expect(() => new GameLoop({ update, maxSteps: 0 })).toThrow(RangeError);
    expect(() => new GameLoop({ update, maxSteps: 1.5 })).toThrow(RangeError);
  });
});

describe("createFrameScheduler", () => {
  it("uses requestAnimationFrame and cancelAnimationFrame when the host has them", () => {
    const requested: Array<(timeMs: number) => void> = [];
    const cancelled: number[] = [];
    vi.stubGlobal("requestAnimationFrame", (callback: (timeMs: number) => void) => {
      requested.push(callback);
      return requested.length;
    });
    vi.stubGlobal("cancelAnimationFrame", (handle: number) => {
      cancelled.push(handle);
    });
    const scheduler = createFrameScheduler();
    const handle = scheduler.request(() => undefined);
    expect(requested).toHaveLength(1);
    scheduler.cancel(handle);
    expect(cancelled).toEqual([1]);
  });

  it("falls back to setTimeout when requestAnimationFrame is missing", () => {
    vi.useFakeTimers();
    vi.stubGlobal("requestAnimationFrame", undefined);
    const updates: number[] = [];
    const loop = new GameLoop({
      rate: 60,
      update: (dt) => {
        updates.push(dt);
      },
    });
    loop.start();
    vi.advanceTimersByTime(200);
    loop.stop();
    expect(updates.length).toBeGreaterThan(0);
  });
});
