// Tests for springs: a settled spring sits on its goal exactly, a critically damped spring does not
// overshoot and an underdamped one does, a stiff spring and a long step stay stable, a settled spring
// wakes when its goal moves, the callbacks run, and the spring follows the game clock.

import { GameClock } from "@velvet/core";
import { describe, expect, it } from "vitest";
import { criticalDamping, spring } from "./spring";
import { Ticker } from "./ticker";

/** Runs a ticker for `seconds` in frames of `step` seconds. */
function run(ticker: Ticker, seconds: number, step = 1 / 60): void {
  const frames = Math.round(seconds / step);
  for (let frame = 0; frame < frames; frame += 1) ticker.advance(step);
}

describe("spring damping", () => {
  it("critical damping is 2 times the square root of stiffness times mass", () => {
    expect(criticalDamping(170)).toBeCloseTo(2 * Math.sqrt(170), 12);
    expect(criticalDamping(16, 4)).toBe(16);
  });
});

describe("spring motion", () => {
  it("settles on its goal exactly, and completes", async () => {
    const ticker = new Ticker();
    const follower = spring({ ticker, to: 100 });
    run(ticker, 3);
    expect(follower.settled).toBe(true);
    expect(follower.value).toBe(100);
    expect(follower.velocity).toBe(0);
    expect(follower.isDone).toBe(true);
    expect(ticker.size).toBe(0);
    await expect(follower.finished).resolves.toBe("complete");
  });

  it("a critically damped spring reaches its goal without overshooting", () => {
    const ticker = new Ticker();
    const values: number[] = [];
    spring({ ticker, to: 100, onUpdate: (value) => values.push(value) });
    run(ticker, 3);
    expect(Math.max(...values)).toBeLessThanOrEqual(100 + 1e-6);
    expect(values.at(-1)).toBe(100);
  });

  it("an underdamped spring overshoots its goal", () => {
    const ticker = new Ticker();
    const values: number[] = [];
    spring({ ticker, to: 100, damping: 5, onUpdate: (value) => values.push(value) });
    run(ticker, 5);
    expect(Math.max(...values)).toBeGreaterThan(110);
  });

  it("stays stable with a stiff spring and one long step", () => {
    const ticker = new Ticker();
    const stiff = spring({ ticker, to: 1, stiffness: 100_000 });
    ticker.advance(0.5);
    expect(Number.isFinite(stiff.value)).toBe(true);
    expect(stiff.value).toBeCloseTo(1, 6);
    expect(stiff.settled).toBe(true);
  });

  it("a long step and short steps both settle on the goal", () => {
    const longTicker = new Ticker();
    const shortTicker = new Ticker();
    const long = spring({ ticker: longTicker, to: 100 });
    const short = spring({ ticker: shortTicker, to: 100 });
    longTicker.advance(3);
    run(shortTicker, 3);
    expect(long.value).toBe(100);
    expect(short.value).toBe(100);
  });

  it("wakes a settled spring when its goal moves", () => {
    const ticker = new Ticker();
    const follower = spring({ ticker, to: 10 });
    run(ticker, 3);
    expect(follower.settled).toBe(true);
    follower.to = 0;
    expect(follower.settled).toBe(false);
    run(ticker, 3);
    expect(follower.value).toBe(0);
    expect(follower.settled).toBe(true);
  });

  it("moves with the game clock, and stops while the clock is paused", () => {
    const clock = new GameClock();
    const ticker = new Ticker({ clock });
    const follower = spring({ ticker, to: 100 });
    clock.pause();
    run(ticker, 1);
    expect(follower.value).toBe(0);
    clock.resume();
    run(ticker, 3);
    expect(follower.value).toBe(100);
  });
});

describe("spring callbacks and controls", () => {
  it("runs onUpdate as the value changes, and onComplete once when it settles", () => {
    const ticker = new Ticker();
    let updates = 0;
    let settles = 0;
    spring({
      ticker,
      to: 5,
      onUpdate: () => {
        updates += 1;
      },
      onComplete: () => {
        settles += 1;
      },
    });
    run(ticker, 3);
    expect(updates).toBeGreaterThan(10);
    expect(settles).toBe(1);
  });

  it("snap moves the value and the goal at rest, with no motion", () => {
    const ticker = new Ticker();
    const follower = spring({ ticker, to: 50 });
    follower.snap(20);
    expect(follower.value).toBe(20);
    expect(follower.to).toBe(20);
    expect(follower.velocity).toBe(0);
    run(ticker, 1);
    expect(follower.value).toBe(20);
  });

  it("cancel ends it where it stands, and resolves finished with cancelled", async () => {
    const ticker = new Ticker();
    const follower = spring({ ticker, to: 100 });
    ticker.advance(0.05);
    const where = follower.value;
    follower.cancel();
    run(ticker, 1);
    expect(follower.value).toBe(where);
    await expect(follower.finished).resolves.toBe("cancelled");
  });

  it("refuses a stiffness, mass or rest threshold that is not above zero, a negative damping, and a value that is not finite", () => {
    const ticker = new Ticker();
    expect(() => spring({ ticker, stiffness: 0 })).toThrow(RangeError);
    expect(() => spring({ ticker, mass: -1 })).toThrow(RangeError);
    expect(() => spring({ ticker, restDelta: 0 })).toThrow(RangeError);
    expect(() => spring({ ticker, damping: -1 })).toThrow(RangeError);
    expect(() => spring({ ticker, to: Number.NaN })).toThrow(RangeError);
  });
});
