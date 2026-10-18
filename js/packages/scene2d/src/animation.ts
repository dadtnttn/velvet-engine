// Animations: a list of frames shown one after another, timed in seconds of game time. An animation joins
// no ticker. Its owner calls `tick(dt)` from the game loop with the same step as the clock, and given a
// clock it scales the step the way the clock does, so nothing moves while the game is paused.
//
// The frame at a given time is a pure function of that time. An animation seeked to a time shows the same
// frame as one that reached it step by step. The playing directions are Aseprite's, so an Aseprite tag can
// be played as it is.

import type { GameClock } from "@velvet/core";
import { clamp, isNonNegativeFinite, isPositiveFinite } from "./math";
import type { Sprite, SpriteFrame } from "./sprite";
import { clockStep } from "./time";

/** The order in which an animation plays its frames. The names are Aseprite's. */
export type PlayDirection = "forward" | "reverse" | "pingpong" | "pingpong_reverse";

/** The frame rate when the options give neither a rate nor durations. */
const DEFAULT_FPS = 12;

/** Slack, in seconds, on the time of a frame change, so that a float error never shows a frame late. */
const TIME_SLACK = 1e-9;

/** Options for {@link createAnimation}. */
export interface AnimationOptions {
  /** Frames per second, used when `durations` is not given. Default 12. */
  fps?: number;
  /** The time that each frame is shown, in seconds, one entry per frame. It overrides `fps`. */
  durations?: readonly number[];
  /** Whether the animation repeats. Default true. */
  loop?: boolean;
  /** The order of the frames. Default "forward". */
  direction?: PlayDirection;
  /** Whether the animation plays from the start. Default true. */
  autoplay?: boolean;
  /** A game clock. Its pause and speed apply to the animation. */
  clock?: GameClock;
  /** A sprite that shows each frame: its `frame` is set whenever the frame changes. */
  sprite?: Sprite;
  /** Runs when the frame changes, including for the first frame. */
  onFrame?: (frame: SpriteFrame, index: number) => void;
  /** Runs once, when a loop-free animation reaches its end. */
  onComplete?: () => void;
}

/** An animation: frames shown in turn, timed by the game's own time. Create it with {@link createAnimation}. */
export class Animation {
  private readonly frames: readonly SpriteFrame[];
  /** The frame index of each step of one cycle, in playing order. */
  private readonly order: readonly number[];
  /** The start time of each step in the cycle, in seconds. */
  private readonly starts: readonly number[];
  private readonly lengthSeconds: number;
  private readonly looping: boolean;
  private readonly clock: GameClock | undefined;
  private readonly sprite: Sprite | undefined;
  private readonly onFrame: ((frame: SpriteFrame, index: number) => void) | undefined;
  private readonly onComplete: (() => void) | undefined;
  private current: SpriteFrame;
  private shown = -1;
  private clockTime = 0;
  private stepIndex = 0;
  private running: boolean;
  private ended = false;

  /**
   * @param frames - the frames, in their own order. At least one.
   * @param options - see {@link AnimationOptions}.
   * @throws RangeError when there are no frames, the durations do not match them, or a rate or duration is not valid.
   */
  constructor(frames: readonly SpriteFrame[], options: AnimationOptions = {}) {
    const first = frames[0];
    if (first === undefined) throw new RangeError("an animation needs at least one frame");
    this.frames = [...frames];
    this.current = first;
    this.looping = options.loop ?? true;
    this.clock = options.clock;
    this.sprite = options.sprite;
    this.onFrame = options.onFrame;
    this.onComplete = options.onComplete;
    this.running = options.autoplay ?? true;

    const seconds = frameSeconds(frames.length, options);
    this.order = playbackOrder(frames.length, options.direction ?? "forward");
    const starts: number[] = [];
    let total = 0;
    for (const index of this.order) {
      starts.push(total);
      total += seconds[index] ?? 0;
    }
    this.starts = starts;
    this.lengthSeconds = total;
    this.apply();
  }

  /** The frame that is shown now. */
  get frame(): SpriteFrame {
    return this.current;
  }

  /** The index in the frame list of the frame that is shown now. */
  get index(): number {
    return this.shown;
  }

  /** The time in seconds into the current cycle. */
  get time(): number {
    return this.clockTime;
  }

  /** The length of one cycle in seconds. */
  get duration(): number {
    return this.lengthSeconds;
  }

  /** True while the animation is running. */
  get playing(): boolean {
    return this.running;
  }

  /** True once a loop-free animation has reached its end. */
  get finished(): boolean {
    return this.ended;
  }

  /** Starts the animation. One that has finished starts again from the beginning. */
  play(): void {
    if (this.ended) {
      this.clockTime = 0;
      this.ended = false;
      this.stepIndex = 0;
      this.apply();
    }
    this.running = true;
  }

