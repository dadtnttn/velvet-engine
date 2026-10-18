// Springs: a value that moves toward a goal the way a weight on a damped spring does. Use one for
// follow-style UI, such as a panel that slides after the pointer or a total that eases to a new
// number. The goal can change at any time. When the value is close enough to the goal and slow
// enough, the spring settles: it stops, sets the goal exactly, leaves its ticker and completes.
//
// The motion is m x'' = -k (x - goal) - c x', integrated with semi-implicit Euler in sub-steps of at
// most 1/240 s, and shorter still for a stiff spring, so that it stays stable. A long step (a frame
// that took half a second) is split into sub-steps and gives the same motion as many short frames.

import type { Outcome } from "./playback";
import { sharedTicker, type Playable, type Ticker } from "./ticker";

/** Longest sub-step, in seconds. */
const MAX_SUBSTEP = 1 / 240;

/** A sub-step is at most this fraction of one period of the spring's oscillation. */
const STEP_PER_PERIOD = 0.1;

/** Stiffness used when the options give none. */
const DEFAULT_STIFFNESS = 170;

/** Position and speed below which a spring counts as at rest, when the options give none. */
const DEFAULT_REST = 0.001;

/** Options for {@link spring}. */
export interface SpringOptions {
  /** The ticker that runs the spring. Default: the shared real-time ticker. */
  ticker?: Ticker;
  /** An object that the spring moves, for `Ticker.killTweensOf`. */
  target?: unknown;
  /** A name for `Ticker.killTag`. */
  tag?: string;
  /** Runs once, when the spring settles. */
  onComplete?(): void;
  /** The value at the start. Default 0. */
  from?: number;
  /** The goal. Default 0. */
  to?: number;
  /** The speed at the start, in value units per second. Default 0. */
  velocity?: number;
  /** How strongly the spring pulls toward the goal. Default 170. Must be above zero. */
  stiffness?: number;
  /** The weight on the spring. Default 1. Must be above zero. */
  mass?: number;
  /**
   * How much the motion is slowed. Default: critical damping for the stiffness and mass, which
   * reaches the goal as fast as possible without overshoot. Zero never settles.
   */
  damping?: number;
  /** How close to the goal the value must be to settle, in value units. Default 0.001. */
  restDelta?: number;
  /** How slow the value must be to settle, in value units per second. Default 0.001. */
  restSpeed?: number;
  /** Runs with the value each time it changes. It does not run when the spring is made. */
  onUpdate?(value: number): void;
}

/**
 * The damping that makes a spring critically damped: 2 times the square root of stiffness times
 * mass. A critically damped spring reaches its goal as soon as it can, without overshooting.
 * @throws RangeError when `stiffness` or `mass` is not a positive finite number.
 */
export function criticalDamping(stiffness: number, mass = 1): number {
  checkPositive(stiffness, "stiffness");
  checkPositive(mass, "mass");
  return 2 * Math.sqrt(stiffness * mass);
}

/** Throws unless `value` is a finite number. */
function checkFinite(value: number, name: string): number {
  if (!Number.isFinite(value)) throw new RangeError(`${name} must be a finite number, got ${value}`);
  return value;
}

/** Throws unless `value` is a finite number above zero. */
function checkPositive(value: number, name: string): number {
  if (!Number.isFinite(value) || value <= 0) throw new RangeError(`${name} must be above zero, got ${value}`);
  return value;
}

/** A value that follows a goal as a damped spring. Create it with {@link spring}. */
export class Spring implements Playable {
  /** The object that the spring moves, if any. */
  readonly target: unknown;
  /** A name for `Ticker.killTag`, if any. */
  readonly tag: string | undefined;
  /** Resolves once: "complete" when the spring settles, "cancelled" when it is cancelled. */
  readonly finished: Promise<Outcome>;
  private settleFinished: ((outcome: Outcome) => void) | undefined;
  private readonly ticker: Ticker;
  private readonly stiffness: number;
  private readonly mass: number;
  private readonly damping: number;
  private readonly restDelta: number;
  private readonly restSpeed: number;
  private readonly changed: ((value: number) => void) | undefined;
  private readonly completion: (() => void) | undefined;
  private position: number;
  private speed: number;
  private goal: number;
  private status: "running" | "complete" | "cancelled" = "running";
  private joined = false;

