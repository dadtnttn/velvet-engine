// The step that the camera and the animations see. A loop gives every system the same real step;
// a system with a game clock scales it the way the clock does, so nothing moves while the game is
// paused, and everything moves at the clock's speed. Without a clock the step is used as it is.

import type { GameClock } from "@velvet/core";

/**
 * The step a system moves by, for a real step of `dt` seconds.
 * @throws RangeError when `dt` is negative or not finite.
 */
export function clockStep(dt: number, clock: GameClock | undefined): number {
  if (!Number.isFinite(dt) || dt < 0) {
    throw new RangeError(`dt must be a non-negative finite number, got ${dt}`);
  }
  if (clock === undefined) return dt;
  return clock.paused ? 0 : dt * clock.speed;
}
