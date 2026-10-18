// The base of tweens and timelines: a playhead in seconds that moves forward or back, a time scale,
// pause, cancel, and a `finished` promise.
//
// A playback that stands alone joins a ticker, which moves its playhead with `advance`. A playback
// that a timeline holds does not join one: the timeline moves it, through `renderAt` and `crossed`,
// so that seeking the timeline puts every child into the right state at once. A cancelled child
// stays where it was and is no longer shown.

import { sharedTicker, type Playable, type Ticker } from "./ticker";

/** How a playback ended: it reached its end, or it was cancelled. */
export type Outcome = "complete" | "cancelled";

/** Options that tweens and timelines share. */
export interface PlaybackOptions {
  /** The ticker that runs the playback. Default: the shared real-time ticker. */
  ticker?: Ticker;
  /** An object that the playback moves, for `Ticker.killTweensOf`. */
  target?: unknown;
  /** A name for `Ticker.killTag`. */
  tag?: string;
  /** Runs when the playback reaches its end. */
  onComplete?(): void;
  /** How fast the playhead moves, relative to the ticker's time. Zero or more. Default 1. */
  timeScale?: number;
}

/** What a timeline needs from a child. Tweens and timelines both provide it. */
export interface TimelineChild {
  /** The child's length in seconds. After it, the child's state no longer changes. Infinity when endless. */
  readonly duration: number;
  /** Takes the child off its ticker, because its timeline moves it from now on. */
  claim(): void;
  /** Shows the child's state at `time` seconds from its own start. Runs no callbacks. */
  renderAt(time: number): void;
  /** Runs the callbacks due between two of the child's own times, as its timeline plays past them. */
  crossed(from: number, to: number): void;
  /** Called when the timeline plays past the child's end: runs its completion callback. */
  endedByParent(): void;
}

/** Where a playback is in its life. */
type Status = "running" | "complete" | "cancelled";

/** Keeps `value` inside [low, high]. */
export function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}

/** Throws unless `value` is a finite time scale of zero or more. */
function checkTimeScale(value: number): number {
  if (!Number.isFinite(value) || value < 0) {
    throw new RangeError(`timeScale must be zero or more, got ${value}`);
  }
  return value;
}

/** The playhead, the time scale, the controls and the promise that tweens and timelines share. */
export abstract class Playback implements Playable, TimelineChild {
  /** The object that the playback moves, if any. */
  readonly target: unknown;
  /** A name for `Ticker.killTag`, if any. */
  readonly tag: string | undefined;
  /** Resolves once, when the playback first reaches its end or is cancelled. */
  readonly finished: Promise<Outcome>;
  private settleFinished: ((outcome: Outcome) => void) | undefined;
  private readonly completion: (() => void) | undefined;
  private readonly ticker: Ticker;
  private status: Status = "running";
  private joined = false;
  private owned = false;
  private playhead = 0;
  private movingBackward = false;
  private scale: number;
  private pausedFlag = false;

  /** @throws RangeError when `timeScale` is negative or not finite. */
  protected constructor(options: PlaybackOptions) {
    this.target = options.target;
    this.tag = options.tag;
    this.ticker = options.ticker ?? sharedTicker();
    this.completion = options.onComplete;
    this.scale = checkTimeScale(options.timeScale ?? 1);
    let resolveOutcome: ((outcome: Outcome) => void) | undefined;
    this.finished = new Promise<Outcome>((resolve) => {
      resolveOutcome = resolve;
    });
    this.settleFinished = resolveOutcome;
  }

  /** The length in seconds: the time after which the playback has ended. Infinity when endless. */
  abstract get duration(): number;

  /** Shows the state at `time` seconds from the start. Subclasses draw their state here. */
  protected abstract render(time: number): void;

  /**
   * Runs the callbacks that fall between two times of this playback. Only a timeline has any; the
   * default does nothing.
   */
  protected onCrossing(_from: number, _to: number): void {}

  /** The playhead: seconds from the start. */
  get time(): number {
    return this.playhead;
  }

  /**
   * The playhead as a fraction of the duration, from 0 to 1. Setting it seeks. An endless playback
   * has no fraction: it reads 0, and setting it throws.
   */
  get progress(): number {
    const length = this.duration;
    if (length === 0) return 1;
    return Number.isFinite(length) ? this.playhead / length : 0;
  }

  set progress(value: number) {
    const length = this.duration;
    if (!Number.isFinite(length)) throw new RangeError("an endless playback has no progress to set");
    this.seek(clamp(value, 0, 1) * length);
  }