  /**
   * Starts a spring on its ticker.
   * @throws RangeError when a value is not finite, or stiffness, mass or a rest threshold is not
   * above zero, or damping is negative.
   */
  constructor(options: SpringOptions = {}) {
    const stiffness = checkPositive(options.stiffness ?? DEFAULT_STIFFNESS, "stiffness");
    const mass = checkPositive(options.mass ?? 1, "mass");
    const damping = options.damping ?? criticalDamping(stiffness, mass);
    if (!Number.isFinite(damping) || damping < 0) throw new RangeError(`damping must be zero or more, got ${damping}`);
    this.stiffness = stiffness;
    this.mass = mass;
    this.damping = damping;
    this.restDelta = checkPositive(options.restDelta ?? DEFAULT_REST, "restDelta");
    this.restSpeed = checkPositive(options.restSpeed ?? DEFAULT_REST, "restSpeed");
    this.position = checkFinite(options.from ?? 0, "from");
    this.goal = checkFinite(options.to ?? 0, "to");
    this.speed = checkFinite(options.velocity ?? 0, "velocity");
    this.target = options.target;
    this.tag = options.tag;
    this.ticker = options.ticker ?? sharedTicker();
    this.changed = options.onUpdate;
    this.completion = options.onComplete;
    let resolveOutcome: ((outcome: Outcome) => void) | undefined;
    this.finished = new Promise<Outcome>((resolve) => {
      resolveOutcome = resolve;
    });
    this.settleFinished = resolveOutcome;
    this.join();
  }

  /** The value now. */
  get value(): number {
    return this.position;
  }

  /** The speed now, in value units per second. */
  get velocity(): number {
    return this.speed;
  }

  /** The goal. Setting it wakes a settled spring, so that it moves to the new goal. */
  get to(): number {
    return this.goal;
  }

  set to(value: number) {
    this.goal = checkFinite(value, "to");
    if (this.status === "complete") {
      this.status = "running";
      this.join();
    }
  }

  /** True once the spring has settled on its goal. */
  get settled(): boolean {
    return this.status === "complete";
  }

  /** True once the spring has settled or been cancelled. */
  get isDone(): boolean {
    return this.status !== "running";
  }

  /**
   * Puts the value at `value` at rest, with no speed, and makes that the goal. It does not
   * change whether the spring has settled.
   */
  snap(value: number): void {
    checkFinite(value, "value");
    const before = this.position;
    this.position = value;
    this.goal = value;
    this.speed = 0;
    if (value !== before) this.changed?.(value);
  }

  /**
   * Moves the value by `dt` seconds toward the goal. Settles it when it is close enough and slow
   * enough. Does nothing when `dt` is zero. The work grows with `dt` (about 240 sub-steps a second),
   * so drive a spring with frames rather than with one long step.
   */
  advance(dt: number): void {
    if (this.status !== "running" || dt <= 0) return;
    const before = this.position;
    const omega = Math.sqrt(this.stiffness / this.mass);
    const maxStep = Math.min(MAX_SUBSTEP, STEP_PER_PERIOD / omega);
    const count = Math.ceil(dt / maxStep);
    const step = dt / count;
    for (let index = 0; index < count; index += 1) {
      const force = -this.stiffness * (this.position - this.goal) - this.damping * this.speed;
      this.speed += (force / this.mass) * step;
      this.position += this.speed * step;
    }
    if (Math.abs(this.position - this.goal) <= this.restDelta && Math.abs(this.speed) <= this.restSpeed) {
      this.position = this.goal;
      this.speed = 0;
      if (this.position !== before) this.changed?.(this.position);
      this.status = "complete";
      this.leave();
      this.settle("complete");
      this.completion?.();
      return;
    }
    if (this.position !== before) this.changed?.(this.position);
  }

  /** Ends the spring where it stands. Its promise resolves with "cancelled". */
  cancel(): void {
    if (this.status !== "running") return;
    this.status = "cancelled";
    this.leave();
    this.settle("cancelled");
  }

  /** Ends the spring. Same as {@link cancel}, which is what the ticker calls. */
  kill(): void {
    this.cancel();
  }

  /** Puts the spring on its ticker. */
  private join(): void {
    this.ticker.add(this);
    this.joined = true;
  }

  /** Takes the spring off its ticker. */
  private leave(): void {
    if (!this.joined) return;
    this.ticker.remove(this);
    this.joined = false;
  }

  /** Resolves the promise the first time only. */
  private settle(outcome: Outcome): void {
    const settleNow = this.settleFinished;
    this.settleFinished = undefined;
    settleNow?.(outcome);
  }
}

/**
 * Makes a spring. It joins `options.ticker`, or the shared real-time ticker. Read its `value`
 * each frame, or pass `onUpdate`, and change its goal with `to`.
 * @throws RangeError when an option is not valid (see {@link Spring}).
 */
export function spring(options: SpringOptions = {}): Spring {
  return new Spring(options);
}
