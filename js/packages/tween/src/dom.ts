// DOM helpers: animate an element's transform (translate, scale and rotate) and its opacity, with
// the writes batched once per frame, and with the reduced-motion setting respected.
//
// Only `transform` and `opacity` change, so the browser can animate them without layout. Each
// element keeps its own motion state, one value per part of it: a tween can move x while another
// moves opacity. A write goes into a batch, and the ticker flushes the batch once at the end of each
// frame. Five tweens on one element therefore write its style once, not five times.
//
// When the operating system asks for reduced motion (`prefers-reduced-motion: reduce`), an animation
// jumps to its end state at once. Pass `skipIfReduced: false` to animate anyway.

import type { EasingFunction } from "./easing";
import type { PlaybackOptions } from "./playback";
import { sharedTicker, type Ticker } from "./ticker";
import { tween, type Tween } from "./tween";

/** The motion of an element: a translation in pixels, a scale, a rotation and an opacity. */
export interface DomMotion {
  /** Horizontal offset in pixels. */
  x?: number;
  /** Vertical offset in pixels. */
  y?: number;
  /** Scale. 1 is the element's own size. */
  scale?: number;
  /** Rotation in degrees, clockwise. */
  rotate?: number;
  /** Opacity, from 0 to 1. Values outside that range are clamped when they are written. */
  opacity?: number;
}

/** Options for {@link animateElement}. */
export interface ElementAnimateOptions extends PlaybackOptions {
  /** The length of one cycle, in seconds. */
  duration: number;
  /** Seconds to wait before the first cycle. Default 0. */
  delay?: number;
  /** The easing, as a function or a name. Default "linear". */
  ease?: EasingFunction | string;
  /** Extra cycles after the first. Infinity repeats forever. Default 0. */
  repeat?: number;
  /** When true, every other cycle plays backward. Default false. */
  yoyo?: boolean;
  /**
   * Where the motion starts, for the parts that are given. The parts that are left out start at the
   * element's current motion, which is rest for an element that has not moved.
   */
  from?: DomMotion;
  /** When true (the default), reduced motion jumps to the end state at once. */
  skipIfReduced?: boolean;
}

/** The parts of a motion, in the order they are written. */
const MOTION_KEYS = ["x", "y", "scale", "rotate", "opacity"] as const;

/** One part of a motion. */
type MotionKey = (typeof MOTION_KEYS)[number];

/** The whole motion of an element, with every part set. */
type MotionState = Record<MotionKey, number>;

/** The motion of an element that has not moved. */
const REST: MotionState = { x: 0, y: 0, scale: 1, rotate: 0, opacity: 1 };

/** The motion of each element, kept between animations. */
const states = new WeakMap<HTMLElement, MotionState>();

/** The strings last written to each element, so that an unchanged style is not written again. */
const written = new WeakMap<HTMLElement, { transform: string; opacity: string }>();

/** The write batch of each ticker. */
const batches = new WeakMap<Ticker, WriteBatch>();

/** The motion of an element. A new element starts at rest. */
function stateOf(el: HTMLElement): MotionState {
  let state = states.get(el);
  if (state === undefined) {
    state = { ...REST };
    states.set(el, state);
  }
  return state;
}

/** Keeps `value` inside [low, high]. */
function clampTo(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}

/** Throws unless `value` is a finite number. */
function checkFinite(value: number, name: string): number {
  if (!Number.isFinite(value)) throw new RangeError(`${name} must be a finite number, got ${value}`);
  return value;
}

/** Writes an element's transform and opacity, only where they differ from the last write. */
function writeMotion(el: HTMLElement): void {
  const state = stateOf(el);
  const transform = `translate(${state.x}px, ${state.y}px) rotate(${state.rotate}deg) scale(${state.scale})`;
  const opacity = String(clampTo(state.opacity, 0, 1));
  const last = written.get(el);
  if (last?.transform !== transform) el.style.transform = transform;
  if (last?.opacity !== opacity) el.style.opacity = opacity;
  written.set(el, { transform, opacity });
}

/** The elements whose motion changed in this frame, written together at the frame's end. */
class WriteBatch {
  private readonly dirty = new Set<HTMLElement>();

  /** Registers the flush with the ticker, so that it runs at the end of every frame. */
  constructor(ticker: Ticker) {
    ticker.onFrameEnd(() => {
      this.flush();
    });
  }

  /** Marks an element as changed. It is written at the end of the frame. */
  mark(el: HTMLElement): void {
    this.dirty.add(el);
  }

  /** Writes every changed element now. */
  flush(): void {
    for (const el of this.dirty) writeMotion(el);
    this.dirty.clear();
  }
}

/** The write batch of a ticker, made on first use. */
function batchFor(ticker: Ticker): WriteBatch {
  let batch = batches.get(ticker);
  if (batch === undefined) {
    batch = new WriteBatch(ticker);
    batches.set(ticker, batch);
  }
  return batch;
}

/**
 * True when the operating system asks for less motion (`prefers-reduced-motion: reduce`). False
 * where the page has no `matchMedia`.
 */
export function isReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/**
 * Sets an element's motion at once, with no animation, and writes it now. The parts that are left
 * out keep their values.
 * @throws RangeError when a part is not a finite number.
 */
export function placeElement(el: HTMLElement, motion: DomMotion): void {
  const state = stateOf(el);
  for (const key of MOTION_KEYS) {
    const value = motion[key];
    if (value !== undefined) state[key] = checkFinite(value, key);
  }
  writeMotion(el);
}

/**
 * Animates the parts of an element's motion that `to` gives, from where they are (or from
 * `options.from`) to the values in `to`. The motion is written once per frame of the ticker.
 * Returns the tween, so the animation can be paused, seeked, cancelled or awaited (`finished`).
 *
 * With reduced motion (and `skipIfReduced` left on), the element jumps to `to` at once, and the
 * tween has no time to run.
 * @throws RangeError when `to` gives no part, or a value is not finite, or the timing is invalid.
 */
export function animateElement(
  el: HTMLElement,
  to: DomMotion,
  options: ElementAnimateOptions,
): Tween<Readonly<Record<string, number>>> {
  const keys = MOTION_KEYS.filter((key) => to[key] !== undefined);
  if (keys.length === 0) {
    throw new RangeError("animateElement needs at least one of x, y, scale, rotate or opacity");
  }
  const ticker = options.ticker ?? sharedTicker();
  const batch = batchFor(ticker);
  const current = stateOf(el);
  const start: Record<string, number> = {};
  const end: Record<string, number> = {};
  for (const key of keys) {
    start[key] = checkFinite(options.from?.[key] ?? current[key], `from.${key}`);
    end[key] = checkFinite(to[key] ?? current[key], key);
  }
  const skip = (options.skipIfReduced ?? true) && isReducedMotion();
  const animation = tween({
    from: start,
    to: end,
    duration: skip ? 0 : options.duration,
    delay: skip ? 0 : options.delay,
    ease: options.ease,
    repeat: skip ? 0 : options.repeat,
    yoyo: skip ? false : options.yoyo,
    ticker,
    target: options.target ?? el,
    tag: options.tag,
    timeScale: options.timeScale,
    onComplete: options.onComplete,
    set: (values: Readonly<Record<string, number>>) => {
      const state = stateOf(el);
      for (const key of keys) state[key] = values[key] ?? state[key];
      batch.mark(el);
    },
  });
  // A skipped animation is already at its end state, so it is written now rather than at the frame's end.
  if (skip) batch.flush();
  return animation;
}
