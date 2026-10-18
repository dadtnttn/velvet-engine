// A run of the level: where the hero is, which coins are taken, the last checkpoint, the hits and the time. A run
// has no drawing and no sound. A step returns the events it caused (a jump, a coin, a hit, the flag), and the game
// turns them into sound, achievements and screens. So the rules can be tested without a page.

import type { Box, CollisionGrid } from "@velvet/tiles";
import { CHECKPOINT_BOX, COIN_BOX, GOAL_BOX, HERO, RESPAWN_SECONDS } from "./config";
import type { Level, LevelPoint } from "./level";
import { heroAtFeet, stepHero, type Hero, type Intent } from "./physics";

/** Where a run is: on its way, waiting to reappear after a hit, or at the flag. */
export type RunPhase = "playing" | "hurt" | "finished";

/** What hurt the hero: the spikes, or a fall out of the level. */
export type HurtReason = "spikes" | "fall";

/** What a step made happen. */
export type WorldEvent =
  | { readonly type: "jump" }
  | { readonly type: "land" }
  | { readonly type: "coin"; readonly index: number; readonly coins: number }
  | { readonly type: "checkpoint"; readonly index: number }
  | { readonly type: "hurt"; readonly reason: HurtReason; readonly hurts: number }
  | { readonly type: "respawn" }
  | { readonly type: "finish"; readonly time: number; readonly coins: number; readonly hurts: number };

/** A small epsilon that keeps a box that ends exactly on a cell's edge in the cell it leaves. */
const EDGE = 1e-6;

/** Whether two boxes overlap, the edges not counted as overlap. */
function overlaps(a: Box, b: Box): boolean {
  return a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y;
}

/** The box that a point of the level has around it, of the given size, centred on the point. */
function boxAround(point: LevelPoint, width: number, height: number): Box {
  return { x: point.x - width / 2, y: point.y - height / 2, width, height };
}

/** The box that a checkpoint or the goal stands in: its pole, from the point up to its height. */
function boxAbove(point: LevelPoint, width: number, height: number): Box {
  return { x: point.x - width / 2, y: point.y - height, width, height };
}

/** Whether any cell that a box covers is not empty in a grid. */
function touchesCells(grid: CollisionGrid, box: Box): boolean {
  const left = Math.floor(box.x / grid.tileWidth);
  const right = Math.floor((box.x + box.width - EDGE) / grid.tileWidth);
  const top = Math.floor(box.y / grid.tileHeight);
  const bottom = Math.floor((box.y + box.height - EDGE) / grid.tileHeight);
  for (let row = top; row <= bottom; row += 1) {
    for (let column = left; column <= right; column += 1) {
      if (grid.kindAt(column, row) !== "empty") return true;
    }
  }
  return false;
}

/** One run of a level, from the start to the flag or until it is stopped. */
export class Run {
  /** The level that the run plays. */
  readonly level: Level;
  private heroState: Hero;
  private phaseValue: RunPhase = "playing";
  private time = 0;
  private hurtCount = 0;
  private coinCount = 0;
  private checkpointIndex = 0;
  private godMode = false;
  private taken: boolean[];
  private respawnTimer = 0;

  /**
   * Starts a run at the start of the level.
   * @param level - the level to play.
   */
  constructor(level: Level) {
    this.level = level;
    this.taken = level.coins.map(() => false);
    this.heroState = heroAtFeet(level.spawn);
  }

  /** The hero now. */
  get hero(): Hero {
    return this.heroState;
  }

  /** Where the run is now. */
  get phase(): RunPhase {
    return this.phaseValue;
  }

  /** Seconds of play so far, counted while the run is not finished. */
  get elapsed(): number {
    return this.time;
  }

  /** How many times the hero was hurt in this run. */
  get hurts(): number {
    return this.hurtCount;
  }

  /** How many coins this run has taken. */
  get coins(): number {
    return this.coinCount;
  }

  /** The index of the last checkpoint reached: 0 is the start. */
  get checkpoint(): number {
    return this.checkpointIndex;
  }

  /** Whether the hero cannot be hurt (the developer's god mode). */
  get god(): boolean {
    return this.godMode;
  }

  /** Turns the god mode on or off. */
  setGod(on: boolean): void {
    this.godMode = on;
  }

  /**
   * Whether a coin is taken.
   * @param index - the coin's index in the level.
   */
  isTaken(index: number): boolean {
    return this.taken[index] === true;
  }

  /** The box of the hero now, in level pixels. */
  heroBox(): Box {
    return { x: this.heroState.x, y: this.heroState.y, width: HERO.width, height: HERO.height };
  }

  /** Starts the run again from the start: the coins come back and the hits are forgotten. */
  restart(): void {
    this.taken = this.level.coins.map(() => false);
    this.coinCount = 0;
    this.hurtCount = 0;
    this.time = 0;
    this.checkpointIndex = 0;
    this.phaseValue = "playing";
    this.respawnTimer = 0;
    this.heroState = heroAtFeet(this.level.spawn);
  }

