// Tests of the hero's motion: landing on the ground and on a one-way plank from above, passing a plank from below,
// bumping a ceiling, the coyote time and the jump buffer, and the variable height of a jump. The levels are drawn
// as text, so each rule is checked on the smallest map that shows it.

import { describe, expect, it } from "vitest";
import { HERO, PHYSICS, STEPS_PER_SECOND } from "./config";
import { heroAtFeet, stepHero, type Hero, type Intent } from "./physics";
import { groundFromRows, type Ground } from "./test-support";

const DT = 1 / STEPS_PER_SECOND;
const STILL: Intent = { move: 0, jumpPressed: false, jumpHeld: false };
const WALK_RIGHT: Intent = { move: 1, jumpPressed: false, jumpHeld: false };

/** Runs steps with one intent, and returns every hero on the way (the first one is the start). */
function run(level: Ground, start: Hero, intent: Intent, steps: number): Hero[] {
  const heroes: Hero[] = [start];
  let hero = start;
  for (let step = 0; step < steps; step += 1) {
    hero = stepHero(level.solid, hero, intent, DT).hero;
    heroes.push(hero);
  }
  return heroes;
}

/** The hero after some steps with no input: it has landed. */
function settled(level: Ground, start: Hero, steps = 90): Hero {
  return run(level, start, STILL, steps).at(-1) ?? start;
}

/**
 * The jump: the button goes down in the first step and stays down for `steps` steps. It returns every hero on the way.
 */
function jump(level: Ground, start: Hero, steps: number): Hero[] {
  const heroes: Hero[] = [start];
  let hero = start;
  for (let step = 0; step < steps; step += 1) {
    hero = stepHero(level.solid, hero, { move: 0, jumpPressed: step === 0, jumpHeld: true }, DT).hero;
    heroes.push(hero);
  }
  return heroes;
}

/** The bottom edge of the hero's box. */
const bottomOf = (hero: Hero): number => hero.y + HERO.height;

describe("landing", () => {
  it("falls onto the ground and stands on its top edge", async () => {
    const level = await groundFromRows(["..........", "..S.......", "..........", "##########"]);
    const last = settled(level, heroAtFeet(level.spawn));
    expect(last.grounded).toBe(true);
    expect(bottomOf(last)).toBeCloseTo(96, 5);
    expect(last.vy).toBe(0);
  });

  it("reports the landing once, in the step where the hero reaches the ground", async () => {
    const level = await groundFromRows(["..........", "..S.......", "..........", "##########"]);
    let hero = heroAtFeet(level.spawn);
    let landings = 0;
    for (let step = 0; step < 90; step += 1) {
      const outcome = stepHero(level.solid, hero, STILL, DT);
      if (outcome.landed) landings += 1;
      hero = outcome.hero;
    }
    expect(landings).toBe(1);
  });
});

describe("one-way planks", () => {
  it("lets the hero land on a plank from above", async () => {
    const level = await groundFromRows(["..........", "..S.......", "-----.....", ".........."]);
    // The plank is in row 2, so its top is at y = 64 and the hero stands with its bottom at 64.
    const last = settled(level, heroAtFeet(level.spawn));
    expect(last.grounded).toBe(true);
    expect(bottomOf(last)).toBeCloseTo(64, 5);
  });

  it("lets the hero jump up through a plank from below", async () => {
    const level = await groundFromRows(["..........", "-----.....", "..........", "##########"]);
    // The hero stands on the ground in row 3, whose top is at y = 96. The plank in row 1 is above it.
    const start = settled(level, heroAtFeet({ x: 80, y: 96 }));
    const heroes = jump(level, start, 30);
    const highest = Math.min(...heroes.map((hero) => hero.y));
    expect(highest).toBeLessThan(32 - HERO.height);
  });

  it("does not block the hero sideways", async () => {
    const level = await groundFromRows([".........", "-----....", ".........", "#########"]);
    // The plank is above the hero's head, and a one-way plank never stops sideways movement.
    const start = settled(level, heroAtFeet({ x: 16, y: 96 }));
    const heroes = run(level, start, WALK_RIGHT, 60);
    expect((heroes.at(-1)?.x ?? 0) - start.x).toBeGreaterThan(150);
  });
});

describe("ceilings", () => {
  it("bumps the head on a ceiling and falls back", async () => {
    const level = await groundFromRows(["##########", "..........", "..S.......", "##########"]);
    // The ceiling's bottom is at y = 32; the ground's top is at y = 96.
    const start = settled(level, heroAtFeet(level.spawn));
    const heroes = jump(level, start, 40);
    const highest = Math.min(...heroes.map((hero) => hero.y));
    expect(highest).toBeGreaterThanOrEqual(32 - 1e-6);
    const bumped = heroes.find((hero) => hero.y <= 32 + 1e-6);
    expect(bumped?.vy).toBe(0);
  });
});

