// The ticker: the scheduler that runs tweens, timelines and springs.
//
// A ticker holds the playables that are running and moves them with `advance(dt)`, where `dt` is
// the real step length in seconds. A ticker that has a game clock scales that step the way the
// clock does: nothing moves while the clock is paused, and everything moves at the clock's speed.
// A ticker without a clock uses real time, so it keeps moving while the game is paused, which is
// what UI needs.
//
// The game calls `advance` from its loop, with the same step it gives the clock:
//
//     const motion = new Ticker({ clock });
//     const ui = new Ticker();
//     const loop = new GameLoop({
//       update(dt) {
//         clock.advance(dt);
//         motion.advance(dt);
//         ui.advance(dt);
//       },
//     });
//
// Nothing here reads the wall clock, so the same steps always give the same result, which is what
// tests and replays need. A ticker with no loop can be driven by `createRealtimeDriver`, which calls
// `advance` from animation frames with the real time between them.

import { createFrameScheduler, type FrameScheduler, type GameClock } from "@velvet/core";

/** The longest step that one animation frame may give a ticker, in seconds. */
const DEFAULT_MAX_STEP = 0.1;

/** Something a ticker runs: a tween, a timeline or a spring. */
export interface Playable {
  /** The object that the playable moves, if any. `Ticker.killTweensOf` matches it. */
  readonly target: unknown;
  /** A name for `Ticker.killTag`, if the playable has one. */
  readonly tag: string | undefined;
  /** True once the playable has ended, so the ticker drops it. */
  readonly isDone: boolean;
  /** Moves the playable forward by `dt` seconds of its own time. The ticker has already applied the clock. */
  advance(dt: number): void;
  /** Ends the playable early and takes it off its ticker. */
  kill(): void;
}

/** Options for {@link Ticker}. */
export interface TickerOptions {
  /**
   * A game clock. Its pause and its speed apply to everything this ticker runs. Without one, the
   * ticker runs in real time.
   */
  clock?: GameClock;
  /**
   * Runs when a playable joins a ticker that had none, so that a driver can start its frames. It
   * does not run for a join while the ticker already has playables.
   */
  onActive?(): void;
}

/** Runs playables from one time source. */
export class Ticker {
  private readonly clock: GameClock | undefined;
  private readonly activated: (() => void) | undefined;
  private readonly playables = new Set<Playable>();
  private readonly frameEndCallbacks = new Set<() => void>();
  private elapsedSeconds = 0;

  /** Creates a ticker. With no options it runs in real time and has no driver. */
  constructor(options: TickerOptions = {}) {
    this.clock = options.clock;
    this.activated = options.onActive;
  }

  /** Seconds that this ticker has moved its playables by, after the clock's pause and speed. */
  get time(): number {
    return this.elapsedSeconds;
  }

  /** The number of playables that are running. */
  get size(): number {
    return this.playables.size;
  }

  /** Adds a playable. The factories call it. A playable that is already here is left alone. */
  add(playable: Playable): void {
    if (this.playables.has(playable)) return;
    const wasIdle = this.playables.size === 0;
    this.playables.add(playable);
    if (wasIdle) this.activated?.();
  }

  /** Takes a playable off the ticker without ending it. Does nothing when it is not here. */
  remove(playable: Playable): void {
    this.playables.delete(playable);
  }

  /**
   * Runs `callback` at the end of every `advance`, after the playables have moved. Writes that
   * should happen once per frame (see the DOM helpers) go here. Returns the function that removes
   * the callback.
   */
  onFrameEnd(callback: () => void): () => void {
    this.frameEndCallbacks.add(callback);
    return () => {
      this.frameEndCallbacks.delete(callback);
    };
  }

  /**
   * Moves every running playable forward by `dt` seconds of real time, scaled by the clock when
   * the ticker has one, then runs the frame-end callbacks. The callbacks run even when nothing
   * moved, so a write that was queued while the clock was paused still happens.
   * @throws RangeError when `dt` is negative or not finite.
   */
  advance(dt: number): void {
    if (!Number.isFinite(dt) || dt < 0) {
      throw new RangeError(`dt must be a non-negative finite number, got ${dt}`);
    }
    const step = this.scaledStep(dt);
    if (step > 0) {
      this.elapsedSeconds += step;
      // A copy, because a playable may end, or start another one, while the step runs.
      for (const playable of [...this.playables]) {
        if (!this.playables.has(playable)) continue;
        playable.advance(step);
        if (playable.isDone) this.playables.delete(playable);
      }
    }
    for (const callback of [...this.frameEndCallbacks]) callback();
  }

