// Tests for tweens: the value at each time, the delay, easing, repeat and yoyo, the other kinds of
// value, the controls (pause, resume, seek, progress, reverse, time scale, cancel), the finished
// promise, ending by target or tag, and the game clock's pause and speed.

import { GameClock } from "@velvet/core";
import { describe, expect, it } from "vitest";
import { Ticker } from "./ticker";
import { tween } from "./tween";

describe("a number tween", () => {
  it("writes its start value when it is made", () => {
    const seen: number[] = [];
    tween({ from: 0, to: 10, duration: 1, ticker: new Ticker(), set: (value) => seen.push(value) });
    expect(seen).toEqual([0]);
  });

  it("moves by the time it is advanced by, and ends exactly on its end value", () => {
    const ticker = new Ticker();
    const seen: number[] = [];
    const animation = tween({ from: 0, to: 10, duration: 1, ticker, set: (value) => seen.push(value) });
    ticker.advance(0.25);
    expect(animation.value).toBeCloseTo(2.5, 12);
    ticker.advance(0.75);
    expect(animation.value).toBe(10);
    expect(seen.at(-1)).toBe(10);
  });

  it("calls onComplete once, and resolves finished with complete", async () => {
    const ticker = new Ticker();
    let completions = 0;
    const animation = tween({
      from: 0,
      to: 1,
      duration: 1,
      ticker,
      onComplete: () => {
        completions += 1;
      },
    });
    ticker.advance(1);
    ticker.advance(1);
    expect(completions).toBe(1);
    expect(animation.isDone).toBe(true);
    expect(ticker.size).toBe(0);
    await expect(animation.finished).resolves.toBe("complete");
  });

  it("holds its start value through the delay", () => {
    const ticker = new Ticker();
    const animation = tween({ from: 0, to: 10, duration: 1, delay: 1, ticker });
    ticker.advance(0.5);
    expect(animation.value).toBe(0);
    ticker.advance(1);
    expect(animation.value).toBeCloseTo(5, 12);
  });

  it("eases the progress with a named easing, or with a function", () => {
    const ticker = new Ticker();
    const named = tween({ from: 0, to: 10, duration: 1, ease: "quadIn", ticker });
    const custom = tween({ from: 0, to: 10, duration: 1, ease: (t) => t * t, ticker });
    ticker.advance(0.5);
    expect(named.value).toBeCloseTo(2.5, 12);
    expect(custom.value).toBeCloseTo(2.5, 12);
  });

  it("repeats its cycles one after another", () => {
    const ticker = new Ticker();
    const animation = tween({ from: 0, to: 10, duration: 1, repeat: 2, ticker });
    expect(animation.duration).toBe(3);
    ticker.advance(1.5);
    expect(animation.value).toBeCloseTo(5, 12);
    ticker.advance(1.5);
    expect(animation.value).toBe(10);
    expect(animation.isDone).toBe(true);
  });

  it("plays every other cycle backward with yoyo, and ends back at the start", () => {
    const ticker = new Ticker();
    const animation = tween({ from: 0, to: 10, duration: 1, repeat: 1, yoyo: true, ticker });
    ticker.advance(1.25);
    expect(animation.value).toBeCloseTo(7.5, 12);
    ticker.advance(0.25);
    expect(animation.value).toBeCloseTo(5, 12);
    ticker.advance(0.5);
    expect(animation.value).toBe(0);
    expect(animation.isDone).toBe(true);
  });

  it("repeats forever with an infinite repeat, and never completes", () => {
    const ticker = new Ticker();
    const animation = tween({ from: 0, to: 10, duration: 1, repeat: Number.POSITIVE_INFINITY, ticker });
    ticker.advance(1000);
    expect(animation.isDone).toBe(false);
    expect(animation.duration).toBe(Number.POSITIVE_INFINITY);
  });

  it("jumps to its end value when its duration is zero", () => {
    const seen: number[] = [];
    tween({ from: 0, to: 5, duration: 0, ticker: new Ticker(), set: (value) => seen.push(value) });
    expect(seen).toEqual([5]);
  });

  it("refuses a bad duration, delay, repeat or easing name", () => {
    const ticker = new Ticker();
    expect(() => tween({ from: 0, to: 1, duration: -1, ticker })).toThrow(RangeError);
    expect(() => tween({ from: 0, to: 1, duration: 1, delay: Number.NaN, ticker })).toThrow(RangeError);
    expect(() => tween({ from: 0, to: 1, duration: 1, repeat: 1.5, ticker })).toThrow(RangeError);
    expect(() => tween({ from: 0, to: 1, duration: 1, ease: "wobble", ticker })).toThrow(RangeError);
  });
});

