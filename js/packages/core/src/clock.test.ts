// Tests for the game clock: game time, pause, speed multipliers and the day cycle.

import { describe, expect, it } from "vitest";
import { GameClock } from "./clock";

describe("GameClock basics", () => {
  it("starts on day 1, at time zero, at the first speed, running", () => {
    const clock = new GameClock();
    expect(clock.elapsed).toBe(0);
    expect(clock.day).toBe(1);
    expect(clock.timeOfDay).toBe(0);
    expect(clock.speed).toBe(1);
    expect(clock.paused).toBe(false);
    expect(clock.dayLength).toBe(600);
  });

  it("moves game time by the real time it is fed", () => {
    const clock = new GameClock();
    clock.advance(0.25);
    clock.advance(0.75);
    expect(clock.elapsed).toBe(1);
  });

  it("stops while paused and carries on after resume", () => {
    const clock = new GameClock();
    clock.pause();
    clock.pause();
    clock.advance(5);
    expect(clock.paused).toBe(true);
    expect(clock.elapsed).toBe(0);
    clock.resume();
    clock.advance(1);
    expect(clock.paused).toBe(false);
    expect(clock.elapsed).toBe(1);
  });

  it("rejects a negative or non-finite time step", () => {
    const clock = new GameClock();
    expect(() => clock.advance(-1)).toThrow(RangeError);
    expect(() => clock.advance(Number.POSITIVE_INFINITY)).toThrow(RangeError);
    expect(() => clock.advance(Number.NaN)).toThrow(RangeError);
  });
});

describe("GameClock speeds", () => {
  it("scales game time by the speed", () => {
    const clock = new GameClock();
    clock.setSpeed(2);
    clock.advance(1);
    expect(clock.speed).toBe(2);
    expect(clock.elapsed).toBe(2);
  });

  it("offers 1, 2 and 3 by default, and refuses any other speed", () => {
    const clock = new GameClock();
    expect(clock.speeds).toEqual([1, 2, 3]);
    expect(() => clock.setSpeed(5)).toThrow(RangeError);
    expect(clock.speed).toBe(1);
  });

  it("cycles through the speeds and wraps around", () => {
    const clock = new GameClock();
    expect(clock.cycleSpeed()).toBe(2);
    expect(clock.cycleSpeed()).toBe(3);
    expect(clock.cycleSpeed()).toBe(1);
  });

  it("takes custom speeds and sorts them ascending", () => {
    const clock = new GameClock({ speeds: [4, 0.5] });
    expect(clock.speeds).toEqual([0.5, 4]);
    expect(clock.speed).toBe(0.5);
    expect(clock.cycleSpeed()).toBe(4);
  });

  it("rejects an empty list, or a speed that is not a positive finite number", () => {
    expect(() => new GameClock({ speeds: [] })).toThrow(RangeError);
    expect(() => new GameClock({ speeds: [0] })).toThrow(RangeError);
    expect(() => new GameClock({ speeds: [-1, 1] })).toThrow(RangeError);
    expect(() => new GameClock({ speeds: [Number.NaN] })).toThrow(RangeError);
  });
});

describe("GameClock days", () => {
  it("starts a new day when game time crosses the day length", () => {
    const days: number[] = [];
    const clock = new GameClock({
      dayLength: 8,
      onDay: (day) => {
        days.push(day);
      },
    });
    clock.advance(7.5);
    expect(clock.day).toBe(1);
    expect(clock.timeOfDay).toBe(7.5);
    expect(days).toEqual([]);
    clock.advance(0.5);
    expect(clock.day).toBe(2);
    expect(clock.timeOfDay).toBe(0);
    expect(days).toEqual([2]);
  });

  it("calls onDay for every day that one large step skips, in order", () => {
    const days: number[] = [];
    const clock = new GameClock({
      dayLength: 8,
      onDay: (day) => {
        days.push(day);
      },
    });
    clock.advance(28);
    expect(clock.day).toBe(4);
    expect(days).toEqual([2, 3, 4]);
    expect(clock.timeOfDay).toBe(4);
  });

  it("counts days in game time, so a faster speed brings them sooner", () => {
    const clock = new GameClock({ dayLength: 8 });
    clock.setSpeed(3);
    clock.advance(2.5); // 7.5 game seconds
    expect(clock.day).toBe(1);
    clock.advance(0.5); // 9 game seconds
    expect(clock.day).toBe(2);
  });

  it("does not start a day while paused", () => {
    const days: number[] = [];
    const clock = new GameClock({
      dayLength: 8,
      onDay: (day) => {
        days.push(day);
      },
    });
    clock.pause();
    clock.advance(100);
    expect(days).toEqual([]);
    expect(clock.day).toBe(1);
  });

  it("uses 600 game seconds as the default day", () => {
    const clock = new GameClock();
    clock.advance(600);
    expect(clock.day).toBe(2);
    expect(clock.timeOfDay).toBe(0);
  });

  it("rejects a day length that is not a positive finite number", () => {
    expect(() => new GameClock({ dayLength: 0 })).toThrow(RangeError);
    expect(() => new GameClock({ dayLength: Number.POSITIVE_INFINITY })).toThrow(RangeError);
  });
});

describe("GameClock snapshot and restore", () => {
  it("carries time, day, speed and pause through a save without firing onDay", () => {
    const days: number[] = [];
    const before = new GameClock({ dayLength: 10 });
    before.setSpeed(3);
    before.advance(9); // 27 game seconds: day 3
    before.pause();
    const state = before.snapshot();
    expect(state).toEqual({ elapsed: 27, speed: 3, paused: true });

    const after = new GameClock({ dayLength: 10, onDay: (d) => days.push(d) });
    after.restore(JSON.parse(JSON.stringify(state)) as typeof state);
    expect(after.elapsed).toBe(27);
    expect(after.day).toBe(3);
    expect(after.timeOfDay).toBe(7);
    expect(after.speed).toBe(3);
    expect(after.paused).toBe(true);
    expect(days).toEqual([]);

    after.resume();
    after.advance(1); // 30: day 4
    expect(days).toEqual([4]);
  });

  it("refuses a speed the clock does not offer and a negative time", () => {
    const clock = new GameClock();
    expect(() => clock.restore({ elapsed: 5, speed: 7, paused: false })).toThrow(RangeError);
    expect(() => clock.restore({ elapsed: -1, speed: 1, paused: false })).toThrow(RangeError);
  });
});