  /** Stops the animation where it is. */
  pause(): void {
    this.running = false;
  }

  /** Goes back to the first frame. It does not change whether the animation is playing. */
  reset(): void {
    this.seek(0);
  }

  /**
   * Moves to a time in the cycle and shows that frame. A loop wraps the time into the cycle, and a loop-free
   * animation stops at its end. Seeking runs no callbacks.
   * @throws RangeError when `seconds` is not finite.
   */
  seek(seconds: number): void {
    if (!Number.isFinite(seconds)) throw new RangeError(`seek time must be a finite number, got ${seconds}`);
    let time = 0;
    if (this.lengthSeconds > 0) {
      time = this.looping
        ? ((seconds % this.lengthSeconds) + this.lengthSeconds) % this.lengthSeconds
        : clamp(seconds, 0, this.lengthSeconds);
    }
    this.clockTime = time;
    this.ended = false;
    this.stepIndex = this.locate(time);
    this.apply();
  }

  /**
   * Moves the animation on by a real step of `dt` seconds, scaled by the clock. It shows the frame of the new
   * time, and it runs `onComplete` once when a loop-free animation reaches its end.
   * @throws RangeError when `dt` is negative or not finite.
   */
  tick(dt: number): void {
    if (!this.running) return;
    const step = clockStep(dt, this.clock);
    if (step <= 0) return;
    let time = this.clockTime + step;
    let completed = false;
    if (this.lengthSeconds <= 0) {
      if (!this.looping) completed = true;
      time = 0;
    } else if (this.looping) {
      time %= this.lengthSeconds;
    } else if (time >= this.lengthSeconds) {
      time = this.lengthSeconds;
      completed = true;
    }
    this.clockTime = time;
    if (completed) {
      this.running = false;
      this.ended = true;
    }
    this.stepIndex = this.locate(time);
    this.apply();
    if (completed) this.onComplete?.();
  }

  /** The step whose start is the last one at or before a time. Equal starts give the later step. */
  private locate(time: number): number {
    let low = 0;
    let high = this.starts.length - 1;
    while (low < high) {
      const middle = (low + high + 1) >> 1;
      if ((this.starts[middle] ?? 0) <= time + TIME_SLACK) low = middle;
      else high = middle - 1;
    }
    return low;
  }

  /** Shows the frame of the current step, if it is not already shown. */
  private apply(): void {
    const index = this.order[this.stepIndex] ?? 0;
    if (index === this.shown) return;
    const frame = this.frames[index];
    if (frame === undefined) return;
    this.shown = index;
    this.current = frame;
    if (this.sprite !== undefined) this.sprite.frame = frame;
    this.onFrame?.(frame, index);
  }
}

/**
 * Makes an animation of frames. It shows the first frame at once, and with `autoplay` it runs from there.
 * Its `tick(dt)` moves it, from the game loop.
 * @throws RangeError when the options are not valid (see {@link Animation}).
 */
export function createAnimation(frames: readonly SpriteFrame[], options: AnimationOptions = {}): Animation {
  return new Animation(frames, options);
}

/** The time that each frame is shown, in seconds: from the durations, or from the frame rate. */
function frameSeconds(count: number, options: AnimationOptions): number[] {
  if (options.durations !== undefined) {
    if (options.durations.length !== count) {
      throw new RangeError(`durations has ${options.durations.length} entries for ${count} frames`);
    }
    return options.durations.map((seconds, index) => {
      if (!isNonNegativeFinite(seconds)) {
        throw new RangeError(`durations[${index}] must be a finite number from 0, got ${seconds}`);
      }
      return seconds;
    });
  }
  const fps = options.fps ?? DEFAULT_FPS;
  if (!isPositiveFinite(fps)) throw new RangeError(`fps must be a positive number, got ${fps}`);
  return Array.from({ length: count }, () => 1 / fps);
}

/**
 * The frame indices of one cycle, in playing order. A ping-pong runs to the end and back, without repeating
 * the end frames, so the cycle has 2 × (count − 1) steps.
 */
function playbackOrder(count: number, direction: PlayDirection): number[] {
  const forward: number[] = [];
  for (let index = 0; index < count; index += 1) forward.push(index);
  const backward = [...forward].reverse();
  if (direction === "forward") return forward;
  if (direction === "reverse") return backward;
  if (count <= 1) return [0];
  // The inner frames, in the order they are visited on the way back.
  const innerBackward = forward.slice(1, count - 1).reverse();
  const innerForward = forward.slice(1, count - 1);
  if (direction === "pingpong") return [...forward, ...innerBackward];
  return [...backward, ...innerForward];
}