  /** True while the playback is paused. */
  get paused(): boolean {
    return this.pausedFlag;
  }

  /** True while the playback is running and not paused. */
  get playing(): boolean {
    return this.status === "running" && !this.pausedFlag;
  }

  /** True when the playback moves backward. */
  get reversed(): boolean {
    return this.movingBackward;
  }

  /** How fast the playhead moves, relative to the ticker's time. */
  get timeScale(): number {
    return this.scale;
  }

  set timeScale(value: number) {
    this.scale = checkTimeScale(value);
  }

  /** True once the playback has ended, by reaching its end or by being cancelled. */
  get isDone(): boolean {
    return this.status !== "running";
  }

  /** Plays forward from the current time. It also resumes a paused playback. */
  play(): void {
    this.movingBackward = false;
    this.resume();
  }

  /** Plays backward from the current time. */
  reverse(): void {
    this.movingBackward = true;
    this.resume();
  }

  /** Stops the playhead where it is. */
  pause(): void {
    this.pausedFlag = true;
  }

  /**
   * Lets the playhead move again. A playback that already completed starts running again from its
   * current time, so `seek(0)` followed by `resume()` plays it through once more. A cancelled one
   * stays cancelled. A standalone playback that is not complete is already on its ticker.
   */
  resume(): void {
    this.pausedFlag = false;
    if (this.status === "complete" && !this.owned) {
      this.status = "running";
      this.join();
    }
  }

  /**
   * Moves the playhead to `time` seconds and shows that state. No callbacks run, so a scrub is
   * silent.
   * @throws RangeError when `time` is not finite.
   */
  seek(time: number): void {
    if (!Number.isFinite(time)) throw new RangeError(`seek time must be a finite number, got ${time}`);
    this.setPlayhead(time);
  }

  /**
   * Ends the playback where it stands. Its promise resolves with "cancelled". The values stay where
   * they are. A playback that already ended is left alone.
   */
  cancel(): void {
    if (this.status !== "running") return;
    this.status = "cancelled";
    this.leave();
    this.settle("cancelled");
  }

  /** Ends the playback. Same as {@link cancel}, which is what the ticker calls. */
  kill(): void {
    this.cancel();
  }

  /**
   * Hands the playback to a timeline, which moves it from now on.
   * @throws RangeError when it already belongs to a timeline, or it was cancelled.
   */
  claim(): void {
    if (this.owned) throw new RangeError("this playback already belongs to a timeline");
    if (this.status === "cancelled") throw new RangeError("a cancelled playback cannot join a timeline");
    this.owned = true;
    this.leave();
  }

  /** Shows the state at `time` seconds from this playback's start, as its timeline asks. */
  renderAt(time: number): void {
    if (this.status === "cancelled") return;
    this.setPlayhead(time);
  }

  /** Runs the callbacks due between two of this playback's times, as its timeline plays past them. */
  crossed(from: number, to: number): void {
    if (this.status === "cancelled") return;
    this.onCrossing(from, to);
  }

  /** Runs the completion callback, as its timeline plays past the end. */
  endedByParent(): void {
    if (this.status === "cancelled") return;
    this.settle("complete");
    this.completion?.();
  }

  /** Moves the playhead by `dt` real seconds, scaled by the time scale and the direction. */
  advance(dt: number): void {
    if (this.status !== "running" || this.pausedFlag) return;
    const from = this.playhead;
    let to = from + dt * this.scale * (this.movingBackward ? -1 : 1);
    let ends = false;
    if (!this.movingBackward && to >= this.duration) {
      to = this.duration;
      ends = true;
    } else if (this.movingBackward && to <= 0) {
      to = 0;
      ends = true;
    }
    this.setPlayhead(to);
    this.onCrossing(from, this.playhead);
    if (ends) this.complete();
  }

  /** Puts this playback on its ticker. Subclasses call it at the end of their constructors. */
  protected join(): void {
    this.ticker.add(this);
    this.joined = true;
  }

  /** Moves the playhead to `time`, kept inside the playback, and draws that state. */
  private setPlayhead(time: number): void {
    this.playhead = clamp(time, 0, this.duration);
    this.render(this.playhead);
  }

  /** Ends a playback that reached its end, and runs its completion callback. */
  private complete(): void {
    this.status = "complete";
    this.leave();
    this.settle("complete");
    this.completion?.();
  }

  /** Takes this playback off its ticker, if it is on one. */
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