describe("coyote time and the jump buffer", () => {
  /** The level with a ledge: ground under the first four cells only, and the hero walking right off it. */
  async function leaveLedge(): Promise<{ level: Ground; hero: Hero }> {
    const level = await groundFromRows(["..........", "..S.......", "####......"]);
    let hero = settled(level, heroAtFeet(level.spawn));
    for (let step = 0; step < 300 && hero.grounded; step += 1) hero = stepHero(level.solid, hero, WALK_RIGHT, DT).hero;
    expect(hero.grounded).toBe(false);
    return { level, hero };
  }

  it("still jumps a few steps after the hero walks off a ledge", async () => {
    const { level, hero } = await leaveLedge();
    let current = hero;
    let jumped = false;
    for (let step = 0; step < 3 && !jumped; step += 1) {
      const outcome = stepHero(level.solid, current, { move: 1, jumpPressed: step === 2, jumpHeld: true }, DT);
      jumped = outcome.jumped;
      current = outcome.hero;
    }
    expect(jumped).toBe(true);
  });

  it("does not jump once the coyote time has run out", async () => {
    const { level, hero } = await leaveLedge();
    let current = hero;
    const steps = Math.ceil(PHYSICS.coyote * STEPS_PER_SECOND) + 4;
    for (let step = 0; step < steps; step += 1) current = stepHero(level.solid, current, WALK_RIGHT, DT).hero;
    const late = stepHero(level.solid, current, { move: 1, jumpPressed: true, jumpHeld: true }, DT);
    expect(late.jumped).toBe(false);
  });

  it("jumps on landing when the press came shortly before the landing", async () => {
    const level = await groundFromRows(["..........", "..S.......", "..........", "##########"]);
    let hero = heroAtFeet({ x: 80, y: 40 });
    // Falls until the hero is 12 px above the ground: then the press comes, and the buffer holds it until the landing.
    for (let step = 0; step < 300 && bottomOf(hero) < 84; step += 1) hero = stepHero(level.solid, hero, STILL, DT).hero;
    let jumped = false;
    for (let step = 0; step < 120 && !jumped; step += 1) {
      const outcome = stepHero(level.solid, hero, { move: 0, jumpPressed: step === 0, jumpHeld: true }, DT);
      jumped = outcome.jumped;
      hero = outcome.hero;
    }
    expect(jumped).toBe(true);
  });

  it("forgets a press that came long before the landing", async () => {
    const level = await groundFromRows(["..........", "..S.......", "..........", "##########"]);
    let hero = heroAtFeet({ x: 80, y: 40 });
    const press = stepHero(level.solid, hero, { move: 0, jumpPressed: true, jumpHeld: false }, DT);
    let jumped = press.jumped;
    hero = press.hero;
    for (let step = 0; step < 120; step += 1) {
      const outcome = stepHero(level.solid, hero, STILL, DT);
      jumped = jumped || outcome.jumped;
      hero = outcome.hero;
    }
    expect(jumped).toBe(false);
    expect(hero.grounded).toBe(true);
  });
});

describe("the height of a jump", () => {
  it("is much lower when the jump button is let go in the first step", async () => {
    const level = await groundFromRows(["..........", "..S.......", "..........", "##########"]);
    const start = settled(level, heroAtFeet(level.spawn));
    const held = jump(level, start, 60);
    // A tap: the button is down for one step only, so the rise is cut to the cut speed.
    let tap = stepHero(level.solid, start, { move: 0, jumpPressed: true, jumpHeld: false }, DT).hero;
    let tapHighest = tap.y;
    for (let step = 0; step < 60; step += 1) {
      tap = stepHero(level.solid, tap, STILL, DT).hero;
      tapHighest = Math.min(tapHighest, tap.y);
    }
    const heldHighest = Math.min(...held.map((hero) => hero.y));
    expect(tapHighest).toBeGreaterThan(heldHighest + 60);
  });

  it("reaches about three tiles when the button is held", async () => {
    const level = await groundFromRows(["..........", "..S.......", "..........", "##########"]);
    const start = settled(level, heroAtFeet(level.spawn));
    const held = jump(level, start, 60);
    const rise = bottomOf(start) - Math.min(...held.map((hero) => bottomOf(hero)));
    expect(rise).toBeGreaterThan(90);
    expect(rise).toBeLessThan(110);
  });
});