  /**
   * Plays one fixed step of the run.
   * @param intent - what the player asks for.
   * @param dt - the length of the step, in seconds.
   * @returns what the step made happen, in order.
   */
  step(intent: Intent, dt: number): WorldEvent[] {
    const events: WorldEvent[] = [];
    if (this.phaseValue === "finished") return events;
    this.time += dt;
    if (this.phaseValue === "hurt") {
      this.respawnTimer -= dt;
      if (this.respawnTimer <= 0) {
        this.heroState = heroAtFeet(this.level.checkpoints[this.checkpointIndex] ?? this.level.spawn);
        this.phaseValue = "playing";
        events.push({ type: "respawn" });
      }
      return events;
    }

    const outcome = stepHero(this.level.solid, this.heroState, intent, dt);
    this.heroState = outcome.hero;
    if (outcome.jumped) events.push({ type: "jump" });
    if (outcome.landed) events.push({ type: "land" });

    const box = this.heroBox();
    if (touchesCells(this.level.hazards, box) && !this.godMode) {
      this.hurt("spikes", events);
      return events;
    }
    if (box.y > this.level.height) {
      this.hurt("fall", events);
      return events;
    }
    this.reachCheckpoints(box, events);
    this.takeCoins(box, events);
    if (overlaps(box, boxAbove(this.level.goal, GOAL_BOX.width, GOAL_BOX.height))) {
      this.phaseValue = "finished";
      events.push({ type: "finish", time: this.time, coins: this.coinCount, hurts: this.hurtCount });
    }
    return events;
  }

  /**
   * Puts the hero's feet on a point of the level. It is the developer's `tp` command: the hero falls from there.
   * @param x - the x of the feet, in level pixels.
   * @param y - the y of the feet, in level pixels.
   */
  teleport(x: number, y: number): void {
    this.heroState = heroAtFeet({ x, y });
    this.phaseValue = "playing";
    this.respawnTimer = 0;
  }

  /**
   * Takes the next coins that are not taken yet: the developer's `coins` command. Each one is an ordinary pickup.
   * @param count - how many coins to take, at most those that are left.
   * @returns the events of the pickups.
   */
  giveCoins(count: number): WorldEvent[] {
    const events: WorldEvent[] = [];
    for (let index = 0; index < this.taken.length && count > 0; index += 1) {
      if (this.taken[index]) continue;
      this.taken[index] = true;
      this.coinCount += 1;
      count -= 1;
      events.push({ type: "coin", index, coins: this.coinCount });
    }
    return events;
  }

  /**
   * Hurts the hero: it counts the hit, and it waits to reappear at the last checkpoint.
   * @param reason - what hurt it.
   * @param events - the list that the step is collecting.
   */
  private hurt(reason: HurtReason, events: WorldEvent[]): void {
    this.hurtCount += 1;
    this.phaseValue = "hurt";
    this.respawnTimer = RESPAWN_SECONDS;
    events.push({ type: "hurt", reason, hurts: this.hurtCount });
  }

  /**
   * Makes the furthest checkpoint that the hero touched the one it returns to.
   * @param box - the hero's box.
   * @param events - the list that the step is collecting.
   */
  private reachCheckpoints(box: Box, events: WorldEvent[]): void {
    const checkpoints = this.level.checkpoints;
    for (let index = this.checkpointIndex + 1; index < checkpoints.length; index += 1) {
      const point = checkpoints[index];
      if (point === undefined || !overlaps(box, boxAbove(point, CHECKPOINT_BOX.width, CHECKPOINT_BOX.height))) continue;
      this.checkpointIndex = index;
      events.push({ type: "checkpoint", index });
    }
  }

  /**
   * Takes the coins that the hero touches.
   * @param box - the hero's box.
   * @param events - the list that the step is collecting.
   */
  private takeCoins(box: Box, events: WorldEvent[]): void {
    this.level.coins.forEach((coin, index) => {
      if (this.taken[index] || !overlaps(box, boxAround(coin, COIN_BOX.width, COIN_BOX.height))) return;
      this.taken[index] = true;
      this.coinCount += 1;
      events.push({ type: "coin", index, coins: this.coinCount });
    });
  }
}

/**
 * The text of a time in seconds, as the score line shows it: minutes, seconds and hundredths, such as 1:05.40.
 * @param seconds - the time, from 0 up.
 */
export function formatTime(seconds: number): string {
  const hundredths = Math.max(0, Math.floor(seconds * 100));
  const minutes = Math.floor(hundredths / 6000);
  const wholeSeconds = Math.floor((hundredths % 6000) / 100);
  const rest = hundredths % 100;
  return `${minutes}:${String(wholeSeconds).padStart(2, "0")}.${String(rest).padStart(2, "0")}`;
}
