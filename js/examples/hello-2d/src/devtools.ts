// The developer tools of hello-2d, for development builds only. Game.start loads this module with a dynamic import
// behind `import.meta.env.DEV`, so a production build leaves it out. The tools are an F3 overlay with watches on the
// hero and the coins, a state view of the run, and three commands: `tp <x> <y>` moves the hero's feet, `coins <n>`
// takes the next coins, and `god on|off` keeps the hero from being hurt. The watches sit in the bottom right corner,
// where the level has no text. The commands' work is in plain functions, so that the tests can call them.

import type { EventEmitter, GameClock, GameLoop, Logger } from "@velvet/core";
import { createDevtools, type Devtools } from "@velvet/devtools";
import type { Stage } from "@velvet/ui";
import type { GameEvents } from "./game";
import type { Level } from "./level";
import type { Run } from "./world";

/** What the tools read and drive. The game passes its own parts; the tools hold no state of their own. */
export interface DevtoolsTarget {
  /** The element that the overlay is added to. */
  readonly host: HTMLElement;
  /** The game clock: the tools pause, resume and step it. */
  readonly clock: GameClock;
  /** The game loop, which gives the tools the frame times. */
  readonly loop: GameLoop;
  /** The stage, which the performance panel describes. */
  readonly stage: Stage;
  /** The game's events, which the event log shows. */
  readonly events: EventEmitter<GameEvents>;
  /** The game's logger, whose entries the log tab shows. */
  readonly logger: Logger;
  /** The level, for its size and its coins. */
  readonly level: Level;
  /** The run, read on each use. */
  readonly run: () => Run;
  /** Puts the hero's feet at a point of the level, and moves the camera there. */
  readonly teleport: (x: number, y: number) => void;
  /** Takes the next coins of the level, as ordinary pickups. */
  readonly giveCoins: (count: number) => void;
  /** Turns the god mode on or off. */
  readonly setGod: (on: boolean) => void;
}

/**
 * The `tp` command: puts the hero's feet at a point of the level.
 * @param target - the parts the tools drive.
 * @param x - the x of the feet, in level pixels.
 * @param y - the y of the feet, in level pixels.
 * @returns the line that the console prints.
 * @throws RangeError when the point is not a number, or lies outside the level.
 */
export function placeHero(target: DevtoolsTarget, x: number, y: number): string {
  if (!Number.isFinite(x) || !Number.isFinite(y)) throw new RangeError("tp needs two numbers");
  if (x < 0 || y < 0 || x > target.level.width || y > target.level.height) {
    throw new RangeError(`the point must be inside the level: 0 to ${target.level.width} by 0 to ${target.level.height}`);
  }
  target.teleport(x, y);
  return `hero at ${Math.round(x)}, ${Math.round(y)}`;
}

/**
 * The `coins` command: takes the next coins of the level, as ordinary pickups.
 * @param target - the parts the tools drive.
 * @param count - how many coins, a whole number from 1 up.
 * @returns the line that the console prints.
 * @throws RangeError when the count is not a whole number from 1 up.
 */
export function takeCoins(target: DevtoolsTarget, count: number): string {
  if (!Number.isInteger(count) || count < 1) throw new RangeError("coins needs a whole number from 1 up");
  target.giveCoins(count);
  return `coins: ${target.run().coins} of ${target.level.coins.length}`;
}

/**
 * The `god` command: the hero cannot be hurt while it is on.
 * @param target - the parts the tools drive.
 * @param on - true to turn the god mode on.
 * @returns the line that the console prints.
 */
export function setGod(target: DevtoolsTarget, on: boolean): string {
  target.setGod(on);
  return `god ${on ? "on" : "off"}`;
}

/**
 * Makes the developer tools of the game: the overlay, the watches, the state view and the three commands.
 * @param target - the parts the tools read and drive.
 * @returns the tools; `destroy` removes them.
 */
export function mountDevtools(target: DevtoolsTarget): Devtools {
  const tools = createDevtools<GameEvents>({
    host: target.host,
    enabled: true,
    clock: target.clock,
    loop: target.loop,
    events: target.events,
    stage: target.stage,
    logger: target.logger,
    hudCorner: "bottom-right",
  });

  tools.watch("x", () => Math.round(target.run().hero.x));
  tools.watch("y", () => Math.round(target.run().hero.y));
  tools.watch("vx", () => Math.round(target.run().hero.vx));
  tools.watch("vy", () => Math.round(target.run().hero.vy));
  tools.watch("grounded", () => (target.run().hero.grounded ? 1 : 0));
  tools.watch("coins", () => target.run().coins);
  tools.inspect("run", () => {
    const run = target.run();
    return { phase: run.phase, elapsed: run.elapsed, hurts: run.hurts, checkpoint: run.checkpoint, god: run.god };
  });

  tools.command("tp", {
    args: [
      { name: "x", type: "number" },
      { name: "y", type: "number" },
    ],
    help: "puts the hero's feet at (x, y), in level pixels",
    run: (values) => placeHero(target, Number(values["x"]), Number(values["y"])),
  });
  tools.command("coins", {
    args: [{ name: "count", type: "number" }],
    help: "takes the next coins of the level, as ordinary pickups",
    run: (values) => takeCoins(target, Math.floor(Number(values["count"]))),
  });
  tools.command("god", {
    args: [{ name: "state", type: "boolean" }],
    help: "god on|off: the hero cannot be hurt",
    run: (values) => setGod(target, values["state"] === true),
  });

  return tools;
}