  /** Ends every playable whose target is `target`. */
  killTweensOf(target: unknown): void {
    for (const playable of [...this.playables]) {
      if (playable.target === target) playable.kill();
    }
  }

  /** Ends every playable that has the tag `tag`. */
  killTag(tag: string): void {
    for (const playable of [...this.playables]) {
      if (playable.tag === tag) playable.kill();
    }
  }

  /** Ends every playable on this ticker. */
  clear(): void {
    for (const playable of [...this.playables]) playable.kill();
  }

  /** The step that the playables see for a real step of `dt`: zero while the clock is paused. */
  private scaledStep(dt: number): number {
    if (this.clock === undefined) return dt;
    return this.clock.paused ? 0 : dt * this.clock.speed;
  }
}

/** Options for {@link createRealtimeDriver}. */
export interface RealtimeDriverOptions {
  /**
   * Where frames come from. Default: the core's frame scheduler, which uses requestAnimationFrame
   * where the host has it.
   */
  scheduler?: FrameScheduler;
  /**
   * The longest step that one frame may give, in seconds. A longer gap, such as a hidden tab, is
   * cut to this length. Default 0.1.
   */
  maxStep?: number;
}

/** Starts and stops the frames that drive a ticker. */
export interface RealtimeDriver {
  /** True while a frame is requested or running. */
  readonly running: boolean;
  /** Starts the frames. Does nothing when they already run. */
  start(): void;
  /** Stops the frames. */
  stop(): void;
}

/**
 * Drives a ticker from animation frames, in real time. Frames run while the ticker has playables.
 * When the last one ends, the driver stops by itself, and the next `start` (the ticker's
 * `onActive`) runs it again.
 * @throws RangeError when `maxStep` is not a positive finite number.
 */
export function createRealtimeDriver(ticker: Ticker, options: RealtimeDriverOptions = {}): RealtimeDriver {
  const maxStep = options.maxStep ?? DEFAULT_MAX_STEP;
  if (!Number.isFinite(maxStep) || maxStep <= 0) {
    throw new RangeError(`maxStep must be a positive number of seconds, got ${maxStep}`);
  }
  const scheduler = options.scheduler ?? createFrameScheduler();
  let handle: number | null = null;
  let inFrame = false;
  let stopRequested = false;
  let lastMs: number | null = null;

  const frame = (timeMs: number): void => {
    handle = null;
    const dt = lastMs === null ? 0 : Math.min(maxStep, Math.max(0, (timeMs - lastMs) / 1000));
    lastMs = timeMs;
    inFrame = true;
    try {
      ticker.advance(dt);
    } finally {
      inFrame = false;
    }
    if (ticker.size > 0 && !stopRequested) {
      handle = scheduler.request(frame);
    } else {
      lastMs = null;
    }
  };

  return {
    get running(): boolean {
      return handle !== null || inFrame;
    },
    start(): void {
      // A start inside a frame is picked up when the frame ends, so it must not request a second one.
      if (handle !== null || inFrame) return;
      stopRequested = false;
      lastMs = null;
      handle = scheduler.request(frame);
    },
    stop(): void {
      if (inFrame) stopRequested = true;
      if (handle !== null) scheduler.cancel(handle);
      handle = null;
      lastMs = null;
    },
  };
}

/** The shared real-time ticker, made on first use. */
let shared: Ticker | undefined;

/**
 * The ticker that runs playables when none is given. It runs in real time, driven by animation
 * frames while it has playables, so a tween works without any setup. Tests and games with a loop
 * should pass their own ticker instead.
 */
export function sharedTicker(): Ticker {
  if (shared === undefined) {
    let driver: RealtimeDriver | undefined;
    const ticker = new Ticker({ onActive: () => driver?.start() });
    driver = createRealtimeDriver(ticker);
    shared = ticker;
  }
  return shared;
}
