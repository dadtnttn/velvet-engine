// The developer tools of hello-cards, for development builds only. Game.start loads this module with a dynamic
// import behind `import.meta.env.DEV`, so a release build leaves it out. The tools are an F3 overlay with two
// watches (the cards on the table, and the coins among them) and a console command, `coins <count>`, that puts
// coin cards on the table.

import type { Board } from "@velvet/cards";
import type { GameClock, GameLoop } from "@velvet/core";
import { createDevtools, type Devtools } from "@velvet/devtools";
import type { Stage } from "@velvet/ui";

/** The most coins that one command puts on the table. */
const MAX_COINS = 20;

/** Where new coins are looked for: the free spot nearest to this point, in board units. */
const COIN_NEAR = { x: 4.5, y: 2.5 } as const;

/** The footprint of one coin card, in board units. */
const COIN_SIZE = { width: 1, height: 1 } as const;

/**
 * Puts coin cards on the table, each one on the free spot nearest to the middle of the table. A spot that is taken
 * is skipped, so no two stacks lie on one spot.
 * @param board - the board to change.
 * @param count - how many coins to add, from 1 to 20.
 * @returns the number of coin cards on the table now.
 * @throws RangeError when the count is not a whole number from 1 to 20.
 */
export function addCoins(board: Board, count: number): number {
  if (!Number.isInteger(count) || count < 1 || count > MAX_COINS) {
    throw new RangeError(`the number of coins must be a whole number from 1 to ${MAX_COINS}, got ${count}`);
  }
  for (let index = 0; index < count; index += 1) {
    const spot = board.freeSpot(COIN_NEAR, COIN_SIZE);
    if (spot === null) break;
    board.spawn("coin", spot.x, spot.y);
  }
  return coinCount(board);
}

/**
 * The coin cards on the table.
 * @param board - the board to count.
 */
export function coinCount(board: Board): number {
  return board.cards().filter((card) => card.def === "coin").length;
}

/** What the tools read and change. The game passes its own parts, and they are read again on each use. */
export interface DevtoolsTarget {
  /** The element that the overlay is added to. */
  readonly host: HTMLElement;
  /** The game clock: the tools pause, resume and step it. */
  readonly clock: GameClock;
  /** The game loop, which gives the tools the frame times. */
  readonly loop: GameLoop;
  /** The stage, which the performance panel describes. */
  readonly stage: Stage;
  /** The board. A load replaces it, so it is read on each use. */
  readonly board: () => Board;
  /** Lets the game apply the events that the board queued, so that new cards pop in and settle. */
  readonly flushEvents: () => void;
}

/**
 * Makes the developer tools of the game: the overlay, the two watches and the `coins` command.
 * @param target - the parts the tools read and change.
 * @returns the tools; `destroy` removes them.
 */
export function mountDevtools(target: DevtoolsTarget): Devtools {
  const tools = createDevtools({
    host: target.host,
    enabled: true,
    clock: target.clock,
    loop: target.loop,
    stage: target.stage,
  });
  tools.watch("cards", () => target.board().cards().length);
  tools.watch("coins", () => coinCount(target.board()));
  tools.command("coins", {
    args: [{ name: "count", type: "number" }],
    help: "puts coin cards on the table: coins 3 adds three",
    run: ({ count }) => {
      const total = addCoins(target.board(), Number(count));
      target.flushEvents();
      return `coins on the table: ${total}`;
    },
  });
  return tools;
}
