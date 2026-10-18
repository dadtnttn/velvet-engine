// Tests of a run of the level: a coin is taken once, a checkpoint is kept, spikes send the hero back to it after the
// wait, the flag ends the run with its time, coins and hits, god mode keeps the hero whole, a fall hurts, and a
// restart gives everything back. The level is drawn as text, so each event has a known place.

import { describe, expect, it } from "vitest";
import { STEPS_PER_SECOND, RESPAWN_SECONDS } from "./config";
import type { Intent } from "./physics";
import { formatTime, Run, type WorldEvent } from "./world";
import { levelFromRows } from "./test-support";

const DT = 1 / STEPS_PER_SECOND;
const STILL: Intent = { move: 0, jumpPressed: false, jumpHeld: false };

/** A small level: the start at the left, two coins, a checkpoint, a spike and the flag, on one ground row. */
const ROWS = [
  "..............",
  "..........C...",
  "S..o...^..o..F",
  "##############",
];

/** Where the things of the small level are, in level pixels (the points at the bottom centre of their cells). */
const SPIKE_FEET = { x: 240, y: 96 };
const CHECKPOINT_FEET = { x: 336, y: 64 };
const GOAL_FEET = { x: 432, y: 96 };

/** Runs steps with no input, and gathers the events. */
function idle(run: Run, steps: number): WorldEvent[] {
  const events: WorldEvent[] = [];
  for (let step = 0; step < steps; step += 1) events.push(...run.step(STILL, DT));
  return events;
}

describe("coins", () => {
  it("are taken once, when the hero touches them", async () => {
    const run = new Run(await levelFromRows(ROWS));
    run.teleport(112, 88);
    const first = run.step(STILL, DT);
    expect(first).toContainEqual({ type: "coin", index: 0, coins: 1 });
    expect(run.isTaken(0)).toBe(true);
    expect(idle(run, 10).some((event) => event.type === "coin" && event.index === 0)).toBe(false);
    expect(run.coins).toBe(1);
  });

  it("can be given by the developer, in order, up to the number left", async () => {
    const run = new Run(await levelFromRows(ROWS));
    const events = run.giveCoins(5);
    expect(events.map((event) => (event.type === "coin" ? event.index : -1))).toEqual([0, 1]);
    expect(run.coins).toBe(2);
    expect(run.giveCoins(1)).toEqual([]);
  });
});

describe("checkpoints and hits", () => {
  it("returns the hero to the last checkpoint after the spikes hurt it", async () => {
    const run = new Run(await levelFromRows(ROWS));
    run.teleport(CHECKPOINT_FEET.x, CHECKPOINT_FEET.y - 4);
    expect(run.step(STILL, DT)).toContainEqual({ type: "checkpoint", index: 1 });
    expect(run.checkpoint).toBe(1);

    idle(run, 40);
    run.teleport(SPIKE_FEET.x, SPIKE_FEET.y);
    const hit = run.step(STILL, DT);
    expect(hit).toContainEqual({ type: "hurt", reason: "spikes", hurts: 1 });
    expect(run.phase).toBe("hurt");

    // The hero reappears in the step that ends the wait, exactly at the checkpoint, before any physics of that step.
    let respawned = false;
    for (let step = 0; step < Math.ceil(RESPAWN_SECONDS * STEPS_PER_SECOND) + 2 && !respawned; step += 1) {
      respawned = run.step(STILL, DT).some((event) => event.type === "respawn");
    }
    expect(respawned).toBe(true);
    expect(run.phase).toBe("playing");
    expect(run.hero.x).toBe(CHECKPOINT_FEET.x - 10);
    expect(run.hero.y).toBe(CHECKPOINT_FEET.y - 28);
  });

  it("waits for the respawn without moving the hero", async () => {
    const run = new Run(await levelFromRows(ROWS));
    run.teleport(SPIKE_FEET.x, SPIKE_FEET.y);
    run.step(STILL, DT);
    const frozen = { x: run.hero.x, y: run.hero.y };
    idle(run, 5);
    expect({ x: run.hero.x, y: run.hero.y }).toEqual(frozen);
  });

  it("counts the time of the wait in the run, as it counts the time of play", async () => {
    const run = new Run(await levelFromRows(ROWS));
    run.teleport(SPIKE_FEET.x, SPIKE_FEET.y);
    const before = run.elapsed;
    idle(run, 10);
    expect(run.elapsed - before).toBeCloseTo(10 * DT, 9);
  });

  it("hurts the hero when it falls out of the level, with the reason fall", async () => {
    const run = new Run(await levelFromRows(ROWS));
    run.teleport(16, 1000);
    expect(run.step(STILL, DT)).toContainEqual({ type: "hurt", reason: "fall", hurts: 1 });
  });

  it("does not hurt the hero in god mode", async () => {
    const run = new Run(await levelFromRows(ROWS));
    run.setGod(true);
    run.teleport(SPIKE_FEET.x, SPIKE_FEET.y);
    const events = run.step(STILL, DT);
    expect(events.some((event) => event.type === "hurt")).toBe(false);
    expect(run.phase).toBe("playing");
    expect(run.hurts).toBe(0);
  });
});

describe("the flag", () => {
  it("ends the run with the time, the coins and the hits", async () => {
    const run = new Run(await levelFromRows(ROWS));
    run.giveCoins(2);
    idle(run, 30);
    run.teleport(GOAL_FEET.x, GOAL_FEET.y);
    const events = run.step(STILL, DT);
    const finish = events.find((event) => event.type === "finish");
    expect(finish?.type).toBe("finish");
    if (finish?.type === "finish") {
      expect(finish.coins).toBe(2);
      expect(finish.hurts).toBe(0);
      expect(finish.time).toBeGreaterThan(0.5);
      expect(finish.time).toBeLessThan(0.6);
    }
    expect(run.phase).toBe("finished");
  });

  it("stops the run: no more events and no more time", async () => {
    const run = new Run(await levelFromRows(ROWS));
    run.teleport(GOAL_FEET.x, GOAL_FEET.y);
    run.step(STILL, DT);
    const time = run.elapsed;
    expect(idle(run, 20)).toEqual([]);
    expect(run.elapsed).toBe(time);
  });
});

describe("restarting", () => {
  it("gives the coins, the hits, the checkpoint and the hero's start back", async () => {
    const run = new Run(await levelFromRows(ROWS));
    run.giveCoins(2);
    run.teleport(CHECKPOINT_FEET.x, CHECKPOINT_FEET.y - 4);
    run.step(STILL, DT);
    run.teleport(SPIKE_FEET.x, SPIKE_FEET.y);
    run.step(STILL, DT);
    run.restart();
    expect(run.coins).toBe(0);
    expect(run.hurts).toBe(0);
    expect(run.checkpoint).toBe(0);
    expect(run.isTaken(0)).toBe(false);
    expect(run.phase).toBe("playing");
    expect(run.elapsed).toBe(0);
    expect(run.hero.x).toBe(16 - 10);
    expect(run.hero.y).toBe(96 - 28);
  });
});

describe("formatTime", () => {
  it("writes minutes, seconds and hundredths", () => {
    expect(formatTime(0)).toBe("0:00.00");
    expect(formatTime(65.4)).toBe("1:05.40");
    expect(formatTime(3599.999)).toBe("59:59.99");
    expect(formatTime(-1)).toBe("0:00.00");
  });
});
