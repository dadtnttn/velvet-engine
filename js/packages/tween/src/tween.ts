// Tweens: animate a number, an array, an object of numbers or a colour from `from` to `to`.
//
// A tween's value is a pure function of its time: the time since it started, less its delay, is
// split into cycles of `duration` seconds, and the eased progress inside the current cycle gives the
// value. So the tween is exact at any time, and seeking or replaying it gives the same values. Each
// time the value is drawn, the tween hands it to `set` and then to `onUpdate`. Drawing the same time
// twice draws nothing, so a paused tween does not call its setter again.

import { ease, type EasingFunction } from "./easing";
import { interpolate, type TweenValue } from "./interpolate";
import { Playback, clamp, type PlaybackOptions } from "./playback";

/** Options for {@link tween}. */
export interface TweenOptions<T extends TweenValue> extends PlaybackOptions {
  /** The value at the start. Its kind (number, array, object or colour text) sets the kind of the tween. */
  from: T;
  /** The value at the end of a cycle. It must be the same kind as `from`. */
  to: T;
  /** The length of one cycle, in seconds. Zero or more. */
  duration: number;
  /** Seconds to wait before the first cycle. Default 0. During the wait the value stays at `from`. */
  delay?: number;
  /** The easing: a function, or the name of one from {@link ease}. Default "linear". */
  ease?: EasingFunction | string;
  /** Extra cycles after the first. Infinity repeats forever. Default 0. */
  repeat?: number;
  /** When true, every other cycle plays backward, so that the value comes back to `from`. Default false. */
  yoyo?: boolean;
  /** Writes each value, for example to a style or to a game object. */
  set?(value: T): void;
  /** Runs after `set`, with the same value. */
  onUpdate?(value: T): void;
}

/** Throws unless `value` is a finite number of seconds from zero. */
function checkSeconds(value: number, name: string): number {
  if (!Number.isFinite(value) || value < 0) {
    throw new RangeError(`${name} must be a finite number of seconds from 0, got ${value}`);
  }
  return value;
}

/** Throws unless `value` is a whole number from 0, or Infinity. */
function checkRepeat(value: number): number {
  if (value !== Infinity && (!Number.isInteger(value) || value < 0)) {
    throw new RangeError(`repeat must be a whole number from 0, or Infinity, got ${value}`);
  }
  return value;
}

/** A tween of one value. Create it with {@link tween}. */
export class Tween<T extends TweenValue> extends Playback {
  private readonly options: TweenOptions<T>;
  private readonly mixer: (t: number) => TweenValue;
  private readonly easing: EasingFunction;
  private readonly delaySeconds: number;
  private readonly cycleSeconds: number;
  private readonly cycleCount: number;
  private readonly yoyoOn: boolean;
  private latest: T;
  private renderedTime: number | null = null;

  /**
   * Starts a tween. It joins its ticker at once, and its value is drawn at once too, so a delayed
   * tween shows `from` from the start.
   * @throws RangeError for a bad duration, delay, repeat or easing name, or for values that cannot be
   * interpolated (see {@link interpolate}).
   */
  constructor(options: TweenOptions<T>) {
    super(options);
    this.options = options;
    this.delaySeconds = checkSeconds(options.delay ?? 0, "delay");
    this.cycleSeconds = checkSeconds(options.duration, "duration");
    this.cycleCount = checkRepeat(options.repeat ?? 0) + 1;
    this.yoyoOn = options.yoyo ?? false;
    this.easing = typeof options.ease === "function" ? options.ease : ease(options.ease ?? "linear");
    this.mixer = interpolate(options.from, options.to);
    this.latest = options.from;
    this.render(0);
    this.join();
  }

  /** The length in seconds: the delay, plus every cycle. Infinity when the tween repeats forever. */
  override get duration(): number {
    return this.delaySeconds + this.cycleTotal();
  }

  /** The value at the current time. */
  get value(): T {
    return this.latest;
  }

  /** Draws the value at `time` seconds from the start, calling `set` and `onUpdate` when it changed. */
  protected override render(time: number): void {
    if (this.renderedTime === time) return;
    this.renderedTime = time;
    const value = this.mixer(this.easing(this.progressAt(time))) as T;
    this.latest = value;
    this.options.set?.(value);
    this.options.onUpdate?.(value);
  }

  /** The total length of the cycles, not counting the delay. */
  private cycleTotal(): number {
    return this.cycleSeconds === 0 ? 0 : this.cycleCount * this.cycleSeconds;
  }

  /** The progress from 0 to 1 at `time`, before the easing is applied. */
  private progressAt(time: number): number {
    const local = time - this.delaySeconds;
    if (local < 0) return 0;
    if (local >= this.cycleTotal()) {
      // The last cycle ends the tween. A yoyo whose last cycle runs backward ends back at the start.
      const lastCycleBackward = this.yoyoOn && (this.cycleCount - 1) % 2 === 1;
      return lastCycleBackward ? 0 : 1;
    }
    const index = Math.floor(local / this.cycleSeconds);
    const fraction = clamp((local - index * this.cycleSeconds) / this.cycleSeconds, 0, 1);
    return this.yoyoOn && index % 2 === 1 ? 1 - fraction : fraction;
  }
}

/** Makes a tween of a number. The `set` callback receives each value from `from` to `to`. */
export function tween(options: TweenOptions<number>): Tween<number>;
/** Makes a tween of a colour text. Colours are mixed in OKLab, and the value is written as `rgb()` or `rgba()`. */
export function tween(options: TweenOptions<string>): Tween<string>;
/** Makes a tween of an array of numbers. */
export function tween(options: TweenOptions<readonly number[]>): Tween<readonly number[]>;
/** Makes a tween of an object of numbers, such as `{ x: 0, y: 0 }`. */
export function tween(
  options: TweenOptions<Readonly<Record<string, number>>>,
): Tween<Readonly<Record<string, number>>>;
/**
 * Makes a tween. It joins `options.ticker`, or the shared real-time ticker when there is none.
 * @throws RangeError or TypeError when the options are invalid (see the {@link Tween} constructor).
 */
export function tween(options: TweenOptions<TweenValue>): Tween<TweenValue> {
  return new Tween(options);
}