describe("tweens of other kinds", () => {
  it("tweens an object of numbers, and gives each step a new object", () => {
    const ticker = new Ticker();
    const seen: Array<Readonly<Record<string, number>>> = [];
    tween({
      from: { x: 0, y: 10 },
      to: { x: 10, y: 0 },
      duration: 1,
      ticker,
      set: (value) => seen.push(value),
    });
    ticker.advance(0.5);
    expect(seen.at(-1)).toEqual({ x: 5, y: 5 });
    expect(seen[0]).not.toBe(seen[1]);
  });

  it("tweens an array of numbers", () => {
    const ticker = new Ticker();
    const animation = tween({ from: [0, 10], to: [10, 0], duration: 1, ticker });
    ticker.advance(0.5);
    expect(animation.value).toEqual([5, 5]);
  });

  it("tweens a colour through OKLab, and ends on the exact end colour", () => {
    const ticker = new Ticker();
    const animation = tween({ from: "#000000", to: "#ffffff", duration: 1, ticker });
    ticker.advance(0.5);
    const match = /^rgb\((\d+), (\d+), (\d+)\)$/.exec(animation.value);
    expect(match).not.toBeNull();
    const [red, green, blue] = [Number(match?.[1]), Number(match?.[2]), Number(match?.[3])];
    expect(red).toBe(green);
    expect(green).toBe(blue);
    expect(red).toBeGreaterThanOrEqual(98);
    expect(red).toBeLessThanOrEqual(100);
    ticker.advance(0.5);
    expect(animation.value).toBe("#ffffff");
  });

  it("refuses ends of two different kinds", () => {
    // @ts-expect-error a number cannot tween to a colour text
    expect(() => tween({ from: 0, to: "#fff", duration: 1, ticker: new Ticker() })).toThrow(TypeError);
  });
});

describe("tween controls", () => {
  it("stops while paused, and carries on after resume", () => {
    const ticker = new Ticker();
    const animation = tween({ from: 0, to: 10, duration: 1, ticker });
    animation.pause();
    ticker.advance(0.5);
    expect(animation.value).toBe(0);
    expect(animation.paused).toBe(true);
    animation.resume();
    ticker.advance(0.5);
    expect(animation.value).toBeCloseTo(5, 12);
  });

  it("seeks to a time and draws it, and a seek to the end does not complete the tween", () => {
    const ticker = new Ticker();
    let completions = 0;
    const animation = tween({
      from: 0,
      to: 10,
      duration: 1,
      ticker,
      onComplete: () => {
        completions += 1;
      },
    });
    animation.seek(0.75);
    expect(animation.value).toBeCloseTo(7.5, 12);
    animation.seek(1);
    expect(animation.value).toBe(10);
    expect(completions).toBe(0);
    expect(animation.isDone).toBe(false);
    ticker.advance(0.1);
    expect(completions).toBe(1);
  });

  it("reads and sets progress as a fraction of the duration", () => {
    const ticker = new Ticker();
    const animation = tween({ from: 0, to: 10, duration: 2, ticker });
    animation.progress = 0.5;
    expect(animation.value).toBeCloseTo(5, 12);
    expect(animation.time).toBeCloseTo(1, 12);
    expect(animation.progress).toBeCloseTo(0.5, 12);
  });

  it("reverse plays back to the start and completes there", () => {
    const ticker = new Ticker();
    let completions = 0;
    const animation = tween({
      from: 0,
      to: 10,
      duration: 1,
      ticker,
      onComplete: () => {
        completions += 1;
      },
    });
    ticker.advance(0.5);
    animation.reverse();
    ticker.advance(0.25);
    expect(animation.value).toBeCloseTo(2.5, 12);
    ticker.advance(0.5);
    expect(animation.value).toBe(0);
    expect(completions).toBe(1);
  });

  it("time scale changes how fast the playhead moves", () => {
    const ticker = new Ticker();
    const animation = tween({ from: 0, to: 10, duration: 1, timeScale: 2, ticker });
    ticker.advance(0.25);
    expect(animation.time).toBeCloseTo(0.5, 12);
    expect(animation.value).toBeCloseTo(5, 12);
  });

  it("cancel ends it where it stands, and resolves finished with cancelled", async () => {
    const ticker = new Ticker();
    const animation = tween({ from: 0, to: 10, duration: 1, ticker });
    ticker.advance(0.5);
    animation.cancel();
    ticker.advance(0.5);
    expect(animation.value).toBeCloseTo(5, 12);
    expect(ticker.size).toBe(0);
    await expect(animation.finished).resolves.toBe("cancelled");
  });

  it("a finished tween plays again after seek(0) and resume()", () => {
    const ticker = new Ticker();
    let completions = 0;
    const animation = tween({
      from: 0,
      to: 10,
      duration: 1,
      ticker,
      onComplete: () => {
        completions += 1;
      },
    });
    ticker.advance(1);
    animation.seek(0);
    animation.resume();
    ticker.advance(1);
    expect(completions).toBe(2);
    expect(animation.value).toBe(10);
  });

  it("ends by target and by tag", async () => {
    const ticker = new Ticker();
    const panel = {};
    const byTarget = tween({ from: 0, to: 1, duration: 1, target: panel, ticker });
    const byTag = tween({ from: 0, to: 1, duration: 1, tag: "menu", ticker });
    ticker.killTweensOf(panel);
    ticker.killTag("menu");
    await expect(byTarget.finished).resolves.toBe("cancelled");
    await expect(byTag.finished).resolves.toBe("cancelled");
  });
});

describe("tweens on the game clock", () => {
  it("runs at the clock's speed, and stops while the clock is paused", () => {
    const clock = new GameClock();
    clock.setSpeed(3);
    const ticker = new Ticker({ clock });
    const animation = tween({ from: 0, to: 10, duration: 1, ticker });
    ticker.advance(1 / 3);
    expect(animation.value).toBeCloseTo(10, 9);
    expect(animation.isDone).toBe(true);

    clock.pause();
    const paused = tween({ from: 0, to: 10, duration: 1, ticker });
    ticker.advance(1);
    expect(paused.value).toBe(0);
  });
});
