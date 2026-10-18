// Tests for the clock's events and its state snapshot: paused, resumed, speed and stepped, which are sent only
// when something changes, the copy that state() gives, and the events that a restore sends.

import { describe, expect, it } from "vitest";
import { GameClock, type GameClockStatus } from "./clock";

describe("GameClock pause events", () => {
  it("sends paused once per pause and resumed once per resume, with the state after the change", () => {
    const clock = new GameClock({ dayLength: 100 });
    const events: string[] = [];
    const paused: GameClockStatus[] = [];
    clock.on("paused", (status) => {
      events.push("paused");
      paused.push(status);
    });
    clock.on("resumed", () => {
      events.push("resumed");
    });
    clock.advance(1);
    clock.pause();
    clock.pause();
    clock.resume();
    clock.resume();
    expect(events).toEqual(["paused", "resumed"]);
    expect(paused[0]).toMatchObject({ paused: true, elapsed: 1, speed: 1, day: 1, dayLength: 100 });
  });

  it("lets a listener of paused see the paused flag already changed", () => {
    const clock = new GameClock();
    const flags: boolean[] = [];
    clock.on("paused", () => {
      flags.push(clock.paused);
    });
    clock.pause();
    expect(flags).toEqual([true]);
  });

  it("adds, runs once, and removes listeners through on, once and off", () => {
    const clock = new GameClock();
    let always = 0;
    let once = 0;
    const listener = (): void => {
      always += 1;
    };
    clock.on("paused", listener);
    clock.once("paused", () => {
      once += 1;
    });
    clock.pause();
    clock.resume();
    clock.pause();
    expect(always).toBe(2);
    expect(once).toBe(1);
    clock.off("paused", listener);
    clock.resume();
    clock.pause();
    expect(always).toBe(2);
  });

  it("removes a listener with the function that on returned", () => {
    const clock = new GameClock();
    let calls = 0;
    const unsubscribe = clock.on("resumed", () => {
      calls += 1;
    });
    clock.pause();
    clock.resume();
    unsubscribe();
    clock.pause();
    clock.resume();
    expect(calls).toBe(1);
  });
});

describe("GameClock speed events", () => {
  it("sends speed with the previous and the new speed, only when the speed changes", () => {
    const clock = new GameClock({ speeds: [1, 2, 3] });
    const changes: Array<{ previous: number; current: number }> = [];
    clock.on("speed", (change) => {
      changes.push(change);
    });
    clock.setSpeed(2);
    clock.setSpeed(2);
    clock.cycleSpeed();
    clock.cycleSpeed();
    expect(changes).toEqual([
      { previous: 1, current: 2 },
      { previous: 2, current: 3 },
      { previous: 3, current: 1 },
    ]);
  });

  it("sends nothing when the clock has one speed and cycles, and nothing when setSpeed is refused", () => {
    const single = new GameClock({ speeds: [1] });
    let changes = 0;
    single.on("speed", () => {
      changes += 1;
    });
    single.cycleSpeed();
    expect(changes).toBe(0);

    const clock = new GameClock({ speeds: [1, 2] });
    clock.on("speed", () => {
      changes += 1;
    });
    expect(() => clock.setSpeed(5)).toThrow(RangeError);
    expect(changes).toBe(0);
  });
});

describe("GameClock stepped event", () => {
  it("sends stepped after each advance of a running clock, with the game seconds that it moved", () => {
    const clock = new GameClock({ speeds: [1, 2] });
    const steps: Array<{ dt: number; gameSeconds: number }> = [];
    clock.on("stepped", (step) => {
      steps.push(step);
    });
    clock.setSpeed(2);
    clock.advance(0.5);
    expect(steps).toEqual([{ dt: 0.5, gameSeconds: 1 }]);
  });

  it("sends nothing while the clock is paused", () => {
    const clock = new GameClock();
    let steps = 0;
    clock.on("stepped", () => {
      steps += 1;
    });
    clock.pause();
    clock.advance(1);
    expect(steps).toBe(0);
  });

  it("sends stepped after onDay, so a listener of stepped sees the new day", () => {
    const order: string[] = [];
    const clock = new GameClock({
      dayLength: 10,
      onDay: (day) => {
        order.push(`onDay ${day}`);
      },
    });
    clock.on("stepped", () => {
      order.push(`stepped on day ${clock.day}`);
    });
    clock.advance(12);
    expect(order).toEqual(["onDay 2", "stepped on day 2"]);
  });
});

describe("GameClock state()", () => {
  it("gives what a save keeps, with the day, the time of the day, the day length and the speeds", () => {
    const clock = new GameClock({ speeds: [1, 2], dayLength: 10 });
    clock.setSpeed(2);
    clock.advance(12);
    clock.pause();
    expect(clock.state()).toEqual({
      elapsed: 24,
      speed: 2,
      paused: true,
      day: 3,
      timeOfDay: 4,
      dayLength: 10,
      speeds: [1, 2],
    });
    expect(clock.state()).toMatchObject(clock.snapshot());
  });

  it("is a copy: changing it does not change the clock", () => {
    const clock = new GameClock({ speeds: [1, 2] });
    const state = clock.state();
    (state.speeds as number[]).push(9);
    expect(clock.speeds).toEqual([1, 2]);
    expect(clock.state().speeds).toEqual([1, 2]);
  });
});

describe("GameClock restore events", () => {
  it("sends the speed and pause changes that restore makes, after the clock holds the saved time", () => {
    const clock = new GameClock({ speeds: [1, 2] });
    const log: string[] = [];
    clock.on("speed", (change) => {
      log.push(`speed ${change.previous} to ${change.current}, elapsed ${clock.elapsed}`);
    });
    clock.on("paused", () => {
      log.push(`paused, elapsed ${clock.elapsed}`);
    });
    clock.on("resumed", () => {
      log.push("resumed");
    });
    clock.restore({ elapsed: 15, speed: 2, paused: true });
    clock.restore({ elapsed: 16, speed: 2, paused: false });
    expect(log).toEqual(["speed 1 to 2, elapsed 15", "paused, elapsed 15", "resumed"]);
  });

  it("sends no pause event when the restored pause is the one the clock already has", () => {
    const clock = new GameClock();
    clock.pause();
    let pauses = 0;
    clock.on("paused", () => {
      pauses += 1;
    });
    clock.restore({ elapsed: 5, speed: 1, paused: true });
    expect(pauses).toBe(0);
  });

  it("changes nothing and sends nothing when the saved speed is not allowed", () => {
    const clock = new GameClock({ speeds: [1, 2] });
    let events = 0;
    clock.on("speed", () => {
      events += 1;
    });
    expect(() => clock.restore({ elapsed: 3, speed: 7, paused: false })).toThrow(RangeError);
    expect(clock.elapsed).toBe(0);
    expect(events).toBe(0);
  });
});
