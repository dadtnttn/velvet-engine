// Test support: a small game that the tests of this package run. It uses the parts that a real game uses
// (a loop, a clock, an action map and a seeded random source), so a recorded session and its replay go
// through the same code. Only the tests import this file; the package index does not export it.

import { ActionMap, GameClock, type GameLoop, type GameLoopOptions } from "@velvet/core";
import type { Driver } from "./driver";
import { hashState } from "./hash";
import { createRandom } from "./random";

/** The actions of the fake game. */
export type FakeAction = "jump" | "fire";

/** The state that the fake game keeps, and that its hash covers. */
export interface FakeState {
  coins: number;
  pointer: { x: number; y: number };
  fired: number;
}

/** Options for {@link startFakeGame}. */
export interface FakeGameOptions {
  /** The seed of the game's random source. Default 1. */
  readonly seed?: number | string;
  /** Coins that one press of "jump" gives, for each frame. The default gives one coin. */
  readonly jumpBonus?: (frame: number) => number;
  /** Coins that the game starts with, as a saved game would give them. Default 0. */
  readonly startCoins?: number;
  /** Wraps the loop's options, as a recorder does for a live page. Default: no wrapping. */
  readonly wrap?: (options: GameLoopOptions) => GameLoopOptions;
}

/** A started fake game. */
export interface FakeGame {
  readonly loop: GameLoop;
  readonly keys: ActionMap<FakeAction>;
  readonly clock: GameClock;
  readonly state: FakeState;
  /** The hash of the state: coins, pointer, clock and the number of shots. */
  hash(): string;
  /** Stops the loop and detaches the action map. */
  dispose(): void;
}

/**
 * Builds the fake game on a driver and starts its loop. The loop runs one update per frame, and each
 * update reads the action map, the clock and the seeded random source, as a real game's update does.
 */
export function startFakeGame(driver: Driver, options: FakeGameOptions = {}): FakeGame {
  const random = createRandom(options.seed ?? 1);
  const jumpBonus = options.jumpBonus ?? (() => 1);
  const clock = new GameClock({ dayLength: 2 });
  const state: FakeState = { coins: options.startCoins ?? 0, pointer: { x: 0, y: 0 }, fired: 0 };
  const keys = new ActionMap<FakeAction>([
    { action: "jump", control: { type: "key", key: "Space" } },
    { action: "fire", control: { type: "mouse", button: "left" } },
    { action: "fire", control: { type: "pad-button", button: "a" } },
  ]);
  keys.attach(driver.target);
  const loopOptions: GameLoopOptions = {
    update(dt: number): void {
      clock.advance(dt);
      if (keys.pressed("jump")) state.coins += jumpBonus(driver.frame);
      if (keys.held("fire")) {
        state.fired += 1;
        state.coins += random.int(1, 3);
      }
      state.pointer = keys.pointer();
      keys.endFrame();
    },
  };
  const loop = driver.createLoop(options.wrap === undefined ? loopOptions : options.wrap(loopOptions));
  loop.start();
  return {
    loop,
    keys,
    clock,
    state,
    hash: () => hashState({ coins: state.coins, fired: state.fired, pointer: state.pointer, clock: clock.snapshot() }),
    dispose: () => {
      loop.stop();
      keys.detach();
    },
  };
}
