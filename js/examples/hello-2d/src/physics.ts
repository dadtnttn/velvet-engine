// The hero's motion, one fixed step at a time, against the solid cells of a level. The step is a pure function of
// the hero, the player's intent and the grid: the same inputs give the same hero, so a replay gives the same run.
//
// Gravity pulls the hero down. A jump starts when a press is buffered (the press came shortly before) and the
// hero is grounded or has walked off a ledge within the coyote time. Letting go of the jump button early cuts the
// rise, so a tap gives a short hop and a held button gives the full jump. The movement is `moveBox`: the hero
// lands on the top of a solid cell or a one-way plank, and it bumps its head on a ceiling.

import { moveBox, type CollisionGrid } from "@velvet/tiles";
import { HERO, PHYSICS } from "./config";

/** The hero's motion: the top-left corner of its box, its speed, whether it touches the ground, and its timers. */
export interface Hero {
  /** The left edge of the box, in level pixels. */
  readonly x: number;
  /** The top edge of the box, in level pixels. */
  readonly y: number;
  /** The speed along x and y, in pixels a second. Positive is right and down. */
  readonly vx: number;
  readonly vy: number;
  /** True when the hero stands on something. */
  readonly grounded: boolean;
  /** Seconds left in which a jump still counts after the hero walked off a ledge. */
  readonly coyote: number;
  /** Seconds left in which a jump press still counts before the hero lands. */
  readonly buffer: number;
  /** Which way the hero faces: 1 right, -1 left. */
  readonly facing: 1 | -1;
}

/** What the player asks for in one step. `move` runs from -1 (left) to 1 (right). */
export interface Intent {
  readonly move: number;
  /** True in the step where the jump button went down. */
  readonly jumpPressed: boolean;
  /** True while the jump button is down. */
  readonly jumpHeld: boolean;
}

/** The result of one step: the new hero, and what happened in it. */
export interface StepOutcome {
  readonly hero: Hero;
  readonly jumped: boolean;
  readonly landed: boolean;
}

/**
 * A hero that stands with its feet on (`feet.x`, `feet.y`): the box's bottom centre. It starts at rest, in the air,
 * so the first step finds the ground.
 * @param feet - the bottom centre of the box, in level pixels.
 */
export function heroAtFeet(feet: { readonly x: number; readonly y: number }): Hero {
  return {
    x: feet.x - HERO.width / 2,
    y: feet.y - HERO.height,
    vx: 0,
    vy: 0,
    grounded: false,
    coyote: 0,
    buffer: 0,
    facing: 1,
  };
}

/**
 * Moves a value toward a target by at most `amount`, without passing the target.
 * @param value - the value now.
 * @param target - where it should go.
 * @param amount - the most it may change.
 */
function approach(value: number, target: number, amount: number): number {
  if (value < target) return Math.min(target, value + amount);
  return Math.max(target, value - amount);
}

/**
 * One fixed step of the hero.
 * @param grid - the solid cells of the level.
 * @param hero - the hero before the step.
 * @param intent - what the player asks for in this step.
 * @param dt - the length of the step, in seconds.
 * @returns the hero after the step, and whether it jumped or landed in it.
 */
export function stepHero(grid: CollisionGrid, hero: Hero, intent: Intent, dt: number): StepOutcome {
  const move = Math.max(-1, Math.min(1, intent.move));
  const accel = hero.grounded ? PHYSICS.groundAccel : PHYSICS.airAccel;
  let vx = approach(hero.vx, move * PHYSICS.run, accel * dt);
  let vy = hero.vy;

  let coyote = hero.grounded ? PHYSICS.coyote : Math.max(0, hero.coyote - dt);
  let buffer = intent.jumpPressed ? PHYSICS.jumpBuffer : Math.max(0, hero.buffer - dt);
  let jumped = false;
  if (buffer > 0 && coyote > 0) {
    vy = -PHYSICS.jumpSpeed;
    coyote = 0;
    buffer = 0;
    jumped = true;
  }
  // Letting go while the hero still rises cuts the rise: a tap gives a short hop.
  if (!intent.jumpHeld && vy < -PHYSICS.cutSpeed) vy = -PHYSICS.cutSpeed;
  vy = Math.min(vy + PHYSICS.gravity * dt, PHYSICS.maxFall);

  const moved = moveBox(grid, { x: hero.x, y: hero.y, width: HERO.width, height: HERO.height }, vx * dt, vy * dt);
  if (moved.contacts.left || moved.contacts.right) vx = 0;
  if (moved.contacts.top && vy < 0) vy = 0;
  if (moved.contacts.bottom && vy > 0) vy = 0;
  // A probe one unit down tells whether the hero stands on something, including a one-way plank.
  const grounded = moveBox(grid, moved.box, 0, 1).contacts.bottom;
  const landed = grounded && !hero.grounded;
  const facing = vx > 1 ? 1 : vx < -1 ? -1 : hero.facing;

  return {
    hero: {
      x: moved.box.x,
      y: moved.box.y,
      vx,
      vy,
      grounded,
      coyote,
      buffer,
      facing,
    },
    jumped,
    landed,
  };
}
