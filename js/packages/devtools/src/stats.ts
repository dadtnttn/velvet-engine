// Frame statistics: the recent frame intervals, their percentiles, the long-frame counters and the
// update and render split of the loop. The sampler does a few array operations per frame.
//
// Percentiles use the nearest-rank method: the p-th percentile of n sorted values is the value at
// rank ceil(p / 100 * n). It is always a value that was measured, which suits frame times.

/** A frame longer than this counts as long at 60 Hz. */
export const LONG_FRAME_60_MS = 16.7;

/** A frame longer than this counts as long at 30 Hz. */
export const LONG_FRAME_30_MS = 33;

/** How many recent frames the percentiles and the graph use: four seconds at 60 Hz. */
export const FRAME_WINDOW = 240;

/** How many recent frames the frame rate is averaged over. */
const FPS_WINDOW = 60;

/** A gap longer than this is a stall, such as a hidden tab, so it is not counted as a frame. */
const MAX_FRAME_MS = 1000;

/** A summary of a list of numbers. */
export interface Summary {
  readonly count: number;
  readonly mean: number;
  readonly p50: number;
  readonly p95: number;
  readonly max: number;
}

/** Frame numbers at one moment. A null means there is not enough data yet. */
export interface FrameStats {
  /** Frames measured since the start or the last reset. */
  readonly frames: number;
  /** Frames per second, from the mean interval of the last 60 frames. */
  readonly fps: number | null;
  /** The newest frame interval, in milliseconds. */
  readonly lastMs: number | null;
  /** The median frame interval over the last 240 frames, in milliseconds. */
  readonly p50Ms: number | null;
  /** The 95th percentile over the same frames, in milliseconds. */
  readonly p95Ms: number | null;
  /** The longest of the same frames, in milliseconds. */
  readonly maxMs: number | null;
  /** Frames longer than 16.7 ms since the start or the last reset. */
  readonly longFrames16: number;
  /** Frames longer than 33 ms since the start or the last reset. */
  readonly longFrames33: number;
  /** Mean time in game updates per loop frame over the window. Null unless the loop is wrapped. */
  readonly updateMs: number | null;
  /** Mean time in the render callback per loop frame over the window. Null unless the loop is wrapped. */
  readonly renderMs: number | null;
}

/** A fixed-size ring of numbers. The newest value replaces the oldest once the ring is full. */
export class Ring {
  private readonly capacity: number;
  private readonly items: number[] = [];
  private nextIndex = 0;

  /** @param capacity - how many values the ring keeps. */
  constructor(capacity: number) {
    this.capacity = capacity;
  }

  /** Adds a value, replacing the oldest one when the ring is full. */
  push(value: number): void {
    if (this.items.length < this.capacity) {
      this.items.push(value);
    } else {
      this.items[this.nextIndex] = value;
    }
    this.nextIndex = (this.nextIndex + 1) % this.capacity;
  }

  /** The values from the oldest to the newest, in a new array. */
  toArray(): number[] {
    if (this.items.length < this.capacity) return [...this.items];
    return [...this.items.slice(this.nextIndex), ...this.items.slice(0, this.nextIndex)];
  }

  /** Empties the ring. */
  clear(): void {
    this.items.length = 0;
    this.nextIndex = 0;
  }
}

/**
 * The nearest-rank percentile of values that are sorted in ascending order. `p` is in percent, from 0 to 100.
 * @returns NaN when there are no values.
 */
export function percentile(sorted: readonly number[], p: number): number {
  if (sorted.length === 0) return Number.NaN;
  const rank = Math.ceil((p / 100) * sorted.length);
  const index = Math.min(sorted.length, Math.max(1, rank)) - 1;
  return sorted[index] ?? Number.NaN;
}

/** The count, mean, median, 95th percentile and maximum of a list, or null for an empty list. */
export function summarize(values: readonly number[]): Summary | null {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const total = sorted.reduce((sum, value) => sum + value, 0);
  return {
    count: sorted.length,
    mean: total / sorted.length,
    p50: percentile(sorted, 50),
    p95: percentile(sorted, 95),
    max: sorted[sorted.length - 1] ?? Number.NaN,
  };
}

/** The mean of a list, or null for an empty list. */
function mean(values: readonly number[]): number | null {
  if (values.length === 0) return null;
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

/**
 * Keeps the frame history and the counters. The devtools feed it one timestamp per frame, and the
 * update and render times of each loop frame when the loop is wrapped.
 */
export class FrameSampler {
  private readonly intervals = new Ring(FRAME_WINDOW);
  private readonly updates = new Ring(FRAME_WINDOW);
  private readonly renders = new Ring(FRAME_WINDOW);
  private lastFrameMs: number | null = null;
  private frameCount = 0;
  private longCount16 = 0;
  private longCount33 = 0;

  /** Records a frame that was drawn at `timeMs`, in milliseconds. The first frame only sets the origin. */
  onFrame(timeMs: number): void {
    const previous = this.lastFrameMs;
    this.lastFrameMs = timeMs;
    if (previous === null) return;
    const interval = timeMs - previous;
    if (!(interval > 0) || interval > MAX_FRAME_MS) return;
    this.intervals.push(interval);
    this.frameCount += 1;
    if (interval > LONG_FRAME_60_MS) this.longCount16 += 1;
    if (interval > LONG_FRAME_30_MS) this.longCount33 += 1;
  }

  /** Records the time spent in the game's updates, and in the render callback, for one loop frame. */
  onLoopFrame(updateMs: number, renderMs: number): void {
    this.updates.push(updateMs);
    this.renders.push(renderMs);
  }

  /** The recent frame intervals, oldest first, for the graph. */
  recentIntervals(): number[] {
    return this.intervals.toArray();
  }

  /** The numbers as they stand now. */
  stats(): FrameStats {
    const intervals = this.intervals.toArray();
    const summary = summarize(intervals);
    const meanInterval = mean(intervals.slice(-FPS_WINDOW));
    return {
      frames: this.frameCount,
      fps: meanInterval === null || meanInterval <= 0 ? null : 1000 / meanInterval,
      lastMs: intervals[intervals.length - 1] ?? null,
      p50Ms: summary?.p50 ?? null,
      p95Ms: summary?.p95 ?? null,
      maxMs: summary?.max ?? null,
      longFrames16: this.longCount16,
      longFrames33: this.longCount33,
      updateMs: mean(this.updates.toArray()),
      renderMs: mean(this.renders.toArray()),
    };
  }

  /** Clears the history and the counters. The next frame sets a new origin. */
  reset(): void {
    this.intervals.clear();
    this.updates.clear();
    this.renders.clear();
    this.lastFrameMs = null;
    this.frameCount = 0;
    this.longCount16 = 0;
    this.longCount33 = 0;
  }
}
